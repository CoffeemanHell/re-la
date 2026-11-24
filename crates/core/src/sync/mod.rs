//! Listening together through re-with (relay protocol v1, docs/development.md).
mod machine;

mod client;

pub use client::{SyncClient, SyncListener, SyncSnapshot};
pub use machine::{Effect, REFOLLOW_TRIES, SyncEvent, SyncMachine};

/// Relay address: `override_url` if set, else <server>/re-with with http(s) -> ws(s).
pub fn relay_url(server: &str, override_url: &str) -> String {
    let o = override_url.trim();
    if !o.is_empty() {
        return o.to_string();
    }
    let Ok(mut u) = url::Url::parse(server.trim()) else { return String::new() };
    let scheme = if u.scheme() == "http" { "ws" } else { "wss" };
    let _ = u.set_scheme(scheme);
    let path = format!("{}/re-with", u.path().trim_end_matches('/'));
    u.set_path(&path);
    u.to_string()
}

/// One clock sample: client send ms, relay ms, client receive ms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockSample {
    pub c: i64,
    pub s: i64,
    pub r: i64,
}

/// Relay clock - local clock from the sample with the smallest round trip; 0 without samples.
pub fn clock_offset(samples: &[ClockSample]) -> i64 {
    samples.iter().min_by_key(|x| x.r - x.c).map_or(0, |x| x.s - (x.c + (x.r - x.c) / 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_url_cases() {
        assert_eq!(relay_url("https://music.example.com", ""), "wss://music.example.com/re-with");
        assert_eq!(relay_url("http://10.0.0.2:4533", ""), "ws://10.0.0.2:4533/re-with");
        assert_eq!(relay_url("https://music.example.com/", ""), "wss://music.example.com/re-with");
        assert_eq!(relay_url("https://x", "wss://relay.example/"), "wss://relay.example/");
    }

    #[test]
    fn clock_offset_min_rtt_wins() {
        assert_eq!(clock_offset(&[]), 0);
        // rtt 100 -> 1000-(0+50)=950 ; rtt 10 -> 2000-(1000+5)=995
        let s = [ClockSample { c: 0, s: 1000, r: 100 }, ClockSample { c: 1000, s: 2000, r: 1010 }];
        assert_eq!(clock_offset(&s), 995);
    }
}
