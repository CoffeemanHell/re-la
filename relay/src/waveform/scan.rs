//! Follows Navidrome's library scans: after each one (and at start) every song without a current waveform is
//! made, one ffmpeg at a time; songs gone from the library lose theirs.
use super::navidrome::{Account, Fetch, Navidrome};
use super::{Store, decode, safe_id};
use crate::status::{Phase, Progress};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The longest one song's decode may take.
const DECODE_LIMIT: Duration = Duration::from_secs(600);

#[derive(Debug)]
pub struct WaveformConfig {
    pub dir: PathBuf,
    pub account: Account,
    pub ffmpeg: String,
    /// How often Navidrome's scan status is asked.
    pub poll: Duration,
}

/// Opens the folder and starts the scanner; the store is shared with the endpoint, `progress` with
/// `re-with status`. Must run inside tokio.
pub fn start(
    cfg: WaveformConfig,
    http: reqwest::Client,
    navidrome: String,
    progress: Arc<Mutex<Progress>>,
) -> std::io::Result<Arc<Mutex<Store>>> {
    let store = Arc::new(Mutex::new(Store::open(cfg.dir)?));
    let nd = Navidrome::new(http, navidrome, cfg.account);
    let (s, ffmpeg, poll) = (store.clone(), cfg.ffmpeg, cfg.poll);
    progress.lock().unwrap().phase = Phase::Waiting;
    tokio::spawn(async move {
        let mut seen: Option<Option<String>> = None; // lastScan of the last whole pass; None: none yet
        let mut rest = Phase::Waiting; // what the screen says between passes
        loop {
            let status = nd.scan_status().await;
            {
                let mut p = progress.lock().unwrap();
                p.scanning = status.as_ref().map(|s| s.0);
                match &status {
                    Some((_, last)) => {
                        p.last_scan.clone_from(last);
                        if p.phase == Phase::Unreachable {
                            // a pass cut short is not done: only the last whole pass's phase may come back
                            p.phase = if seen.as_ref() == Some(last) { rest } else { Phase::Waiting };
                        }
                    }
                    None => p.phase = Phase::Unreachable,
                }
            }
            if let Some((false, last)) = status
                && seen.as_ref() != Some(&last)
                && pass(&nd, &s, &ffmpeg, &last, &progress).await
            {
                seen = Some(last);
                rest = progress.lock().unwrap().phase;
            }
            tokio::time::sleep(poll).await;
        }
    });
    Ok(store)
}

/// One pass over the library after the scan that ended at `last`. false: not done (no list, a scan started
/// meanwhile, Navidrome went away), try again at the next poll.
async fn pass(nd: &Navidrome, store: &Mutex<Store>, ffmpeg: &str, last: &Option<String>, progress: &Mutex<Progress>) -> bool {
    let set = |phase| progress.lock().unwrap().phase = phase;
    {
        // a new pass: the last one's numbers and errors would read as this one's
        let mut p = progress.lock().unwrap();
        let (scanning, last_scan) = (p.scanning, p.last_scan.take());
        *p = Progress { phase: Phase::Listing, scanning, last_scan, ..Progress::default() };
    }
    let Some(songs) = nd.songs().await else {
        set(Phase::Unreachable);
        return false;
    };
    // a scan that started while the pages were read may have shifted them: such a list must delete nothing
    if nd.scan_status().await.as_ref().is_none_or(|(scanning, l)| *scanning || l != last) {
        set(Phase::Waiting);
        return false;
    }
    let todo: Vec<_> = {
        let mut st = store.lock().unwrap();
        let live: HashSet<&str> = songs.iter().map(|s| s.id.as_str()).collect();
        for id in st.ids() {
            if !live.contains(id.as_str()) {
                st.remove(&id);
            }
        }
        songs.iter().filter(|s| safe_id(&s.id) && st.key(&s.id) != Some(&s.key)).cloned().collect()
    };
    if !todo.is_empty() {
        eprintln!("waveforms: {} songs to make", todo.len());
    }
    {
        let mut p = progress.lock().unwrap();
        (p.phase, p.total, p.done, p.failed) = (Phase::Working, todo.len(), 0, 0);
        p.pass_start = Some(std::time::Instant::now());
    }
    let failed = |e: String| {
        let mut p = progress.lock().unwrap();
        p.failed += 1;
        p.last_error = Some(e);
    };
    let work = store.lock().unwrap().work_dir();
    for song in todo {
        progress.lock().unwrap().current = Some((song.title.clone(), std::time::Instant::now()));
        let file = work.join(&song.id);
        let fetched = nd.download(&song.id, &file).await;
        let made = match fetched {
            Ok(()) => decode::levels(ffmpeg, &file, DECODE_LIMIT).await,
            Err(Fetch::Unreachable(e)) => {
                let _ = std::fs::remove_file(&file);
                eprintln!("waveforms: Navidrome unreachable ({e}); the pass goes on at the next poll");
                let mut p = progress.lock().unwrap();
                (p.phase, p.current) = (Phase::Unreachable, None);
                return false; // not done: the songs left wait for Navidrome, not for its next scan
            }
            Err(Fetch::Disabled) => {
                let _ = std::fs::remove_file(&file);
                eprintln!("waveforms: Navidrome has downloads disabled (EnableDownloads); nothing can be made");
                let mut p = progress.lock().unwrap();
                (p.phase, p.current) = (Phase::DownloadsOff, None);
                return true; // tried; the next scan tries again
            }
            Err(e) => Err(e.to_string()),
        };
        let _ = std::fs::remove_file(&file);
        let saved = made.and_then(|l| store.lock().unwrap().save(&song.id, &song.key, &l).map_err(|e| e.to_string()));
        match saved {
            Ok(()) => progress.lock().unwrap().done += 1,
            Err(e) => {
                eprintln!("waveform {}: {e}", song.id);
                failed(format!("{}: {e}", song.title));
            }
        }
    }
    let mut p = progress.lock().unwrap();
    (p.phase, p.current) = (Phase::Current, None);
    true
}
