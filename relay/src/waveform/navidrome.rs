//! re-with's own Navidrome account: scan status, the song list, a song's original file.
use super::Key;
use md5::{Digest, Md5};
use serde_json::Value;
use std::hash::BuildHasher;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
use tokio::io::AsyncWriteExt;

/// Songs per search3 page.
const PAGE: usize = 500;
/// The longest a song's file may take to arrive (127.0.0.1: only a huge file gets near it).
const DOWNLOAD_LIMIT: Duration = Duration::from_secs(600);

pub struct Account {
    pub user: String,
    pub password: String,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account").field("user", &self.user).finish_non_exhaustive() // never the password
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub id: String,
    pub key: Key,
    /// "artist – title", for `re-with status`.
    pub title: String,
}

pub struct Navidrome {
    http: reqwest::Client,
    base: String,
    acct: Account,
}

/// Never repeats within a run; a Subsonic salt needs no secrecy, only freshness.
fn salt() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let h = std::hash::RandomState::new().hash_one((SystemTime::now(), N.fetch_add(1, Ordering::Relaxed)));
    format!("{h:016x}")
}

impl Navidrome {
    pub fn new(http: reqwest::Client, base: String, acct: Account) -> Navidrome {
        Navidrome { http, base: base.trim_end_matches('/').to_string(), acct }
    }

    fn url(&self, method: &str, params: &[(&str, &str)]) -> Option<reqwest::Url> {
        let s = salt();
        let t: String = Md5::digest(format!("{}{s}", self.acct.password).as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        let mut q = vec![("u", self.acct.user.as_str()), ("t", &t), ("s", &s), ("v", "1.16.1"), ("c", "re-with"), ("f", "json")];
        q.extend_from_slice(params);
        reqwest::Url::parse_with_params(&format!("{}/rest/{method}", self.base), &q).ok()
    }

    /// The `subsonic-response` of an ok answer; None for anything else.
    async fn call(&self, method: &str, params: &[(&str, &str)]) -> Option<Value> {
        let resp = self.http.get(self.url(method, params)?).send().await.ok()?;
        let mut v: Value = serde_json::from_slice(&resp.bytes().await.ok()?).ok()?;
        let r = v.get_mut("subsonic-response")?.take();
        (r["status"] == "ok").then_some(r)
    }

    /// (scanning, lastScan); None when Navidrome doesn't answer.
    pub async fn scan_status(&self) -> Option<(bool, Option<String>)> {
        let r = self.call("getScanStatus", &[]).await?;
        let s = &r["scanStatus"];
        Some((s["scanning"].as_bool().unwrap_or(false), s["lastScan"].as_str().map(str::to_string)))
    }

    /// Every song; None if any page failed (a partial list must not delete anything).
    pub async fn songs(&self) -> Option<Vec<Song>> {
        let mut all = Vec::new();
        loop {
            let (count, offset) = (PAGE.to_string(), all.len().to_string());
            let q = [("query", "\"\""), ("artistCount", "0"), ("albumCount", "0"), ("songCount", &count), ("songOffset", &offset)];
            let r = self.call("search3", &q).await?;
            let page = r["searchResult3"]["song"].as_array().cloned().unwrap_or_default();
            let n = page.len();
            all.extend(page.iter().filter_map(|s| {
                let key = Key { size: s["size"].as_u64()?, suffix: s["suffix"].as_str().unwrap_or_default().to_string() };
                let title = match (s["artist"].as_str(), s["title"].as_str()) {
                    (Some(a), Some(t)) => format!("{a} – {t}"),
                    (a, t) => t.or(a).unwrap_or_default().to_string(),
                };
                Some(Song { id: s["id"].as_str()?.to_string(), key, title })
            }));
            if n < PAGE {
                return Some(all);
            }
        }
    }

    /// The song's original file into `to`.
    pub async fn download(&self, id: &str, to: &Path) -> Result<(), Fetch> {
        // reqwest errors print their URL, which holds the token: always `without_url`
        let down = |e: reqwest::Error| Fetch::Unreachable(e.without_url().to_string());
        let url = self.url("download", &[("id", id)]).ok_or(Fetch::Failed("bad url".into()))?;
        let mut resp = self.http.get(url).timeout(DOWNLOAD_LIMIT).send().await.map_err(down)?;
        if !resp.status().is_success() {
            return Err(Fetch::Failed(format!("HTTP {}", resp.status().as_u16())));
        }
        let json = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).is_some_and(|t| t.contains("json"));
        if json {
            // a Subsonic error comes as JSON with status 200
            let b = resp.bytes().await.map_err(down)?;
            let v: Value = serde_json::from_slice(&b).unwrap_or_default();
            let e = &v["subsonic-response"]["error"];
            return Err(match e["code"].as_u64() {
                Some(50) => Fetch::Disabled,
                _ => Fetch::Failed(e["message"].as_str().unwrap_or("not audio").to_string()),
            });
        }
        let write = |e: std::io::Error| Fetch::Failed(format!("write: {e}"));
        let mut f = tokio::fs::File::create(to).await.map_err(write)?;
        while let Some(chunk) = resp.chunk().await.map_err(down)? {
            f.write_all(&chunk).await.map_err(write)?;
        }
        f.flush().await.map_err(write)
    }
}

/// Why a song's file did not come.
#[derive(Debug, PartialEq, Eq)]
pub enum Fetch {
    /// Navidrome is not answering (down, restarting): the pass stops and is tried again.
    Unreachable(String),
    /// Navidrome refuses downloads (`EnableDownloads` off).
    Disabled,
    /// This song only.
    Failed(String),
}

impl std::fmt::Display for Fetch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fetch::Unreachable(e) => write!(f, "Navidrome unreachable: {e}"),
            Fetch::Disabled => write!(f, "downloads disabled"),
            Fetch::Failed(e) => write!(f, "download: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn errors_never_carry_the_token() {
        let acct = Account { user: "re-with".into(), password: "pw".into() };
        let nd = Navidrome::new(reqwest::Client::new(), "http://127.0.0.1:1".into(), acct);
        let e = nd.download("a", Path::new("/nonexistent/a")).await.unwrap_err();
        assert!(!e.to_string().contains("t=") && !e.to_string().contains("s="), "{e}");
    }
}
