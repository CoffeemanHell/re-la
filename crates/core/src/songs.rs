//! Songs kept on disk for playing without the server: one file per song and quality ("raw", or "opus-128" for a
//! transcode), named by a hash of the server and the song id.
use crate::disk;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const RAW: &str = "raw";

pub struct SongCache {
    dir: PathBuf,
}

fn stem(server: &str, id: &str) -> String {
    disk::hashed(&format!("{server}\n{id}"))
}

impl SongCache {
    /// What a crash left half done goes.
    pub fn open(dir: PathBuf) -> SongCache {
        let _ = std::fs::create_dir_all(&dir);
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "part") {
                let _ = std::fs::remove_file(e.path());
            }
        }
        SongCache { dir }
    }

    pub fn path(&self, server: &str, id: &str, quality: &str) -> PathBuf {
        self.dir.join(format!("{}.{quality}", stem(server, id)))
    }

    /// The song in this quality, or the original, which is at least as good; `any`: whatever there is.
    pub fn find(&self, server: &str, id: &str, quality: &str, any: bool) -> Option<PathBuf> {
        let exact = [quality, RAW].into_iter().map(|q| self.path(server, id, q)).find(|p| p.is_file());
        if exact.is_some() || !any {
            return exact;
        }
        let stem = stem(server, id);
        let held = |p: &Path| p.file_stem().is_some_and(|s| s == stem.as_str()) && p.extension().is_some_and(|x| x != "part");
        std::fs::read_dir(&self.dir).ok()?.flatten().map(|e| e.path()).find(|p| held(p))
    }

    /// The songs held in any quality; `holds` asks about one.
    pub fn held(&self) -> HashSet<String> {
        let files = std::fs::read_dir(&self.dir).into_iter().flatten().flatten().map(|e| e.path());
        files.filter(|p| p.extension().is_some_and(|x| x != "part")).filter_map(|p| Some(p.file_stem()?.to_str()?.to_string())).collect()
    }

    pub fn holds(held: &HashSet<String>, server: &str, id: &str) -> bool {
        held.contains(&stem(server, id))
    }

    pub fn trim(&self, cap: u64, keep: &HashSet<PathBuf>) -> u64 {
        disk::trim(&self.dir, cap, keep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn cache(name: &str) -> SongCache {
        let d = std::env::temp_dir().join(format!("re-la-songs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        SongCache::open(d)
    }

    fn put(c: &SongCache, id: &str, quality: &str, len: usize, age_s: u64) -> PathBuf {
        let p = c.path("https://m", id, quality);
        std::fs::write(&p, vec![0u8; len]).unwrap();
        std::fs::File::options().write(true).open(&p).unwrap().set_modified(SystemTime::now() - Duration::from_secs(age_s)).unwrap();
        p
    }

    #[test]
    fn the_wanted_quality_then_the_original_then_any() {
        let c = cache("find");
        let opus = put(&c, "a", "opus-128", 1, 0);
        assert_eq!(c.find("https://m", "a", "opus-128", false), Some(opus.clone()));
        assert_eq!(c.find("https://m", "a", RAW, false), None, "a transcode is not the original");
        assert_eq!(c.find("https://m", "a", RAW, true), Some(opus));
        assert_eq!(c.find("https://other", "a", RAW, true), None, "another server's song");
        let raw = put(&c, "a", RAW, 1, 0);
        assert_eq!(c.find("https://m", "a", "mp3-96", false), Some(raw));
        assert!(SongCache::holds(&c.held(), "https://m", "a") && !SongCache::holds(&c.held(), "https://m", "b"));
    }

    #[test]
    fn over_the_cap_the_oldest_go_but_not_the_kept() {
        let c = cache("trim");
        let (old, mid, new) = (put(&c, "old", RAW, 100, 300), put(&c, "mid", RAW, 100, 200), put(&c, "new", RAW, 100, 100));
        let part = put(&c, "x", "raw.part", 100, 900);
        assert_eq!(c.trim(250, &HashSet::from([old.clone()])), 200);
        assert!(old.exists() && !mid.exists() && new.exists() && part.exists());
        let c = SongCache::open(c.dir.clone());
        assert!(!part.exists(), "left by a crash");
        assert_eq!(c.trim(0, &HashSet::new()), 0);
    }
}
