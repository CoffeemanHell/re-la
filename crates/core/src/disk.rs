//! What the disk caches share: a file appears whole or not at all, its name is a hash, the least recently used goes first.
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) fn hashed(key: &str) -> String {
    Sha256::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

/// As write_atomic, readable by the owner alone (listening history).
pub(crate) fn write_atomic_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let _ = std::fs::remove_file(&tmp); // a stale one would keep its old mode
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    std::io::Write::write_all(&mut open.open(&tmp)?, bytes)?;
    std::fs::rename(tmp, path)
}

pub(crate) fn touch(path: &Path) {
    if let Ok(f) = std::fs::File::options().write(true).open(path) {
        let _ = f.set_modified(SystemTime::now());
    }
}

/// Deletes the files used longest ago until `dir` holds at most `cap` bytes; `keep` and unfinished ones stay. The bytes
/// left.
pub(crate) fn trim(dir: &Path, cap: u64, keep: &HashSet<PathBuf>) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let p = e.path();
            let busy = p.extension().is_some_and(|x| x == "part" || x == "tmp");
            (m.is_file() && !busy).then(|| m.modified().ok().map(|t| (t, m.len(), p)))?
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, len, p) in files {
        if total <= cap {
            break;
        }
        if !keep.contains(&p) && std::fs::remove_file(&p).is_ok() {
            total -= len;
        }
    }
    total
}
