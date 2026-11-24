//! `<dir>/<id>.wf`: a song's levels and the key (size, suffix) they were made from. Written to a temp file
//! and renamed, so a crash never leaves half a file.
use super::{BINS, Levels};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

const MAGIC: &[u8; 4] = b"RWF1";

/// What a song's file looked like when its levels were made: a new size or type means a new file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub size: u64,
    pub suffix: String,
}

/// Ids become file names: letters, digits, `_` and `-` only.
pub fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn encode(key: &Key, l: &Levels) -> Vec<u8> {
    let suffix = &key.suffix.as_bytes()[..key.suffix.len().min(255)];
    let mut b = Vec::with_capacity(16 + suffix.len() + 2 * l.peaks.len());
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&key.size.to_le_bytes());
    b.push(suffix.len() as u8);
    b.extend_from_slice(suffix);
    b.extend_from_slice(&(l.peaks.len() as u16).to_le_bytes());
    b.extend_from_slice(&l.peaks);
    b.extend_from_slice(&l.rms);
    b
}

/// The key and where the body (`bins` u16 LE, peaks, RMS) starts.
fn header(b: &[u8]) -> Option<(Key, usize)> {
    if b.get(..4)? != MAGIC {
        return None;
    }
    let size = u64::from_le_bytes(b.get(4..12)?.try_into().ok()?);
    let len = *b.get(12)? as usize;
    let suffix = String::from_utf8(b.get(13..13 + len)?.to_vec()).ok()?;
    Some((Key { size, suffix }, 13 + len))
}

/// The part the clients get, checked to be whole.
pub fn body(b: &[u8]) -> Option<&[u8]> {
    let (_, at) = header(b)?;
    let body = b.get(at..)?;
    let bins = u16::from_le_bytes(body.get(..2)?.try_into().ok()?) as usize;
    (bins > 0 && body.len() == 2 + 2 * bins).then_some(body)
}

pub fn decode(b: &[u8]) -> Option<(Key, Levels)> {
    let (key, _) = header(b)?;
    let body = body(b)?;
    let bins = (body.len() - 2) / 2;
    Some((key, Levels { peaks: body[2..2 + bins].to_vec(), rms: body[2 + bins..].to_vec() }))
}

/// A file's key from its first bytes, if the file is as long as its header says (a cut file is made again).
fn peek(path: &std::path::Path) -> Option<Key> {
    use std::io::Read;
    let f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let mut b = Vec::with_capacity(4 + 8 + 1 + 255 + 2);
    f.take(4 + 8 + 1 + 255 + 2).read_to_end(&mut b).ok()?;
    let (key, at) = header(&b)?;
    let bins = u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as u64;
    (bins > 0 && len == at as u64 + 2 + 2 * bins).then_some(key)
}

/// The folder of `.wf` files, with every file's key in memory.
pub struct Store {
    dir: PathBuf,
    keys: HashMap<String, Key>,
}

impl Store {
    /// Makes the folder (and `.work/`), empties `.work/`, drops half-written files and reads every key.
    pub fn open(dir: PathBuf) -> io::Result<Store> {
        let work = dir.join(".work");
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work)?;
        let mut keys = HashMap::new();
        for e in std::fs::read_dir(&dir)? {
            let path = e?.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if name.ends_with(".wf.tmp") {
                let _ = std::fs::remove_file(&path);
            } else if let Some(id) = name.strip_suffix(".wf")
                && safe_id(id)
                && let Some(key) = peek(&path)
            {
                keys.insert(id.to_string(), key);
            }
        }
        Ok(Store { dir, keys })
    }

    pub fn work_dir(&self) -> PathBuf {
        self.dir.join(".work")
    }

    pub fn key(&self, id: &str) -> Option<&Key> {
        self.keys.get(id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }

    fn path(&self, id: &str) -> Option<PathBuf> {
        safe_id(id).then(|| self.dir.join(format!("{id}.wf")))
    }

    pub fn save(&mut self, id: &str, key: &Key, l: &Levels) -> io::Result<()> {
        let path = self.path(id).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unsafe id"))?;
        debug_assert_eq!(l.peaks.len(), BINS);
        let tmp = path.with_extension("wf.tmp");
        std::fs::write(&tmp, encode(key, l))?;
        std::fs::rename(&tmp, &path)?;
        self.keys.insert(id.to_string(), key.clone());
        Ok(())
    }

    pub fn remove(&mut self, id: &str) {
        if let Some(p) = self.path(id) {
            let _ = std::fs::remove_file(p);
        }
        self.keys.remove(id);
    }

    pub fn read(&self, id: &str) -> Option<Vec<u8>> {
        self.keys.contains_key(id).then(|| std::fs::read(self.path(id)?).ok()).flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::{BINS, Levels};

    fn levels(v: u8) -> Levels {
        Levels { peaks: vec![v; BINS], rms: vec![v / 2; BINS] }
    }

    fn key(size: u64) -> Key {
        Key { size, suffix: "flac".into() }
    }

    #[test]
    fn encode_decode_round_trip() {
        let b = encode(&key(42), &levels(200));
        let (k, l) = decode(&b).unwrap();
        assert_eq!(k, key(42));
        assert_eq!(l.peaks, levels(200).peaks);
        assert_eq!(l.rms, levels(200).rms);
        let body = body(&b).unwrap();
        assert_eq!(body.len(), 2 + 2 * BINS);
        assert_eq!(u16::from_le_bytes([body[0], body[1]]) as usize, BINS);
    }

    #[test]
    fn decode_rejects_bad_magic_and_short() {
        let mut b = encode(&key(1), &levels(1));
        assert!(decode(&b[..b.len() - 1]).is_none());
        assert!(decode(&[]).is_none());
        b[0] = b'X';
        assert!(decode(&b).is_none());
        assert!(body(&b).is_none());
    }

    #[test]
    fn a_cut_file_is_not_kept() {
        let dir = std::env::temp_dir().join(format!("rwf-cut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Store::open(dir.clone()).unwrap();
        s.save("a", &key(1), &levels(10)).unwrap();
        let full = std::fs::read(dir.join("a.wf")).unwrap();
        std::fs::write(dir.join("a.wf"), &full[..full.len() - 100]).unwrap(); // power cut: header whole, body short
        let s = Store::open(dir.clone()).unwrap();
        assert_eq!(s.key("a"), None, "a cut file must be made again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn safe_id_cases() {
        assert!(safe_id("a1_B-2"));
        for bad in ["", "../x", "a/b", "a.b", "a b"] {
            assert!(!safe_id(bad), "{bad}");
        }
    }

    #[test]
    fn store_reopens_and_cleans() {
        let dir = std::env::temp_dir().join(format!("rwf-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Store::open(dir.clone()).unwrap();
        s.save("a", &key(1), &levels(10)).unwrap();
        s.save("b", &key(2), &levels(20)).unwrap();
        std::fs::write(dir.join("x.wf.tmp"), b"half").unwrap();
        std::fs::write(s.work_dir().join("junk"), b"song").unwrap();
        drop(s);
        let s = Store::open(dir.clone()).unwrap();
        assert_eq!(s.key("a"), Some(&key(1)));
        assert_eq!(s.key("b"), Some(&key(2)));
        let mut ids = s.ids();
        ids.sort();
        assert_eq!(ids, ["a", "b"]);
        assert!(!dir.join("x.wf.tmp").exists());
        assert!(!s.work_dir().join("junk").exists());
        assert_eq!(decode(&s.read("a").unwrap()).unwrap().1.peaks[0], 10);
        let mut s = s;
        s.remove("a");
        assert!(s.key("a").is_none() && !dir.join("a.wf").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
