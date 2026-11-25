//! Where a song plays from: the song cache, or the server in the quality this network is set to. And filling that
//! cache: the songs coming up, then the one playing once it counts as played.
use super::state::{Job, PlayerCore};
use crate::queue::{next_index, text};
use crate::songs::{RAW, SongCache};
use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

pub(super) struct Source {
    pub(super) url: String,
    /// The server makes the stream as it sends it: no ranges, a seek asks for a new one (timeOffset).
    pub(super) live_transcode: bool,
}

/// "opus-128" as (format, kbps); None for the original or anything else.
fn parse_transcode(s: &str) -> Option<(&str, i64)> {
    let (format, kbps) = s.split_once('-')?;
    let kbps = kbps.parse().ok().filter(|k| *k > 0)?;
    (!format.is_empty() && format.bytes().all(|b| b.is_ascii_alphanumeric())).then_some((format, kbps))
}

impl PlayerCore {
    fn option_text(&self, key: &str) -> String {
        self.options.get(key).map(text).unwrap_or_default()
    }

    fn option_num(&self, key: &str) -> i64 {
        self.options.get(key).and_then(Value::as_i64).unwrap_or(0)
    }

    /// The transcode set for this network as (format, kbps); None: the original, also when it is small enough already
    /// (the server sends it as it is then, and offsets would be wrong).
    fn transcode(&self, id: &str) -> Option<(String, i64)> {
        let set = self.option_text(if self.mobile { "transcodeMobile" } else { "transcodeWifi" });
        let (format, kbps) = parse_transcode(&set)?;
        let song = self.queue.iter().find(|s| text(&s["id"]) == id).unwrap_or(&Value::Null);
        let rate = song["bitRate"].as_i64().unwrap_or(0);
        let same = text(&song["suffix"]).eq_ignore_ascii_case(format);
        let small = if rate > 0 { rate <= kbps } else { same };
        (!small).then(|| (format.to_string(), kbps))
    }

    fn quality(&self, id: &str) -> String {
        self.transcode(id).map_or(RAW.into(), |(f, k)| format!("{f}-{k}"))
    }

    /// Without the server any copy will do.
    pub(super) fn source(&self, id: &str) -> Source {
        if let Some(p) = self.songs.find(&self.login.server, id, &self.quality(id), !self.online) {
            crate::disk::touch(&p);
            return Source { url: p.to_string_lossy().into_owned(), live_transcode: false };
        }
        let Some((format, kbps)) = self.transcode(id) else {
            return Source { url: self.stream_url(id), live_transcode: false };
        };
        Source { url: self.call_url("stream", &self.transcode_query(id, &format, kbps)), live_transcode: true }
    }

    fn transcode_query(&self, id: &str, format: &str, kbps: i64) -> Vec<(String, String)> {
        vec![("id".into(), id.into()), ("format".into(), format.into()), ("maxBitRate".into(), kbps.to_string())]
    }

    /// The network the app is on: which transcode setting applies, from the next song on.
    pub fn set_mobile(&mut self, on: bool) {
        if on != self.mobile {
            self.mobile = on;
            self.drop_prefetch();
        }
    }

    // ------------------------------------------------------------ filling

    /// The songs to have on disk now: the next ones, then the one playing if it counts as played (a skipped one is not
    /// worth a download).
    fn wanted(&self) -> Vec<String> {
        let (mut out, n) = (vec![], self.queue.len() as i64);
        let mut i = self.idx;
        for _ in 0..self.option_num("offlineAhead") {
            i = next_index(i, n, if self.repeat == 1 { 1 } else { 0 });
            if i < 0 || i == self.idx {
                break;
            }
            out.push(self.song_id(i));
        }
        if self.scrobbled == Some(self.play) {
            out.push(self.song_id(self.idx));
        }
        out
    }

    /// One download at a time, while a song plays. Never more than fits: a song coming up is not dropped for another one
    /// coming up. shortcut: a song jumped to is streamed and then downloaded too; worth a tee of the stream if that hurts.
    pub(super) fn fill_cache(&mut self) {
        let cap = self.cache_cap();
        if self.clock_ms < self.cache_next || cap == 0 || !self.online || !self.playing || self.caching || !self.station.is_empty() || !self.valid(self.idx) {
            return;
        }
        self.cache_next = self.clock_ms + 1000.0;
        let Some(client) = self.client.clone() else { return };
        let server = self.login.server.clone();
        let (mut keep, mut bytes, mut missing) = (HashSet::new(), 0, None);
        for id in self.wanted() {
            match self.songs.find(&server, &id, &self.quality(&id), false) {
                Some(p) => {
                    bytes += p.metadata().map_or(0, |m| m.len());
                    keep.insert(p);
                }
                None => {
                    missing.get_or_insert(id);
                }
            }
        }
        let Some(id) = missing.filter(|_| bytes < cap) else { return };
        let query = match self.transcode(&id) {
            Some((format, kbps)) => self.transcode_query(&id, &format, kbps),
            None => vec![("id".into(), id.clone()), ("format".into(), RAW.into())],
        };
        let path = self.songs.path(&server, &id, &self.quality(&id));
        keep.insert(path.clone());
        self.caching = true;
        self.cache_keep = keep;
        self.jobs.push(Job::CacheSong { client, query, path });
    }

    pub fn cache_done(&mut self, ok: bool) {
        self.caching = false;
        if !ok {
            self.cache_next = self.clock_ms + 30000.0;
            return;
        }
        self.cache_next = 0.0;
        let keep = std::mem::take(&mut self.cache_keep);
        // switched off meanwhile: what there is stays, the clear button is for that
        self.trim_songs(if self.cache_cap() > 0 { self.cache_cap() } else { u64::MAX }, keep);
    }

    pub(super) fn cache_cap(&self) -> u64 {
        (self.option_num("offlineGb").max(0) as u64) << 30
    }

    /// Down to cap, but for `keep` and what the decks hold.
    pub(super) fn trim_songs(&mut self, cap: u64, mut keep: HashSet<PathBuf>) {
        for i in [self.idx, self.upcoming()].into_iter().filter(|i| self.valid(*i)) {
            let id = self.song_id(i);
            keep.extend(self.songs.find(&self.login.server, &id, &self.quality(&id), true));
        }
        self.cache_bytes = self.songs.trim(cap, &keep);
        self.changes.options = true;
    }

    pub fn clear_song_cache(&mut self) {
        self.trim_songs(0, HashSet::new());
    }

    // ------------------------------------------------------------ without the server

    /// The server can't be reached: the queue from disk stays open if any of its songs is held.
    pub(super) fn enter_offline(&mut self) {
        self.held = self.songs.held();
        self.offline = (0..self.queue.len() as i64).any(|i| self.holds(i));
        if self.offline {
            self.set_status("Çevrimdışı · inen şarkılar çalar");
        }
    }

    pub(super) fn holds(&self, i: i64) -> bool {
        self.valid(i) && SongCache::holds(&self.held, &self.login.server, &self.song_id(i))
    }

    /// i, or offline the first held song from i on (step 1) or back (-1); -1: none.
    pub(super) fn playable(&self, i: i64, step: i64) -> i64 {
        let n = self.queue.len() as i64;
        if !self.offline || i < 0 {
            return i;
        }
        let wrap = self.repeat == 1 && step > 0;
        (0..n).map(|k| i + k * step).map(|j| if wrap { j % n } else { j }).find(|j| self.holds(*j)).unwrap_or(-1)
    }

    /// The song after this one.
    pub(super) fn upcoming(&self) -> i64 {
        self.playable(next_index(self.idx, self.queue.len() as i64, self.repeat as i32), 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcode_text() {
        assert_eq!(parse_transcode("opus-128"), Some(("opus", 128)));
        for s in ["", "raw", "opus", "opus-0", "opus-x", "-128", "../x-1"] {
            assert_eq!(parse_transcode(s), None, "{s}");
        }
    }
}
