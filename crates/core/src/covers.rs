//! Cover art disk cache. Entries are keyed by the getCoverArt URL without the login (`without_login`), so the files
//! hold no secret and a new login keeps the covers. The server's cache headers decide when to ask again.
use crate::disk::{hashed, touch, write_atomic};
use crate::queue::without_login;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Freshness when the server says nothing.
const DEFAULT_FRESH: Duration = Duration::from_secs(24 * 3600);

pub struct CoverCache {
    dir: PathBuf,
    cap: u64,
    /// Bytes held, at most: what is written again counts twice until evict counts the files (as the first store does).
    held: AtomicU64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Meta {
    key: String,
    expires_ms: i64,
    etag: Option<String>,
    last_modified: Option<String>,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// (store at all, ms it stays fresh) from Cache-Control. shortcut: Expires is ignored; Navidrome sends max-age.
fn freshness(headers: &reqwest::header::HeaderMap) -> (bool, i64) {
    let cc = headers.get("cache-control").and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let mut fresh = DEFAULT_FRESH.as_millis() as i64;
    for part in cc.split(',').map(str::trim) {
        if part == "no-store" {
            return (false, 0);
        }
        if part == "no-cache" {
            fresh = 0;
        }
        if let Some(n) = part.strip_prefix("max-age=").and_then(|n| n.parse::<i64>().ok()) {
            fresh = n.saturating_mul(1000);
        }
    }
    (true, fresh)
}

fn header(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string)
}

impl CoverCache {
    /// `dir` is created; a sibling `covers` folder (the old Qt disk cache) is deleted.
    pub fn open(dir: PathBuf, cap_bytes: u64) -> CoverCache {
        if let Some(old) = dir.parent().map(|p| p.join("covers")).filter(|o| *o != dir) {
            let _ = std::fs::remove_dir_all(old);
        }
        let _ = std::fs::create_dir_all(&dir);
        CoverCache { dir, cap: cap_bytes, held: AtomicU64::new(u64::MAX) }
    }

    fn paths(&self, key: &str) -> (PathBuf, PathBuf) {
        let h = hashed(key);
        (self.dir.join(format!("{h}.img")), self.dir.join(format!("{h}.meta")))
    }

    fn read(&self, key: &str) -> Option<(Vec<u8>, Meta)> {
        let (img, meta) = self.paths(key);
        let m: Meta = serde_json::from_slice(&std::fs::read(&meta).ok()?).ok()?;
        let body = std::fs::read(&img).ok()?;
        touch(&img); // the cap drops the least recently used first
        Some((body, m))
    }

    fn store(&self, key: &str, body: &[u8], meta: &Meta) {
        let (img, m) = self.paths(key);
        let meta = serde_json::to_vec(meta).unwrap_or_default();
        if write_atomic(&img, body).is_ok() && write_atomic(&m, &meta).is_ok() {
            let held = self.held.load(Ordering::Relaxed).saturating_add((body.len() + meta.len()) as u64);
            self.held.store(if held > self.cap { self.evict() } else { held }, Ordering::Relaxed);
        }
    }

    /// Over the cap: the entries used longest ago go first, down to 90 % of it, so a full cache is not read through for
    /// every cover. The bytes left.
    fn evict(&self) -> u64 {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return 0 };
        let mut entries: Vec<(SystemTime, u64, PathBuf)> = rd
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "img"))
            .filter_map(|e| {
                let p = e.path();
                let meta_len = std::fs::metadata(p.with_extension("meta")).map_or(0, |m| m.len());
                let m = e.metadata().ok()?;
                Some((m.modified().ok()?, m.len() + meta_len, p))
            })
            .collect();
        let mut total: u64 = entries.iter().map(|e| e.1).sum();
        if total <= self.cap {
            return total;
        }
        entries.sort_by_key(|e| e.0);
        for (_, len, p) in entries {
            if total <= self.cap - self.cap / 10 {
                break;
            }
            let _ = std::fs::remove_file(&p);
            let _ = std::fs::remove_file(p.with_extension("meta"));
            total = total.saturating_sub(len);
        }
        total
    }

    /// The cover's bytes: a fresh entry without asking; otherwise the server (revalidating with the stored ETag or
    /// Last-Modified), falling back to a stale entry when it can't answer. `cache_only` (offline): never the network.
    pub async fn get(&self, http: &reqwest::Client, url: &url::Url, cache_only: bool) -> Option<Vec<u8>> {
        let key = without_login(url).to_string();
        let cached = self.read(&key);
        if cache_only {
            return cached.map(|(b, _)| b);
        }
        if let Some((body, m)) = &cached
            && m.expires_ms > now_ms()
        {
            return Some(body.clone());
        }
        let mut req = http.get(url.clone());
        if let Some((_, m)) = &cached {
            if let Some(e) = &m.etag {
                req = req.header("if-none-match", e);
            }
            if let Some(l) = &m.last_modified {
                req = req.header("if-modified-since", l);
            }
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(_) => return cached.map(|(b, _)| b),
        };
        let (keep, fresh) = freshness(resp.headers());
        let etag = header(resp.headers(), "etag");
        let last_modified = header(resp.headers(), "last-modified");
        match resp.status().as_u16() {
            304 => {
                let (body, mut m) = cached?;
                m.expires_ms = now_ms() + fresh;
                if etag.is_some() {
                    m.etag = etag;
                }
                let _ = write_atomic(&self.paths(&key).1, &serde_json::to_vec(&m).unwrap_or_default()); // the image is the same
                Some(body)
            }
            200 => {
                let body = resp.bytes().await.ok()?.to_vec();
                if keep {
                    self.store(&key, &body, &Meta { key: key.clone(), expires_ms: now_ms() + fresh, etag, last_modified });
                } else {
                    let (img, meta) = self.paths(&key);
                    let _ = std::fs::remove_file(img);
                    let _ = std::fs::remove_file(meta);
                }
                Some(body)
            }
            _ => cached.map(|(b, _)| b),
        }
    }
}
