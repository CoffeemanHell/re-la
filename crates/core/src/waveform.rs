//! A song's waveform from the relay (re-with works it out once on the server): peaks and RMS, 0..1. Kept on disk as it
//! came.
use crate::disk;
use crate::subsonic::Login;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// About 8000 songs.
const CACHE_BYTES: u64 = 64 << 20;

#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub peaks: Vec<f32>,
    pub rms: Vec<f32>, // each <= its peak
}

/// The relay's body: `bins` u16 LE, `bins` peak bytes, `bins` RMS bytes.
pub fn parse(b: &[u8]) -> Option<Waveform> {
    let bins = u16::from_le_bytes(b.get(..2)?.try_into().ok()?) as usize;
    if bins == 0 || b.len() != 2 + 2 * bins {
        return None;
    }
    let f = |x: &[u8]| x.iter().map(|&v| f32::from(v) / 255.0).collect();
    Some(Waveform { peaks: f(&b[2..2 + bins]), rms: f(&b[2 + bins..]) })
}

fn file(dir: &Path, server: &str, id: &str) -> PathBuf {
    dir.join(disk::hashed(&format!("{server}\n{id}")))
}

/// From the disk cache. shortcut: an entry is never asked for again; a song file replaced under the same id keeps its
/// old waveform until the cache drops it.
pub fn cached(dir: &Path, server: &str, id: &str) -> Option<Waveform> {
    let f = file(dir, server, id);
    let w = parse(&std::fs::read(&f).ok()?)?;
    disk::touch(&f);
    Some(w)
}

/// From the relay, into the disk cache.
pub async fn fetch(http: &reqwest::Client, url: url::Url, login: &Login, dir: &Path, id: &str) -> Option<Waveform> {
    let body = crate::relay::get(http, url, login).await?;
    let w = parse(&body)?;
    let _ = std::fs::create_dir_all(dir);
    if disk::write_atomic(&file(dir, &login.server, id), &body).is_ok() {
        disk::trim(dir, CACHE_BYTES, &HashSet::new());
    }
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cases() {
        let mut b = 2u16.to_le_bytes().to_vec();
        b.extend([255, 0, 51, 0]);
        assert_eq!(parse(&b), Some(Waveform { peaks: vec![1.0, 0.0], rms: vec![0.2, 0.0] }));
        assert_eq!(parse(&b[..5]), None);
        assert_eq!(parse(&[0, 0]), None);
        assert_eq!(parse(&[]), None);
    }
}
