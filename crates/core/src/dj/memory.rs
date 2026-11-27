//! What this listener did with each song over the months: skips that fade (half-life `Knobs::memory_days`), the day last heard, and
//! which genres were heard back to back. On disk as JSON.

use crate::dj::Knobs;
use crate::dj::listen::Heard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Days since 1970 of a wall clock in ms.
pub fn day(clock_ms: f64) -> u32 {
    (clock_ms / 86_400_000.0).max(0.0) as u32
}

fn faded(v: f32, from: u32, to: u32, k: &Knobs) -> f32 {
    v * 0.5f32.powf(to.saturating_sub(from) as f32 / k.memory_days)
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Entry {
    skip: f32,
    skip_day: u32, // the day `skip` was last brought up to date
    last_skip_day: u32,
    skip_days: u8, // different days with an early skip
    last_heard: u32,
    heard: u16,
    // "more like this" (1) / "less like this" (-1), when, and what it was called then (the index keeps no titles)
    #[serde(default, skip_serializing_if = "is_zero")]
    fb: i8,
    #[serde(default, skip_serializing_if = "is_zero")]
    fb_day: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    artist: String,
}

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// One "more / less like this", newest first in `Memory::feedback_list`.
#[derive(Clone, Debug, PartialEq)]
pub struct Feedback {
    pub id: String,
    pub dir: i8,
    pub day: u32,
    pub title: String,
    pub artist: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Pair {
    n: f32,
    day: u32,
}

#[derive(Serialize, Deserialize, Default)]
struct File {
    v: u32,
    songs: HashMap<String, Entry>,
    pairs: HashMap<String, Pair>,
}

pub struct Memory {
    path: PathBuf,
    file: File,
    dirty: bool,
}

impl Memory {
    /// From disk; empty when the file is missing, broken or of another version.
    pub fn open(path: PathBuf) -> Memory {
        let file = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<File>(&b).ok())
            .filter(|f| f.v == 1)
            .unwrap_or(File { v: 1, ..Default::default() });
        Memory { path, file, dirty: false }
    }

    pub fn record(&mut self, id: &str, h: Heard, day: u32, k: &Knobs) {
        if h == Heard::Neutral {
            return;
        }
        let e = self.file.songs.entry(id.to_string()).or_default();
        e.skip = faded(e.skip, e.skip_day, day, k);
        e.skip_day = day;
        match h {
            Heard::EarlySkip => {
                if e.skip < 0.5 {
                    e.skip_days = 0; // what was held against it has faded: old days do not count again
                }
                e.skip += 1.0;
                if e.last_skip_day != day || e.skip_days == 0 {
                    e.skip_days = e.skip_days.saturating_add(1);
                    e.last_skip_day = day;
                }
            }
            Heard::Full => {
                e.skip = (e.skip - 1.0).max(0.0);
                e.last_heard = day;
                e.heard = e.heard.saturating_add(1);
            }
            Heard::Neutral => {}
        }
        self.dirty = true;
    }

    /// Skipped early again and again, on more than one day; fades back out over the months.
    pub fn disliked(&self, id: &str, day: u32, k: &Knobs) -> bool {
        self.file.songs.get(id).is_some_and(|e| e.skip_days >= k.dislike_days && faded(e.skip, e.skip_day, day, k) >= k.dislike_skips)
    }

    /// The faded early-skip count.
    pub fn skip(&self, id: &str, day: u32, k: &Knobs) -> f32 {
        self.file.songs.get(id).map_or(0.0, |e| faded(e.skip, e.skip_day, day, k))
    }

    /// The listener asked for more (1) or less (-1) like this song; 0 takes it back.
    pub fn set_feedback(&mut self, id: &str, dir: i8, day: u32, title: &str, artist: &str) {
        let e = self.file.songs.entry(id.to_string()).or_default();
        e.fb = dir.clamp(-1, 1);
        e.fb_day = if dir == 0 { 0 } else { day };
        (e.title, e.artist) = if dir == 0 { Default::default() } else { (title.to_string(), artist.to_string()) };
        self.dirty = true;
    }

    pub fn feedback(&self, id: &str) -> i8 {
        self.file.songs.get(id).map_or(0, |e| e.fb)
    }

    pub fn feedback_list(&self) -> Vec<Feedback> {
        let mut v: Vec<Feedback> = self
            .file
            .songs
            .iter()
            .filter(|(_, e)| e.fb != 0)
            .map(|(id, e)| Feedback { id: id.clone(), dir: e.fb, day: e.fb_day, title: e.title.clone(), artist: e.artist.clone() })
            .collect();
        v.sort_by(|a, b| b.day.cmp(&a.day).then_with(|| a.id.cmp(&b.id)));
        v
    }

    pub fn last_heard(&self, id: &str) -> Option<u32> {
        self.file.songs.get(id).filter(|e| e.heard > 0).map(|e| e.last_heard)
    }

    fn pair_key(a: &str, b: &str) -> String {
        if a <= b { format!("{a}\n{b}") } else { format!("{b}\n{a}") }
    }

    /// Two genres heard in full one after the other.
    pub fn record_pair(&mut self, a: &str, b: &str, day: u32, k: &Knobs) {
        let p = self.file.pairs.entry(Self::pair_key(a, b)).or_default();
        p.n = faded(p.n, p.day, day, k) + 1.0;
        p.day = day;
        self.dirty = true;
    }

    /// (a, b, faded count) with a <= b.
    pub fn pairs(&self, day: u32, k: &Knobs) -> Vec<(String, String, f32)> {
        self.file
            .pairs
            .iter()
            .filter_map(|(key, p)| {
                let (a, b) = key.split_once('\n')?;
                Some((a.to_string(), b.to_string(), faded(p.n, p.day, day, k)))
            })
            .collect()
    }

    /// Writes when something changed since the last save.
    pub fn save(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(d) = self.path.parent() {
            std::fs::create_dir_all(d)?;
        }
        crate::disk::write_atomic_private(&self.path, &serde_json::to_vec(&self.file).map_err(std::io::Error::other)?)?;
        self.dirty = false;
        Ok(())
    }

    /// Forgets everything, on disk too: the file goes now rather than waiting for a write that may fail.
    pub fn clear(&mut self) {
        self.file = File { v: 1, ..Default::default() };
        let _ = std::fs::remove_file(self.path.with_extension("tmp")); // a write that died leaves its copy
        self.dirty = std::fs::remove_file(&self.path).is_err_and(|e| e.kind() != std::io::ErrorKind::NotFound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dj::listen::Heard::*;

    fn kn() -> Knobs {
        Knobs::default()
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("re-la-djmem-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("dj-memory.json")
    }

    fn two_days(m: &mut Memory) {
        for d in [1, 1, 2, 2] {
            m.record("x", EarlySkip, d, &kn());
        }
    }

    // forgotten on disk at once: no later write has to succeed (or happen) for it to stay forgotten
    #[test]
    fn clear_forgets_on_disk() {
        let p = tmp("clear-on-disk");
        let mut m = Memory::open(p.clone());
        two_days(&mut m);
        m.save().unwrap();
        std::fs::write(p.with_extension("tmp"), b"{}").unwrap(); // left by a write that died
        m.clear();
        assert!(!p.with_extension("tmp").exists());
        assert!(Memory::open(p).skip("x", 2, &kn()) == 0.0);
    }

    // listening history: the owner's alone
    #[cfg(unix)]
    #[test]
    fn saved_private() {
        use std::os::unix::fs::PermissionsExt;
        let p = tmp("private");
        let mut m = Memory::open(p.clone());
        two_days(&mut m);
        m.save().unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn one_bad_day() {
        let mut m = Memory::open(tmp("bad"));
        for _ in 0..5 {
            m.record("x", EarlySkip, 1, &kn());
        }
        assert!(!m.disliked("x", 1, &kn()));
    }

    #[test]
    fn two_days_dislike_then_forget() {
        let mut m = Memory::open(tmp("two"));
        two_days(&mut m);
        assert!(m.disliked("x", 2, &kn()));
        assert!(!m.disliked("x", 2 + 120, &kn()));
        assert!(!m.disliked("other", 2, &kn()));
    }

    // a dislike that faded away does not come back from one bad evening months later
    #[test]
    fn old_days_do_not_count_again() {
        let mut m = Memory::open(tmp("old"));
        two_days(&mut m);
        for _ in 0..3 {
            m.record("x", EarlySkip, 300, &kn());
        }
        assert!(!m.disliked("x", 300, &kn()));
    }

    #[test]
    fn full_listen_heals() {
        let mut m = Memory::open(tmp("heal"));
        two_days(&mut m);
        m.record("x", Full, 2, &kn());
        m.record("x", Full, 2, &kn());
        assert!(!m.disliked("x", 2, &kn()));
    }

    #[test]
    fn skip_count() {
        let mut m = Memory::open(tmp("skip"));
        m.record("x", EarlySkip, 0, &kn());
        assert_eq!(m.skip("x", 0, &kn()), 1.0);
        assert!((m.skip("x", 60, &kn()) - 0.5).abs() < 1e-4);
        assert_eq!(m.skip("y", 0, &kn()), 0.0);
    }

    #[test]
    fn last_heard() {
        let mut m = Memory::open(tmp("heard"));
        m.record("x", Full, 7, &kn());
        m.record("x", Neutral, 9, &kn());
        m.record("x", EarlySkip, 10, &kn());
        assert_eq!(m.last_heard("x"), Some(7));
        assert_eq!(m.last_heard("y"), None);
    }

    #[test]
    fn pairs_decay() {
        let mut m = Memory::open(tmp("pairs"));
        m.record_pair("Rock", "Metal", 0, &kn());
        m.record_pair("Metal", "Rock", 0, &kn());
        let at = |m: &Memory, d| m.pairs(d, &kn()).iter().find(|p| p.0 == "Metal" && p.1 == "Rock").map(|p| p.2).unwrap();
        assert!((at(&m, 0) - 2.0).abs() < 1e-4);
        assert!((at(&m, 60) - 1.0).abs() < 1e-3);
        assert_eq!(m.pairs(0, &kn()).len(), 1);
    }

    #[test]
    fn roundtrip() {
        let p = tmp("round");
        let mut m = Memory::open(p.clone());
        two_days(&mut m);
        m.record("y", Full, 3, &kn());
        m.record_pair("A", "B", 3, &kn());
        m.save().unwrap();
        let m = Memory::open(p);
        assert!(m.disliked("x", 2, &kn()));
        assert_eq!(m.last_heard("y"), Some(3));
        assert_eq!(m.pairs(3, &kn()).len(), 1);
    }

    #[test]
    fn corrupt_file() {
        let p = tmp("corrupt");
        std::fs::write(&p, "{nope").unwrap();
        let mut m = Memory::open(p.clone());
        assert!(m.last_heard("x").is_none());
        m.record("x", Full, 1, &kn());
        m.save().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v["v"], 1);
    }

    #[test]
    fn clear_forgets() {
        let mut m = Memory::open(tmp("clear"));
        two_days(&mut m);
        m.clear();
        assert!(!m.disliked("x", 2, &kn()));
    }

    #[test]
    fn day_of_clock() {
        assert_eq!(day(0.0), 0);
        assert_eq!(day(86_400_000.0 * 3.5), 3);
    }

    // a new threshold judges skips already on record
    #[test]
    fn dislike_threshold_applies_to_old_skips() {
        let mut m = Memory::open(tmp("threshold"));
        let k = Knobs::default();
        for d in [1, 1, 2, 2] {
            m.record("x", EarlySkip, d, &k);
        }
        assert!(m.disliked("x", 2, &k));
        assert!(!m.disliked("x", 2, &Knobs { dislike_skips: 5.0, ..Knobs::default() }));
    }

    #[test]
    fn memory_days_sets_half_life() {
        let mut m = Memory::open(tmp("half-life"));
        let k = Knobs { memory_days: 30.0, ..Knobs::default() };
        m.record("x", EarlySkip, 0, &k);
        assert!((m.skip("x", 30, &k) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn feedback_roundtrip() {
        let p = tmp("feedback");
        let mut m = Memory::open(p.clone());
        m.set_feedback("x", 1, 5, "Song X", "Artist X");
        m.set_feedback("y", -1, 6, "Song Y", "Artist Y");
        m.save().unwrap();
        let mut m = Memory::open(p);
        assert_eq!((m.feedback("x"), m.feedback("y"), m.feedback("z")), (1, -1, 0));
        let list = m.feedback_list();
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].id.as_str(), list[0].dir, list[0].day, list[0].title.as_str()), ("y", -1, 6, "Song Y"));
        m.set_feedback("x", 0, 7, "", "");
        assert_eq!(m.feedback("x"), 0);
        assert_eq!(m.feedback_list().len(), 1);
    }

    // a file from before feedback opens with everything it had
    #[test]
    fn old_file_without_fb_opens() {
        let p = tmp("old-file");
        std::fs::write(&p, r#"{"v":1,"songs":{"a":{"skip":1.0,"skip_day":5,"last_skip_day":5,"skip_days":1,"last_heard":0,"heard":0}},"pairs":{}}"#).unwrap();
        let m = Memory::open(p);
        assert_eq!(m.skip("a", 5, &kn()), 1.0);
        assert_eq!(m.feedback("a"), 0);
    }

    #[test]
    fn dislike_days_one() {
        let mut m = Memory::open(tmp("days-one"));
        for _ in 0..4 {
            m.record("x", EarlySkip, 1, &kn());
        }
        assert!(!m.disliked("x", 1, &kn()));
        assert!(m.disliked("x", 1, &Knobs { dislike_days: 1, ..Knobs::default() }));
    }
}
