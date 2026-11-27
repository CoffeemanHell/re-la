//! What the decks report and what the 50 ms tick does: positions, crossfade, gapless, drift, reports, the queue end.
use super::deck::{DeckEvent, EndReason};
use super::state::PlayerCore;
use crate::queue::{Drift, drift_action, play_report, scrobble_due, similar_append, text};
use crate::dj::{Weights, extra_ids, pick};
use serde_json::Value;
use std::collections::HashSet;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

fn now_param(clock_ms: f64) -> (String, String) {
    ("time".into(), (clock_ms as i64).to_string())
}

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

impl PlayerCore {
    /// The tick runs the fade, drift and position while a song plays; else 1 s is enough.
    pub fn tick_ms(&self) -> u64 {
        if (self.playing && self.station.is_empty()) || self.fading { 50 } else { 1000 }
    }

    fn opt(&self, key: &str) -> bool {
        self.options.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn deck_event(&mut self, k: usize, e: DeckEvent) {
        let active = k == self.cur;
        let now = self.clock_ms;
        let d = &mut self.decks[k];
        match e {
            DeckEvent::Position(Some(v)) => {
                let v = v + d.base;
                // not a stale time-pos of the previous file, or of the place before a seek
                if !d.loading && (!d.seeking || (v - d.pos).abs() < 0.5 || now - d.seek_at > 3000.0) {
                    d.pos = v;
                    d.seeking = false;
                }
            }
            DeckEvent::Position(None) => d.pos = -1.0,
            DeckEvent::Duration(v) if !d.live_transcode => d.dur = v,
            DeckEvent::Duration(_) => {} // what is left of it, or a guess: the song's own is right
            DeckEvent::Bitrate(v) => d.bitrate = v.unwrap_or(0.0),
            DeckEvent::Prop(name, Some(v)) => {
                d.props.insert(name, v);
            }
            DeckEvent::Prop(name, None) => {
                d.props.remove(&name);
            }
            DeckEvent::StartFile => {
                if active && d.appended {
                    // gapless transition inside this deck
                    d.appended = false;
                    d.loading = true;
                    d.seeking = false;
                    d.pos = -1.0;
                    d.base = 0.0;
                    d.live_transcode = d.appended_live;
                    if d.live_transcode {
                        d.dur = 0.0; // the last song's
                    }
                    let (from_dur, i) = (d.appended_from_dur, d.appended_idx);
                    d.out.drop_first();
                    self.anchor += from_dur * 1000.0;
                    let turn = self.heard(true);
                    self.idx = i;
                    self.play += 1;
                    if self.valid(i) {
                        self.decks[k].id = self.song_id(i);
                    }
                    if turn {
                        self.drop_dj_picks();
                    }
                    self.state_changed();
                    self.timer(3000, |c| c.publish());
                }
            }
            DeckEvent::FileLoaded => {
                d.loading = false;
                let pending = d.pending_seek;
                d.pending_seek = 0.0;
                let there = if d.live_transcode { (pending - d.base).abs() < 2.0 } else { pending <= 0.05 };
                if !there {
                    self.seek_deck(k, pending); // the timeline goes straight there, no 0:00 in between
                }
                self.failed = 0;
            }
            DeckEvent::EndFile { reason, error } => self.end_file(k, active, reason, &error),
        }
        self.notify_position();
    }

    fn end_file(&mut self, k: usize, active: bool, reason: EndReason, error: &str) {
        if !active {
            self.decks[k].failed |= reason == EndReason::Error; // the crossfade preload: cross_to tries it once more
            return;
        }
        if self.decks[k].appended {
            return;
        }
        if !self.station.is_empty() {
            // a stream never advances the queue
            if reason == EndReason::Error {
                self.set_status(crate::i18n::tr("Yayın açılamadı", "Could not open the stream"));
            }
            if matches!(reason, EndReason::Error | EndReason::Eof) {
                self.stop_station();
            }
            return;
        }
        match reason {
            EndReason::Error => {
                if !self.online {
                    // opened from disk before connecting: restore_queue loads it again once connected
                    self.decks[k].id.clear();
                    let n = self.playable(self.idx + 1, 1);
                    if self.offline && self.playing && self.valid(n) {
                        self.load(n, 0.0);
                    }
                    return;
                }
                let title = if self.valid(self.idx) { text(&self.queue[self.idx as usize]["title"]) } else { String::new() };
                self.set_status(format!("Can't play {title}: {error}"));
                self.failed += 1;
                if self.failed >= 3 {
                    // three in a row (server down): stop here instead of running through the queue
                    self.failed = 0;
                    self.held = self.songs.held();
                    if let Some(i) = (self.idx + 1..self.queue.len() as i64).find(|i| self.holds(*i)) {
                        return self.load(i, 0.0); // the network went away: what is on disk still plays
                    }
                    self.decks[k].id.clear(); // play opens this song again (toggle)
                    self.set_playing(false);
                    return;
                }
                self.load(self.idx + 1, 0.0);
            }
            EndReason::Eof => {
                if self.upcoming() >= 0 {
                    self.advance();
                } else {
                    self.heard(true); // nothing comes after it to make way for
                    self.decks[k].id.clear(); // the deck is empty: play opens the song again (toggle)
                    self.set_playing(false);
                }
            }
            _ => {}
        }
    }

    /// positionChanged only when something it covers changed (paused: nothing to redraw).
    pub(super) fn notify_position(&mut self) {
        if !self.station.is_empty() || !self.valid(self.idx) {
            return; // a stream has no timeline
        }
        let now = (self.position(), self.duration(), self.bitrate(), self.signal_path());
        if now != self.pos_sent {
            self.pos_sent = now;
            self.changes.position = true;
        }
    }

    /// The crossfade out of a song: never more than half of it.
    pub(super) fn fade_of(&self, dur: f64) -> f64 {
        (self.xfade as f64).min(dur / 2.0).max(0.0)
    }

    pub(super) fn cross_to(&mut self, i: i64) {
        self.cur = 1 - self.cur;
        let k = self.cur;
        self.idx = i;
        self.play += 1;
        let id = self.song_id(i);
        if self.decks[k].id != id || self.decks[k].failed {
            self.load_deck(k, &id, 0.0, true); // normally preloaded by tick
        }
        self.decks[k].out.set_volume(0.0);
        self.set_speed(k, 1.0);
        self.decks[k].out.set_paused(false);
        self.fading = true;
        self.fade_start = self.clock_ms;
        self.state_changed();
    }

    /// Natural advance: the next track's 0:00 is scheduled on the shared timeline, so peers stay locked regardless of
    /// how long each one took to buffer.
    pub(super) fn advance(&mut self) {
        let turn = self.heard(true);
        self.fade_s = self.fade_of(self.duration());
        self.anchor += (self.duration() - self.fade_s) * 1000.0;
        let n = self.upcoming();
        self.cross_to(n);
        if turn {
            self.drop_dj_picks();
        }
        self.timer(((self.xfade + 3) * 1000) as u64, |c| c.publish());
    }

    pub fn tick(&mut self) {
        self.report_playback();
        if !self.station.is_empty() {
            // live stream: no timeline, crossfade or prefetch
            let info = self.stream_info();
            if info != self.stream_info_sent {
                self.stream_info_sent = info;
                self.changes.stream_info = true;
            }
            if self.online {
                // the station's track counts like a song's: now playing, then the play, each for the signed-in user
                let step = self.radio_plays.update(&self.radio_id.clone(), self.radio_pos_ms, self.radio_dur);
                let id = self.radio_id.clone();
                if step.now_playing && self.opt("nowPlaying") && !self.playback_report {
                    self.api("scrobble", q(&[("id", &id), ("submission", "false")]), true, |_, _| {});
                }
                if step.count && self.opt("scrobble") {
                    let mut p = q(&[("id", &id), ("submission", "true")]);
                    p.push(now_param(self.clock_ms));
                    self.api("scrobble", p, true, |_, _| {});
                }
            }
            return;
        }

        if self.fading {
            let (inn, out, v) = (self.cur, 1 - self.cur, self.volume);
            let k = if self.fade_s > 0.0 { ((self.clock_ms - self.fade_start) / (self.fade_s * 1000.0)).min(1.0) } else { 1.0 };
            // equal-power curve; mpv volume is cubic (gain = (vol/100)^3), hence cbrt
            self.decks[out].out.set_volume(v * (k * PI / 2.0).cos().max(0.0).cbrt());
            self.decks[inn].out.set_volume(v * (k * PI / 2.0).sin().cbrt());
            if k >= 1.0 {
                self.fading = false;
                self.decks[out].out.stop();
                self.decks[out].id.clear();
            }
        }

        if !self.valid(self.idx) {
            return;
        }
        self.notify_position();
        self.fill_cache();

        // The queue never runs dry, also while skipping or paused: DJ keeps radioBatch songs ahead, without DJ at least 2.
        let ahead = self.queue.len() as i64 - self.idx - 1;
        let want = if self.dj { self.options.get("radioBatch").and_then(Value::as_i64).unwrap_or(10) } else { 2 };
        if ahead < want
            && self.online
            && !self.extending
            && self.opt("autoContinue")
            && self.repeat != 1
            && !self.following()
            && self.clock_ms >= self.extend_retry
        {
            self.extend((want - ahead) as i32);
        }
        if self.held_skip.as_ref().is_some_and(|h| self.clock_ms - h.2 > 90_000.0) {
            self.dj_flush_held(); // no search came of it (paused after one skip)
        }
        self.index_refresh();

        let k = self.cur;
        let d = &self.decks[k];
        if !self.playing || d.loading || d.seeking || d.pos < 0.0 {
            return;
        }
        let (pos, dur) = (d.pos, self.duration());
        let left = dur - pos;

        // Scrobble: "now playing" when a song starts, the play once it counts. Each device for its own user.
        if self.online && self.now_sent != Some(self.play) {
            self.now_sent = Some(self.play); // marked even when off, so switching on mid-song sends nothing late
            if self.opt("nowPlaying") && !self.playback_report {
                let id = self.song_id(self.idx);
                self.api("scrobble", q(&[("id", &id), ("submission", "false")]), true, |_, _| {});
            }
        }
        if self.online && self.scrobbled != Some(self.play) && scrobble_due(pos, dur) {
            self.scrobbled = Some(self.play);
            if self.opt("scrobble") {
                let id = self.song_id(self.idx);
                let mut p = q(&[("id", &id), ("submission", "true")]);
                p.push(now_param(self.clock_ms));
                self.api("scrobble", p, true, |_, _| {});
            }
        }

        let n = self.upcoming();
        if n >= 0 && !self.fading && left < self.xfade as f64 + 15.0 {
            let next = self.song_id(n);
            if self.xfade == 0 {
                // gapless: queue next inside the same deck
                if !self.decks[k].appended {
                    let src = self.source(&next);
                    let d = &mut self.decks[k];
                    d.appended = true;
                    d.appended_live = src.live_transcode;
                    d.appended_idx = n;
                    d.appended_from_dur = dur;
                    d.out.append(&src.url);
                }
            } else {
                // crossfade: preload next on the other deck, paused
                if self.decks[1 - k].id != next {
                    self.load_deck(1 - k, &next, 0.0, true);
                }
                if left <= self.fade_of(dur) {
                    return self.advance();
                }
            }
        }

        // Drift vs the shared timeline: small = nudge speed (scaletempo keeps pitch), large = seek. Alone, the timeline
        // follows us. shortcut: the speed nudge is not bit-perfect while active (drift > 150 ms); it snaps back under 30 ms.
        let expected = (self.server_now() - self.anchor) / 1000.0;
        let shared = self.following() || self.sync.has_listeners();
        let off = pos - expected;
        // shortcut: a live transcode seeks by a new stream, in whole seconds and after the server's start: up to 5 s is
        // made up by speed, or it would seek forever
        let nudge = self.decks[k].live_transcode && off.abs() < 5.0;
        match drift_action(off, shared) {
            Drift::Seek if nudge => self.set_speed(k, if off > 0.0 { 0.98 } else { 1.02 }),
            Drift::Seek => self.seek_deck(k, expected), // the drift check waits until it landed (seeking)
            Drift::Slow => self.set_speed(k, 0.98),
            Drift::Fast => self.set_speed(k, 1.02),
            Drift::Steady => self.set_speed(k, 1.0),
            Drift::Reanchor => {
                // sync_state takes the anchor from the deck; the relay hears it, so a late joiner starts in step
                self.set_speed(k, 1.0);
                self.publish();
            }
            Drift::Keep => {}
        }
    }

    /// OpenSubsonic playbackReport: song, state and position to the server on each change, so Navidrome's now playing
    /// (and its plugins, e.g. Discord) sees pauses and seeks. Plays are still counted by our own scrobble.
    fn report_playback(&mut self) {
        if !self.online || !self.playback_report || !self.opt("nowPlaying") {
            return;
        }
        let d = &self.decks[self.cur];
        let song = self.station.is_empty() && self.valid(self.idx);
        let radio = !self.station.is_empty() && !self.radio_id.is_empty() && self.radio_pos_ms > 0.0; // a library track the listener reached
        if song && (d.loading || d.pos < 0.0) {
            return; // wait for the real position
        }
        // The station moved to a new track but the listener still hears the old one (stream buffer): leave the server as it is.
        if !self.station.is_empty() && !self.radio_id.is_empty() && !radio {
            return;
        }
        let id = if song { self.song_id(self.idx) } else if radio { self.radio_id.clone() } else { self.report.sent.id.clone() };
        let state = if !song && !radio { "stopped" } else if self.playing { "playing" } else { "paused" };
        if id.is_empty() {
            return;
        }
        let pos = if song { d.pos * 1000.0 } else if radio { self.radio_pos_ms } else { 0.0 };
        let clock = self.clock_ms;
        if !self.report.due(&play_report(&id, state, pos, clock), clock, 1000.0) {
            return;
        }
        let p = q(&[
            ("mediaId", &id),
            ("mediaType", "song"),
            ("positionMs", &(pos.round() as i64).to_string()),
            ("state", state),
            ("ignoreScrobble", "true"),
        ]);
        self.api("reportPlayback", p, true, |_, _| {});
    }

    /// Queue end. DJ off: Navidrome's getSimilarSongs as it comes (random songs when it has nothing new). DJ on:
    /// candidates from similar songs + same genre + a wide random pool + the index's best fits (getSong), picked by
    /// dj::pick with the weights from settings; a listener looking for something else gets more new songs. Only the root extends; followers get the songs with its
    /// next state.
    pub(super) fn extend(&mut self, need: i32) {
        let seed = self.queue[self.idx as usize].clone();
        let seed_id = text(&seed["id"]);
        let generation = self.generation;
        self.extend_seq += 1;
        let seq = self.extend_seq;
        self.extending = true;
        fn songs_of(r: &Value, key: &str) -> Vec<Value> {
            r[key]["song"].as_array().cloned().unwrap_or_default()
        }
        // Appends what is still new; nothing new (network down, tiny library) tries again in 15 s.
        let finish = move |c: &mut PlayerCore, songs: Vec<Value>| {
            if seq != c.extend_seq {
                return; // superseded: a skip streak wants looser picks
            }
            c.extending = false;
            let seed_there = c.queue.iter().any(|s| text(&s["id"]) == seed_id);
            if generation != c.generation || !seed_there || !c.valid(c.idx) {
                return; // logged out or a new queue was played meanwhile
            }
            let mut add = similar_append(&c.queue, &songs);
            if c.dj {
                let batch = c.options.get("radioBatch").and_then(Value::as_i64).unwrap_or(10);
                let room = (batch - (c.queue.len() as i64 - c.idx - 1)).max(0) as usize; // skips while waiting
                add.truncate(room);
            }
            if add.is_empty() {
                c.extend_retry = c.clock_ms + 15000.0;
                return;
            }
            Arc::make_mut(&mut c.queue).extend(add);
            c.queue_edited();
        };
        let seed_id = text(&seed["id"]);
        if !self.dj {
            self.api("getSimilarSongs", q(&[("id", &seed_id)]), true, move |c, r| {
                let songs = r.map(|r| songs_of(&r, "similarSongs")).unwrap_or_default();
                if !similar_append(&c.queue, &songs).is_empty() {
                    return finish(c, songs);
                }
                c.api("getRandomSongs", q(&[("size", "10")]), true, move |c, r| {
                    finish(c, r.map(|r| songs_of(&r, "randomSongs")).unwrap_or_default())
                });
            });
            return;
        }
        struct Fetch {
            pool: Vec<Value>,
            similar: HashSet<String>,
            pending: usize,
        }
        let opt = |k: &str, def: i64| self.options.get(k).and_then(Value::as_i64).unwrap_or(def) as i32;
        let w = Weights { language: opt("radioLanguage", 80), genre: opt("radioGenre", 50), explore: opt("radioExplore", 20), batch: need };
        let (now, day) = (self.clock_ms, crate::dj::memory::day(self.clock_ms));
        let searching = self.dj_brain().session.searching(now);
        let at = ((self.idx + 1).max(0) as usize).min(self.queue.len());
        let history = self.queue[..at].to_vec();
        let extra = extra_ids(&self.brain.as_ref().unwrap().1, &history, &w, searching, day, &mut self.rng);
        let fetch = Arc::new(Mutex::new(Fetch { pool: vec![], similar: HashSet::new(), pending: 0 }));
        let finish = Arc::new(Mutex::new(Some(finish)));
        let mut calls = vec![("getSimilarSongs", q(&[("id", &seed_id), ("count", "50")]), "similarSongs", true)];
        let genre = text(&seed["genre"]);
        if !genre.is_empty() {
            calls.push(("getRandomSongs", q(&[("size", "100"), ("genre", &genre)]), "randomSongs", false));
        }
        if w.language > 0 || searching {
            // wide pool: the language quota can be met, and a searching listener gets variety
            calls.push(("getRandomSongs", q(&[("size", "400")]), "randomSongs", false));
        }
        for id in &extra {
            calls.push(("getSong", q(&[("id", id)]), "song", false));
        }
        fetch.lock().unwrap().pending = calls.len();
        for (method, params, key, is_similar) in calls {
            let (fetch, finish) = (fetch.clone(), finish.clone());
            self.api(method, params, true, move |c, r| {
                let mut f = fetch.lock().unwrap();
                let got = match r {
                    Some(r) if key == "song" && r["song"].is_object() => vec![r["song"].clone()],
                    Some(_) if key == "song" => vec![],
                    Some(r) => songs_of(&r, key),
                    None => vec![],
                };
                for s in got {
                    if is_similar {
                        f.similar.insert(text(&s["id"]));
                    }
                    f.pool.push(s);
                }
                f.pending -= 1;
                if f.pending > 0 {
                    return;
                }
                if generation != c.generation || seq != c.extend_seq {
                    return; // another account or a new direction: these songs are not for this index
                }
                let at = ((c.idx + 1).max(0) as usize).min(c.queue.len());
                let pool = similar_append(&c.queue, &f.pool);
                c.dj_brain();
                let brain = &mut c.brain.as_mut().unwrap().1;
                let mut picked = pick(brain, &pool, &c.queue[..at], &c.queue[at..], &f.similar, &w, searching, day, &mut c.rng);
                for s in &mut picked {
                    s["dj"] = true.into(); // so a skip streak can swap the DJ's own picks, never the listener's
                }
                drop(f);
                if let Some(finish) = finish.lock().unwrap().take() {
                    finish(c, picked);
                }
            });
        }
    }

    /// Keeps the DJ's library index fresh: the whole library (search3, 500 a page) when it is empty or a week old and
    /// the network is not metered; otherwise the newest albums down to the first one it knows. Every 6 hours.
    pub(super) fn index_refresh(&mut self) {
        if !self.dj || !self.online || self.index_busy || self.clock_ms < self.index_next || self.following() {
            return;
        }
        let day = crate::dj::memory::day(self.clock_ms);
        let mobile = self.mobile;
        let ix = &self.dj_brain().index;
        // songs from DJ pools land in the index too: only a whole walk makes it the library's
        let (never, stale) = (ix.full_day == 0, ix.full_day.saturating_add(7) <= day);
        if never && mobile {
            self.index_next = self.clock_ms + 15.0 * 60_000.0; // until Wi-Fi
            return;
        }
        self.index_busy = true;
        if (never || stale) && !mobile {
            self.index_page(0, HashSet::new());
        } else {
            self.index_newest(0);
        }
    }

    fn index_done(&mut self, ok: bool) {
        self.index_busy = false;
        self.index_next = self.clock_ms + if ok { 6.0 * 3_600_000.0 } else { 15.0 * 60_000.0 };
        let day = crate::dj::memory::day(self.clock_ms);
        let b = self.dj_brain();
        let _ = b.index.save();
        b.rebuild_genres(day);
    }

    fn index_page(&mut self, offset: usize, mut ids: HashSet<String>) {
        const PAGE: usize = 500;
        let generation = self.generation;
        let off = offset.to_string();
        let query = q(&[("query", ""), ("songCount", "500"), ("songOffset", &off), ("artistCount", "0"), ("albumCount", "0")]);
        self.api("search3", query, true, move |c, r| {
            if generation != c.generation {
                return c.index_busy = false;
            }
            let Some(r) = r else { return c.index_done(false) }; // half a walk: nothing is dropped
            let day = crate::dj::memory::day(c.clock_ms);
            let songs = r["searchResult3"]["song"].as_array().cloned().unwrap_or_default();
            let ix = &mut c.dj_brain().index;
            for s in &songs {
                ix.upsert(s, day);
                ids.insert(text(&s["id"]));
            }
            if songs.len() == PAGE {
                if c.mobile {
                    return c.index_done(false); // on mobile data now: the rest waits for Wi-Fi
                }
                return c.index_page(offset + PAGE, ids);
            }
            ix.retain(&ids);
            ix.full_day = day;
            c.index_done(true);
        });
    }

    /// Newest albums first, a page of 50 at a time, until one the index already has (20 pages at most).
    fn index_newest(&mut self, page: usize) {
        const SIZE: usize = 50;
        let generation = self.generation;
        let off = (page * SIZE).to_string();
        self.api("getAlbumList2", q(&[("type", "newest"), ("size", "50"), ("offset", &off)]), true, move |c, r| {
            if generation != c.generation {
                return c.index_busy = false;
            }
            let Some(r) = r else { return c.index_done(false) };
            let albums = r["albumList2"]["album"].as_array().cloned().unwrap_or_default();
            let ix = &c.dj_brain().index;
            let mut todo = vec![];
            let mut known = false;
            for a in &albums {
                let id = text(&a["id"]);
                if ix.has_album(&id) {
                    known = true;
                    break;
                }
                todo.push(id);
            }
            let more = !known && albums.len() == SIZE && page + 1 < 20;
            c.index_albums(todo, more.then_some(page + 1));
        });
    }

    fn index_albums(&mut self, mut todo: Vec<String>, next_page: Option<usize>) {
        if todo.is_empty() {
            return match next_page {
                Some(p) => self.index_newest(p),
                None => self.index_done(true),
            };
        }
        let id = todo.remove(0);
        let generation = self.generation;
        self.api("getAlbum", q(&[("id", &id)]), true, move |c, r| {
            if generation != c.generation {
                return c.index_busy = false;
            }
            let day = crate::dj::memory::day(c.clock_ms);
            let ix = &mut c.dj_brain().index;
            for s in r.as_ref().and_then(|r| r["album"]["song"].as_array()).into_iter().flatten() {
                ix.upsert(s, day);
            }
            c.index_albums(todo, next_page);
        });
    }
}
