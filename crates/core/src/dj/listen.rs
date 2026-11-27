//! What a song's end says: heard in full, left early, or nothing to learn from.

use crate::dj::Knobs;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Heard {
    Full,
    Neutral,
    EarlySkip,
}

/// RMS (0..1) below which a part of the waveform counts as silence.
pub const SILENCE_RMS: f32 = 0.02;

/// Where the music ends, as a share of the song: the start of a silence of 20 s or more that has at least 20 s of
/// music before it (so a silent intro does not count). The last such silence is taken, so one before a hidden track
/// counts. 1.0 when there is none.
pub fn music_end(rms: &[f32], dur: f64) -> f64 {
    let n = rms.len();
    if n == 0 || dur <= 0.0 {
        return 1.0;
    }
    let min = ((20.0 / dur) * n as f64).ceil() as usize;
    let loud_before = |i: usize| rms[..i].iter().filter(|&&x| x >= SILENCE_RMS).count();
    let mut end = n; // one past the silent run being measured
    let mut i = n;
    while i > 0 {
        i -= 1;
        if rms[i] >= SILENCE_RMS {
            end = i;
            continue;
        }
        let start = i;
        if (i == 0 || rms[i - 1] >= SILENCE_RMS) && end - start >= min.max(1) && start > 0 && loud_before(start) >= min {
            return start as f64 / n as f64;
        }
    }
    1.0
}

/// Silent around this share of the song: the part and its neighbours within 3 s.
fn silent_at(rms: &[f32], frac: f64, dur: f64) -> bool {
    let n = rms.len();
    let at = ((frac * n as f64) as usize).min(n - 1);
    let half = ((3.0 / dur) * n as f64) as usize;
    rms[at.saturating_sub(half)..(at + half + 1).min(n)].iter().all(|&x| x < SILENCE_RMS)
}

/// A song left at `pos` of `dur` seconds (`natural_end`: it played to the end). With its waveform, leaving in a
/// silence or after the music counts as heard in full.
pub fn classify(pos: f64, dur: f64, natural_end: bool, rms: Option<&[f32]>, k: &Knobs) -> Heard {
    if natural_end {
        return Heard::Full;
    }
    if dur <= 0.0 {
        return Heard::Neutral;
    }
    let frac = (pos / dur).clamp(0.0, 1.0);
    let mut end = 1.0;
    if let Some(r) = rms.filter(|r| !r.is_empty()) {
        end = music_end(r, dur);
        let music_before = r[..((frac * r.len() as f64) as usize).min(r.len())].iter().any(|&x| x >= SILENCE_RMS);
        if (end < 1.0 && frac >= end) || (music_before && silent_at(r, frac, dur)) {
            return Heard::Full;
        }
    }
    let ratio = frac / end;
    if ratio >= k.full_share {
        Heard::Full
    } else if ratio < k.skip_share && pos < k.skip_max_sec {
        Heard::EarlySkip
    } else {
        Heard::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4096 parts: the given ranges loud (0.3), the rest silent.
    fn wave(loud: &[(usize, usize)]) -> Vec<f32> {
        let mut w = vec![0.0; 4096];
        for &(a, b) in loud {
            w[a..b].iter_mut().for_each(|x| *x = 0.3);
        }
        w
    }

    #[test]
    fn trailing_silence() {
        assert!((music_end(&wave(&[(0, 2048)]), 300.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn hidden_track() {
        let w = wave(&[(0, 820), (3700, 4096)]);
        assert!((music_end(&w, 900.0) - 820.0 / 4096.0).abs() < 0.01);
    }

    #[test]
    fn silent_intro_ignored() {
        assert_eq!(music_end(&wave(&[(512, 4096)]), 200.0), 1.0);
    }

    #[test]
    fn short_gap_ignored() {
        // 10 s of a 300 s song is ~137 parts
        assert_eq!(music_end(&wave(&[(0, 3000), (3137, 4096)]), 300.0), 1.0);
    }

    // skipping inside a silent intro is leaving a song unheard, not a silence to forgive
    #[test]
    fn silent_intro_skip_is_early() {
        let w = wave(&[(512, 4096)]); // 200 s song, first ~25 s silent
        assert_eq!(classify(10.0, 200.0, false, Some(&w), &Knobs::default()), Heard::EarlySkip);
        assert_eq!(classify(0.0, 200.0, false, Some(&w), &Knobs::default()), Heard::EarlySkip);
    }

    #[test]
    fn classify_cases() {
        let tail = wave(&[(0, 2048)]);
        assert_eq!(classify(10.0, 300.0, true, None, &Knobs::default()), Heard::Full);
        assert_eq!(classify(160.0, 300.0, false, Some(&tail), &Knobs::default()), Heard::Full); // the music is over
        let gap = wave(&[(0, 1000), (1500, 4096)]);
        assert_eq!(classify(300.0 * 1200.0 / 4096.0, 300.0, false, Some(&gap), &Knobs::default()), Heard::Full); // skipped in silence
        assert_eq!(classify(30.0, 300.0, false, None, &Knobs::default()), Heard::EarlySkip);
        assert_eq!(classify(150.0, 300.0, false, None, &Knobs::default()), Heard::Neutral);
        assert_eq!(classify(181.0, 300.0, false, None, &Knobs::default()), Heard::Neutral);
        assert_eq!(classify(280.0, 300.0, false, None, &Knobs::default()), Heard::Full);
        assert_eq!(classify(5.0, 0.0, false, None, &Knobs::default()), Heard::Neutral);
    }

    #[test]
    fn skip_share_moves_early_skip() {
        let mut k = Knobs::default();
        assert_eq!(classify(100.0, 300.0, false, None, &k), Heard::Neutral);
        k.skip_share = 0.5;
        assert_eq!(classify(100.0, 300.0, false, None, &k), Heard::EarlySkip);
    }
}
