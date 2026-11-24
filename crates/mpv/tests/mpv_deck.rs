//! The mpv deck on real libmpv, audio to the null output: a generated WAV plays to the end, gapless append moves on.
use rela_core::player::{DeckEvent, DeckSink, EndReason};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A mono 16-bit 44.1 kHz sine WAV of the given length.
fn wav(name: &str, seconds: f64) -> PathBuf {
    let rate = 44_100u32;
    let n = (seconds * rate as f64) as u32;
    let mut b = Vec::with_capacity(44 + n as usize * 2);
    b.extend(b"RIFF");
    b.extend((36 + n * 2).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes()); // PCM
    b.extend(1u16.to_le_bytes()); // mono
    b.extend(rate.to_le_bytes());
    b.extend((rate * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((n * 2).to_le_bytes());
    for i in 0..n {
        let v = ((i as f64 * 440.0 * std::f64::consts::TAU / rate as f64).sin() * 8000.0) as i16;
        b.extend(v.to_le_bytes());
    }
    let p = std::env::temp_dir().join(format!("rela-mpv-{}-{name}.wav", std::process::id()));
    std::fs::write(&p, b).unwrap();
    p
}

fn deck() -> (Box<dyn rela_core::player::Deck>, mpsc::Receiver<DeckEvent>) {
    let (tx, rx) = mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    let sink: DeckSink = std::sync::Arc::new(move |k, e| {
        assert_eq!(k, 3);
        let _ = tx.lock().unwrap().send(e);
    });
    (rela_mpv::mpv_deck(3, sink, "null"), rx)
}

/// Events until pred matches one (that one included), or panics after 10 s.
fn until(rx: &mpsc::Receiver<DeckEvent>, pred: impl Fn(&DeckEvent) -> bool) -> Vec<DeckEvent> {
    let end = Instant::now() + Duration::from_secs(10);
    let mut seen = vec![];
    while let Ok(e) = rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
        let hit = pred(&e);
        seen.push(e);
        if hit {
            return seen;
        }
    }
    panic!("timed out; saw {seen:?}");
}

#[test]
fn plays_a_wav_to_the_end() {
    let f = wav("one", 1.5);
    let (mut d, rx) = deck();
    d.load(f.to_str().unwrap(), false);
    let seen = until(&rx, |e| matches!(e, DeckEvent::FileLoaded));
    assert!(seen.contains(&DeckEvent::StartFile), "{seen:?}");
    let seen = until(&rx, |e| matches!(e, DeckEvent::EndFile { .. }));
    assert!(seen.iter().any(|e| matches!(e, DeckEvent::Duration(d) if (d - 1.5).abs() < 0.01)), "{seen:?}");
    assert!(seen.iter().any(|e| matches!(e, DeckEvent::Position(Some(p)) if *p > 0.5)), "{seen:?}");
    assert!(seen.iter().any(|e| matches!(e, DeckEvent::Prop(k, Some(v)) if k == "audio-params/samplerate" && v == "44100")), "{seen:?}");
    assert_eq!(seen.last().unwrap(), &DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
}

#[test]
fn append_starts_next_without_end_file() {
    let (a, b) = (wav("a", 1.0), wav("b", 1.0));
    let (mut d, rx) = deck();
    d.load(a.to_str().unwrap(), false);
    until(&rx, |e| matches!(e, DeckEvent::FileLoaded));
    d.append(b.to_str().unwrap());
    // mpv ends the first file (eof) and starts the appended one in the same deck: the core ignores that end while it
    // has appended
    let seen = until(&rx, |e| matches!(e, DeckEvent::StartFile));
    assert!(!seen.iter().any(|e| matches!(e, DeckEvent::EndFile { reason: EndReason::Error, .. })), "{seen:?}");
    // an idle gap is mpv with nothing loaded between the end and the next start; newer mpv (the Windows build) does
    // clear the first file's properties before its end-file, which is no gap
    let mut after_end = seen.iter().skip_while(|e| !matches!(e, DeckEvent::EndFile { .. }));
    assert!(!after_end.any(|e| matches!(e, DeckEvent::Position(None))), "no idle gap: {seen:?}");
    d.drop_first();
    let seen = until(&rx, |e| matches!(e, DeckEvent::EndFile { .. }));
    assert_eq!(seen.last().unwrap(), &DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
}

#[test]
fn a_missing_file_is_an_error_with_mpv_text() {
    let (mut d, rx) = deck();
    d.load("/nonexistent/re-la.flac", false);
    let seen = until(&rx, |e| matches!(e, DeckEvent::EndFile { .. }));
    match seen.last().unwrap() {
        DeckEvent::EndFile { reason: EndReason::Error, error } => assert!(!error.is_empty()),
        e => panic!("{e:?}"),
    }
}

#[test]
fn dropping_a_deck_ends_its_threads() {
    let a = wav("drop", 5.0);
    let (mut d, rx) = deck();
    d.load(a.to_str().unwrap(), false);
    until(&rx, |e| matches!(e, DeckEvent::Position(Some(_))));
    let (done, gone) = mpsc::channel();
    std::thread::spawn(move || {
        drop(d);
        let _ = done.send(());
    });
    gone.recv_timeout(Duration::from_secs(2)).expect("the deck went away within 2 s");
}

/// Not a check: a load for heaptrack (crates/rela_desktop/extras/leak-check.sh). Decks come and go, each gets 50 volume
/// changes and plays a short file twice, gapless, to the end, for RELA_SOAK_SECS seconds.
#[test]
#[ignore]
fn soak() {
    let secs = std::env::var("RELA_SOAK_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let a = wav("soak", 0.3);
    let url = a.to_str().unwrap();
    let end = Instant::now() + Duration::from_secs(secs);
    let mut rounds = 0;
    while Instant::now() < end {
        let (mut d, rx) = deck();
        for v in 0..50 {
            d.set_volume(f64::from(v));
        }
        d.load(url, false);
        d.append(url); // the gapless path: the second plays on without an EndFile for the first
        until(&rx, |e| matches!(e, DeckEvent::EndFile { .. }));
        let _ = d.audio_log();
        rounds += 1;
    }
    eprintln!("soak: {rounds} decks");
}
