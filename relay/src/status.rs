//! What re-with is doing, for `re-with status` on the server: a unix socket (never TCP, so Caddy cannot reach it)
//! that answers every connection with one JSON snapshot.
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Off,
    Waiting,
    Listing,
    Working,
    Unreachable,
    DownloadsOff,
    Current,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Off => "off",
            Phase::Waiting => "waiting",
            Phase::Listing => "listing",
            Phase::Working => "working",
            Phase::Unreachable => "unreachable",
            Phase::DownloadsOff => "downloads-off",
            Phase::Current => "current",
        }
    }
}

/// The waveform scanner's state, in memory only.
#[derive(Debug, Default)]
pub struct Progress {
    pub phase: Phase,
    pub scanning: Option<bool>,
    pub last_scan: Option<String>,
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    /// "artist – title" and when it started.
    pub current: Option<(String, Instant)>,
    pub pass_start: Option<Instant>,
    pub last_error: Option<String>,
}

impl Progress {
    pub fn json(&self, stored: Option<usize>) -> Value {
        let secs = |i: &Instant| i.elapsed().as_secs();
        json!({
            "phase": self.phase.as_str(),
            "scanning": self.scanning,
            "last_scan": self.last_scan,
            "total": self.total,
            "done": self.done,
            "failed": self.failed,
            "current": self.current.as_ref().map(|c| &c.0),
            "current_s": self.current.as_ref().map(|c| secs(&c.1)),
            "pass_s": self.pass_start.as_ref().map(secs),
            "last_error": self.last_error,
            "stored": stored,
        })
    }
}

/// This process's resident memory in kB (Linux), None elsewhere.
pub fn rss_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines().find_map(|l| l.strip_prefix("VmRSS:"))?.split_whitespace().next()?.parse().ok()
}

/// Listens at `path` (a stale file there is removed first). Must run inside tokio.
#[cfg(unix)]
pub fn serve(path: &Path, snapshot: impl Fn() -> Value + Send + Sync + 'static) -> std::io::Result<()> {
    let _ = std::fs::remove_file(path);
    let listener = tokio::net::UnixListener::bind(path)?;
    let snapshot = Arc::new(snapshot);
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                tokio::time::sleep(Duration::from_millis(100)).await; // e.g. out of file descriptors
                continue;
            };
            let body = snapshot().to_string();
            // a reader that never reads costs one task for 2 s, never the relay
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = tokio::time::timeout(Duration::from_secs(2), s.write_all(body.as_bytes())).await;
            });
        }
    });
    Ok(())
}

/// No unix sockets: no status (re-with runs on Linux; Windows only builds it for re-la's tests).
#[cfg(not(unix))]
pub fn serve(_: &Path, _: impl Fn() -> Value + Send + Sync + 'static) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// "3 sa 12 dk", "4 dk 5 sn", "12 sn".
fn span(s: u64) -> String {
    match s {
        0..60 => format!("{s} sn"),
        60..3600 => format!("{} dk {} sn", s / 60, s % 60),
        _ => format!("{} sa {} dk", s / 3600, s % 3600 / 60),
    }
}

/// "22.11 14:02" from an RFC 3339 time (as Navidrome gives it, UTC or with its offset).
fn day_time(t: &str) -> Option<String> {
    Some(format!("{}.{} {}", t.get(8..10)?, t.get(5..7)?, t.get(11..16)?))
}

/// The screen for one snapshot; any field may be missing.
pub fn render(v: &Value) -> String {
    let n = |x: &Value| x.as_u64();
    let w = &v["waveforms"];
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    let mut head = format!("re-with {}", v["version"].as_str().unwrap_or("?"));
    if let Some(u) = n(&v["uptime_s"]) {
        head += &format!(" · {} açık", span(u));
    }
    if let Some(kb) = n(&v["rss_kb"]) {
        head += &format!(" · {} MB", kb / 1024);
    }
    line(head);
    let scan = match w["scanning"].as_bool() {
        Some(true) => "taranıyor",
        Some(false) => "tarama yok",
        None => "bilinmiyor",
    };
    let last = w["last_scan"].as_str().and_then(day_time).map(|t| format!(" · son tarama {t}")).unwrap_or_default();
    line(format!("Navidrome    {scan}{last}"));
    line(String::new());

    let phase = match w["phase"].as_str().unwrap_or("off") {
        "waiting" => "bekliyor",
        "listing" => "liste alınıyor",
        "working" => "çalışıyor",
        "unreachable" => "Navidrome'a ulaşılamıyor",
        "downloads-off" => "Navidrome'da indirme kapalı (EnableDownloads)",
        "current" => "güncel",
        _ => "kapalı",
    };
    line(format!("Dalga formu  {phase}"));
    let (total, done, failed) = (n(&w["total"]).unwrap_or(0), n(&w["done"]).unwrap_or(0), n(&w["failed"]).unwrap_or(0));
    let seen = done + failed;
    if let Some(fill) = (seen * 20).checked_div(total) {
        let fill = fill.min(20) as usize;
        let bar = "█".repeat(fill) + &"░".repeat(20 - fill);
        line(format!("  {bar}  {seen} / {total}  %{}", (seen * 100 + total / 2) / total));
    }
    if let Some(c) = w["current"].as_str() {
        line(format!("  şimdi      {c} ({})", span(n(&w["current_s"]).unwrap_or(0))));
    }
    if w["phase"] == "working"
        && let Some(p) = n(&w["pass_s"]).filter(|p| *p > 0 && seen > 0)
    {
        let left = total.saturating_sub(seen) * p / seen;
        line(format!("  hız        ~{}/dk · kalan ~{}", seen * 60 / p, span(left)));
    }
    if failed > 0 {
        let last = w["last_error"].as_str().map(|e| format!(" · son: {e}")).unwrap_or_default();
        line(format!("  hata       {failed}{last}"));
    }
    if let Some(s) = n(&w["stored"]) {
        line(format!("  diskte     {s} dosya"));
    }
    line(String::new());
    let (on, groups) = (n(&v["sync"]["online"]).unwrap_or(0), n(&v["sync"]["groups"]).unwrap_or(0));
    line(format!("Senkron      {on} çevrimiçi · {groups} grup"));
    out
}

/// `re-with status`: redraws every second on a terminal, prints once otherwise.
#[cfg(unix)]
pub fn client(path: &Path) -> std::process::ExitCode {
    use std::io::{IsTerminal, Read, Write};
    let live = std::io::stdout().is_terminal();
    loop {
        let got = std::os::unix::net::UnixStream::connect(path).and_then(|mut s| {
            s.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut b = vec![];
            s.read_to_end(&mut b)?;
            serde_json::from_slice::<Value>(&b).map_err(std::io::Error::other)
        });
        let screen = match got {
            Ok(v) => render(&v),
            Err(e) if !live => {
                eprintln!("re-with çalışmıyor ({}: {e})", path.display());
                return std::process::ExitCode::FAILURE;
            }
            Err(e) => format!("re-with çalışmıyor ({}: {e})\n", path.display()), // e.g. restarting: keep trying
        };
        if !live {
            print!("{screen}");
            return std::process::ExitCode::SUCCESS;
        }
        print!("\x1b[H\x1b[2J{screen}\nÇıkmak için Ctrl-C");
        let _ = std::io::stdout().flush();
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(not(unix))]
pub fn client(_: &Path) -> std::process::ExitCode {
    eprintln!("re-with status needs unix sockets");
    std::process::ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_working() {
        let v = json!({
            "version": "0.1.0", "uptime_s": 11520, "rss_kb": 60416,
            "sync": {"online": 3, "groups": 1},
            "waveforms": {"phase": "working", "scanning": false, "last_scan": "2026-11-22T14:02:00Z",
                "total": 7268, "done": 3118, "failed": 2, "current": "Artist – Song", "current_s": 12,
                "pass_s": 4920, "last_error": "Bad – One: ffmpeg: invalid data", "stored": 3120}
        });
        let s = render(&v);
        for want in ["3120 / 7268", "%43", "Artist – Song (12 sn)", "Bad – One: ffmpeg", "3 çevrimiçi · 1 grup", "59 MB"] {
            assert!(s.contains(want), "no {want:?} in:\n{s}");
        }
    }

    #[test]
    fn render_empty() {
        let s = render(&json!({}));
        assert!(s.contains("kapalı"), "{s}");
    }

    #[test]
    fn render_zero_total() {
        let v = json!({"waveforms": {"phase": "current", "total": 0, "done": 0, "failed": 0, "pass_s": 0}});
        assert!(render(&v).contains("güncel"));
    }
}
