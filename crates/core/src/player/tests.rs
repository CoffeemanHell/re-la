//! The player logic against a fake deck, a fake relay and canned Subsonic answers.
use super::*;
use crate::sync::SyncEvent;
use md5::Digest;
use crate::settings::{JsonSettings, Settings};
use crate::subsonic::Failure;
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<String>>>;

/// "id" of a stream URL, else the URL (stations).
fn id_of_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "id").map(|(_, v)| v.to_string()))
        .unwrap_or_else(|| url.to_string())
}

struct FakeDeck {
    i: usize,
    log: Log,
}

impl FakeDeck {
    fn push(&self, s: String) {
        self.log.lock().unwrap().push(format!("{} {s}", self.i));
    }
}

impl Deck for FakeDeck {
    fn load(&mut self, url: &str, paused: bool) {
        let from = url.split_once("&timeOffset=").map_or(String::new(), |(_, s)| format!(" from {s}"));
        self.push(format!("load {}{from} paused={paused}", id_of_url(url)));
    }
    fn append(&mut self, url: &str) {
        self.push(format!("append {}", id_of_url(url)));
    }
    fn clear_appended(&mut self) {
        self.push("clear".into());
    }
    fn drop_first(&mut self) {
        self.push("drop_first".into());
    }
    fn set_paused(&mut self, paused: bool) {
        self.push(format!("pause {paused}"));
    }
    fn seek(&mut self, s: f64) {
        self.push(format!("seek {s:.1}"));
    }
    fn set_volume(&mut self, v: f64) {
        self.push(format!("volume {v:.1}"));
    }
    fn set_speed(&mut self, s: f64) {
        self.push(format!("speed {s}"));
    }
    fn stop(&mut self) {
        self.push("stop".into());
    }
    fn set_exclusive(&mut self, on: bool) {
        self.push(format!("exclusive {on}"));
    }
    fn set_audio_device(&mut self, name: &str) {
        self.push(format!("device {name}"));
    }
}

#[derive(Default)]
struct SyncState {
    following: String,
    listeners: bool,
    now: f64,
    published: Vec<Map<String, Value>>,
    sent: Vec<(String, Value)>,
    started: Vec<String>,
    stopped: Vec<bool>,
}

#[derive(Clone, Default)]
struct FakeSync(Arc<Mutex<SyncState>>);

impl SyncPort for FakeSync {
    fn following(&self) -> String {
        self.0.lock().unwrap().following.clone()
    }
    fn has_listeners(&self) -> bool {
        self.0.lock().unwrap().listeners
    }
    fn now(&self) -> f64 {
        self.0.lock().unwrap().now
    }
    fn publish(&self, state: Map<String, Value>) {
        self.0.lock().unwrap().published.push(state);
    }
    fn send(&self, op: &str, args: Value) {
        self.0.lock().unwrap().sent.push((op.into(), args));
    }
    fn start(&self, url: String, _user: String, _token: String, _salt: String) {
        self.0.lock().unwrap().started.push(url);
    }
    fn stop(&self, bye: bool) {
        self.0.lock().unwrap().stopped.push(bye);
    }
    fn demo(&self) {}
}

#[derive(Clone, Default)]
struct FakeStation(Arc<Mutex<Vec<String>>>);

impl StationPort for FakeStation {
    fn open(&self, home: String, stream_url: String) {
        self.0.lock().unwrap().push(format!("open {home} {stream_url}"));
    }
    fn close(&self) {
        self.0.lock().unwrap().push("close".into());
    }
}

struct T {
    c: PlayerCore,
    log: Log,
    sync: FakeSync,
    station: FakeStation,
    settings: Arc<dyn Settings>,
    dir: PathBuf,
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("re-la-player-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

impl T {
    /// A core with a saved login (not connected yet).
    fn new(name: &str) -> T {
        let dir = tmp(name);
        let settings: Arc<dyn Settings> = Arc::new(JsonSettings::open(dir.join("settings.json")));
        for (k, v) in [("url", "https://music.example.com"), ("user", "ana"), ("salt", "s4lt"), ("token", "t0ken")] {
            settings.set(k, json!(v));
        }
        T::with(name, dir, settings)
    }

    fn with(_name: &str, dir: PathBuf, settings: Arc<dyn Settings>) -> T {
        let log: Log = Arc::default();
        let sync = FakeSync::default();
        sync.0.lock().unwrap().now = 1_000_000.0;
        let station = FakeStation::default();
        let decks: [Box<dyn Deck>; 2] = [Box::new(FakeDeck { i: 0, log: log.clone() }), Box::new(FakeDeck { i: 1, log: log.clone() })];
        let mut c = PlayerCore::new(settings.clone(), dir.clone(), decks, Box::new(sync.clone()), Box::new(station.clone()));
        c.clock_ms = 1_000_000.0;
        T { c, log, sync, station, settings, dir }
    }

    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
    fn clear(&self) {
        self.log.lock().unwrap().clear();
    }
    fn has(&self, line: &str) -> bool {
        self.log().iter().any(|l| l == line)
    }
    fn ids(&self) -> Vec<String> {
        self.c.snapshot().queue.iter().map(|s| s["id"].as_str().unwrap().to_string()).collect()
    }
    fn current(&self) -> String {
        self.c.snapshot().current["id"].as_str().unwrap_or("").to_string()
    }
    fn published(&self) -> Vec<Map<String, Value>> {
        self.sync.0.lock().unwrap().published.clone()
    }
    fn sent(&self) -> Vec<(String, Value)> {
        self.sync.0.lock().unwrap().sent.clone()
    }
    fn follow(&self, root: &str) {
        self.sync.0.lock().unwrap().following = root.into();
    }
    fn set_now(&mut self, ms: f64) {
        self.c.clock_ms = ms;
        self.sync.0.lock().unwrap().now = ms;
    }
    fn advance(&mut self, ms: f64) {
        let now = self.c.clock_ms + ms;
        self.set_now(now);
    }
    /// The active deck at pos, loaded and settled.
    fn at(&mut self, pos: f64) {
        let d = &mut self.c.decks[self.c.cur];
        d.loading = false;
        d.seeking = false;
        d.pos = pos;
    }

    /// Runs the queued jobs: Api through answer (None: the call fails), Timer only when timers is set. Loops until
    /// nothing is left, so callbacks that queue more calls are answered too.
    fn run(&mut self, timers: bool, answer: impl Fn(&str, &[(String, String)]) -> Option<Value>) -> Vec<String> {
        let mut calls = vec![];
        for _ in 0..50 {
            let jobs = self.c.take_jobs();
            if jobs.is_empty() {
                break;
            }
            for j in jobs {
                match j {
                    Job::Api { method, query, then, .. } => {
                        calls.push(format!("{method}?{}", crate::queue::encode_query(&query)));
                        let r = match answer(&method, &query) {
                            Some(v) if v["status"] == "ok" => Ok(v),
                            Some(v) => Err(Failure {
                                code: v["error"]["code"].as_i64().unwrap_or(0),
                                message: v["error"]["message"].as_str().unwrap_or("").into(),
                                http: 200,
                                net_error: String::new(),
                            }),
                            None => Err(Failure {
                                code: crate::queue::NO_SERVER,
                                message: String::new(),
                                http: 0,
                                net_error: "Connection refused".into(),
                            }),
                        };
                        then(&mut self.c, r);
                    }
                    Job::Timer { ms, then } if timers => {
                        calls.push(format!("timer {ms}"));
                        then(&mut self.c);
                    }
                    Job::Timer { ms, .. } => calls.push(format!("timer {ms} (not run)")),
                    Job::Download { id, .. } => calls.push(format!("download {id}")),
                    Job::CacheSong { query, path, .. } => {
                        calls.push(format!("cache {}", crate::queue::encode_query(&query)));
                        std::fs::write(&path, b"song").unwrap();
                        self.c.cache_done(true);
                    }
                    Job::CancelDownload => calls.push("cancel download".into()),
                }
            }
        }
        calls
    }
}

fn ok(body: Value) -> Option<Value> {
    let mut o = body;
    o["status"] = "ok".into();
    Some(o)
}

fn song(id: &str) -> Value {
    json!({"id": id, "title": format!("T {id}"), "artist": "A", "duration": 200, "genre": "Pop"})
}

fn songs(n: usize) -> Vec<Value> {
    (0..n).map(|i| song(&format!("s{i}"))).collect()
}

fn ids_of(state: &Map<String, Value>) -> Vec<String> {
    state["queue"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect()
}

// ---------------------------------------------------------------- Task 1: queue actions

#[test]
fn play_replaces_queue_and_loads_stream() {
    let mut t = T::new("play");
    t.c.play(songs(3), 1);
    assert_eq!(t.c.snapshot().index, 1);
    assert!(t.c.snapshot().playing);
    assert!(t.has("0 load s1 paused=false"), "{:?}", t.log());
    let p = t.published().last().cloned().unwrap();
    assert_eq!(ids_of(&p), ["s0", "s1", "s2"]);
    assert_eq!(p["index"], 1);
    assert_eq!(p["playing"], true);
    // peers get only what they need to show and play a song
    let mut extra = song("s9");
    extra["path"] = "/music/secret.flac".into();
    t.c.play(vec![extra], 0);
    assert!(t.published().last().unwrap()["queue"][0].get("path").is_none());
}

#[test]
fn stream_url_is_raw_with_login() {
    let t = T::new("stream");
    let u = t.c.stream_url("s1");
    assert!(u.starts_with("https://music.example.com/rest/stream?id=s1&format=raw&u=ana&t=t0ken&s=s4lt"), "{u}");
}

#[test]
fn following_sends_commands_instead() {
    let mut t = T::new("follow");
    t.c.play(songs(4), 0);
    let before = t.published().len();
    t.follow("root");
    t.clear();
    t.c.play(songs(2), 1);
    t.c.add(vec![song("x")], 1);
    t.c.toggle();
    t.c.next();
    t.c.prev();
    t.c.seek(5.0);
    t.c.jump(2);
    t.c.remove(0);
    t.c.move_song(0, 2);
    t.c.clear_upcoming();
    t.c.set_shuffle(true);
    t.c.set_repeat(1);
    t.c.play_station("st");
    let ops: Vec<String> = t.sent().iter().map(|(o, _)| o.clone()).collect();
    assert_eq!(ops, ["play", "add", "toggle", "next", "prev", "seek", "jump", "remove", "move", "clearUpcoming", "shuffle", "repeat", "station"]);
    let sent = t.sent();
    assert_eq!(sent[0].1["start"], 1);
    assert_eq!(sent[1].1["where"], 1);
    assert_eq!(sent[5].1, json!({"s": 5.0}));
    assert_eq!(sent[8].1, json!({"from": 0, "to": 2}));
    assert_eq!(t.ids(), ["s0", "s1", "s2", "s3"]); // the root's queue changes, not ours
    assert_eq!(t.published().len(), before);
    assert!(t.log().is_empty(), "{:?}", t.log());
}

#[test]
fn shuffle_keeps_current_first_and_off_restores() {
    let mut t = T::new("shuffle");
    t.c.play(songs(12), 3);
    t.c.set_shuffle(true);
    let ids = t.ids();
    assert_eq!(ids[..4], ["s0", "s1", "s2", "s3"]);
    assert_ne!(ids[4..], songs(12)[4..].iter().map(|s| s["id"].as_str().unwrap().to_string()).collect::<Vec<_>>());
    t.c.set_shuffle(false);
    assert_eq!(t.ids(), (0..12).map(|i| format!("s{i}")).collect::<Vec<_>>());
    // shuffle on, then play: the picked song first
    t.c.set_shuffle(true);
    t.c.play(songs(12), 5);
    assert_eq!(t.ids()[0], "s5");
    assert_eq!(t.c.snapshot().index, 0);
    assert!(t.settings.get_bool("shuffle", false));
}

#[test]
fn remove_current_reloads_same_index() {
    let mut t = T::new("remove");
    t.c.play(songs(3), 1);
    t.clear();
    t.c.remove(1);
    assert_eq!(t.ids(), ["s0", "s2"]);
    assert_eq!(t.current(), "s2");
    assert!(t.has("0 load s2 paused=false"), "{:?}", t.log());
    t.c.remove(0);
    assert_eq!(t.c.snapshot().index, 0);
    assert_eq!(t.current(), "s2");
}

#[test]
fn move_tracks_index() {
    let mut t = T::new("move");
    t.c.play(songs(5), 2);
    t.c.move_song(2, 4);
    assert_eq!((t.c.snapshot().index, t.current()), (4, "s2".into()));
    t.c.move_song(0, 4);
    assert_eq!((t.c.snapshot().index, t.current()), (3, "s2".into()));
    t.c.move_song(4, 0);
    assert_eq!((t.c.snapshot().index, t.current()), (4, "s2".into()));
    t.c.move_song(0, 1);
    assert_eq!((t.c.snapshot().index, t.current()), (4, "s2".into()));
}

#[test]
fn next_counts_skips_and_dj_drop() {
    let mut t = T::new("skips");
    t.c.set_dj(true);
    let dj = |id: &str| {
        let mut s = song(id);
        s["dj"] = true.into();
        s
    };
    t.c.play(vec![song("s0"), song("s1"), dj("d0"), dj("d1"), song("u0"), dj("d2")], 0);
    t.c.next();
    assert_eq!(t.ids(), ["s0", "s1", "d0", "d1", "u0", "d2"]);
    t.c.next(); // second skip in a row: the DJ's upcoming picks go, the listener's stay
    assert_eq!(t.current(), "d0");
    assert_eq!(t.ids(), ["s0", "s1", "d0", "u0"]);
}

#[test]
fn prev_restarts_after_3s() {
    let mut t = T::new("prev");
    t.c.play(songs(3), 1);
    t.at(10.0);
    t.c.prev();
    assert_eq!(t.current(), "s1");
    t.at(2.0);
    t.c.prev();
    assert_eq!(t.current(), "s0");
}

#[test]
fn queue_changed_only_when_songs_differ() {
    let mut t = T::new("qchanged");
    t.c.play(songs(3), 0);
    let ch = t.c.take_changes();
    assert!(ch.state && ch.queue);
    t.c.toggle();
    let ch = t.c.take_changes();
    assert!(ch.state && !ch.queue);
    t.c.add(vec![song("x")], 2);
    assert!(t.c.take_changes().queue);
}

#[test]
fn crossfade_refused_while_following_and_clamped() {
    let mut t = T::new("xfade");
    t.c.set_crossfade(20);
    assert_eq!(t.c.snapshot().crossfade, 12);
    assert_eq!(t.settings.get_i64("crossfade", 0), 12);
    t.follow("root");
    t.c.set_crossfade(3);
    assert_eq!(t.c.snapshot().crossfade, 12);
}

#[test]
fn exclusive_forces_crossfade_off() {
    let mut t = T::new("exclusive");
    t.c.set_exclusive(true);
    let s = t.c.snapshot();
    assert!(s.exclusive);
    assert_eq!(s.crossfade, 0);
    assert!(t.has("0 exclusive true") && t.has("1 exclusive true") && t.has("1 stop"), "{:?}", t.log());
    t.c.set_crossfade(6);
    assert_eq!(t.c.snapshot().crossfade, 0);
}

#[test]
fn volume_is_clamped_and_saved() {
    let mut t = T::new("volume");
    t.c.set_volume(140.0);
    assert_eq!(t.c.snapshot().volume, 100.0);
    t.c.set_volume(-3.0);
    assert_eq!(t.c.snapshot().volume, 0.0);
    assert!(t.has("0 volume 0.0"));
    assert!(t.c.take_changes().volume);
    assert_eq!(t.settings.get_f64("volume", 50.0), 0.0);
}

// ---------------------------------------------------------------- Task 2: deck events, crossfade, gapless, drift

fn ev(t: &mut T, k: usize, e: DeckEvent) {
    t.c.deck_event(k, e);
}

#[test]
fn stale_time_pos_ignored_while_loading_and_seeking() {
    let mut t = T::new("stale");
    t.c.play(songs(2), 0);
    ev(&mut t, 0, DeckEvent::Position(Some(77.0))); // the previous file's time-pos
    assert_eq!(t.c.snapshot().position, 0.0);
    ev(&mut t, 0, DeckEvent::FileLoaded);
    ev(&mut t, 0, DeckEvent::Position(Some(1.0)));
    assert_eq!(t.c.snapshot().position, 1.0);
    t.c.seek(100.0);
    ev(&mut t, 0, DeckEvent::Position(Some(1.2))); // from before the seek
    assert_eq!(t.c.snapshot().position, 100.0);
    ev(&mut t, 0, DeckEvent::Position(Some(100.1)));
    assert_eq!(t.c.snapshot().position, 100.1);
    t.c.seek(150.0);
    t.advance(3001.0); // a seek that never lands: after 3 s the deck is believed again
    ev(&mut t, 0, DeckEvent::Position(Some(20.0)));
    assert_eq!(t.c.snapshot().position, 20.0);
}

#[test]
fn file_loaded_seeks_pending() {
    let mut t = T::new("pending");
    t.c.queue = Arc::new(songs(2));
    t.c.load(0, 42.0);
    assert_eq!(t.c.snapshot().position, 42.0); // shown while loading
    t.clear();
    ev(&mut t, 0, DeckEvent::FileLoaded);
    assert_eq!(t.log(), ["0 seek 42.0"]);
}

#[test]
fn crossfade_preloads_then_fades() {
    let mut t = T::new("crossfade");
    t.c.play(songs(3), 0);
    t.at(180.0);
    t.clear();
    t.c.tick();
    assert!(t.has("1 load s1 paused=true"), "{:?}", t.log());
    t.at(194.1);
    t.clear();
    t.c.tick();
    let s = t.c.snapshot();
    assert_eq!((s.index, t.current()), (1, "s1".into()));
    assert!(t.has("1 volume 0.0") && t.has("1 pause false"), "{:?}", t.log());
    assert!(!t.has("1 load s1 paused=true"), "preloaded, not loaded again");
    assert_eq!(t.c.tick_ms(), 50);
    t.advance(3000.0);
    t.clear();
    t.c.tick();
    assert!(t.has("0 volume 89.1") && t.has("1 volume 89.1"), "{:?}", t.log()); // k = 0.5, equal power, cubic volume
    t.advance(3000.0);
    t.clear();
    t.c.tick();
    assert!(t.has("0 stop") && t.has("1 volume 100.0"), "{:?}", t.log());
    assert!(!t.c.fading);
}

#[test]
fn gapless_appends_and_moves_on_start_file() {
    let mut t = T::new("gapless");
    t.c.set_crossfade(0);
    t.c.play(songs(3), 0);
    t.at(190.0);
    t.clear();
    t.c.tick();
    assert!(t.has("0 append s1"), "{:?}", t.log());
    t.c.tick();
    assert_eq!(t.log().iter().filter(|l| l.contains("append")).count(), 1);
    let anchor = t.c.anchor;
    ev(&mut t, 0, DeckEvent::StartFile);
    assert_eq!(t.current(), "s1");
    assert_eq!(t.c.anchor, anchor + 200_000.0);
    assert!(t.has("0 drop_first"));
    // a queue edit before the move: the appended song is forgotten
    t.at(190.0);
    t.c.tick();
    t.clear();
    t.c.add(vec![song("x")], 1);
    assert!(t.has("0 clear"), "{:?}", t.log());
}

#[test]
fn end_file_eof_advances_or_stops() {
    let mut t = T::new("eof");
    t.c.play(songs(2), 0);
    t.at(10.0);
    ev(&mut t, 0, DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
    assert_eq!(t.current(), "s1");
    assert!(t.has("1 load s1 paused=true"), "{:?}", t.log());
    ev(&mut t, 1, DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
    assert!(!t.c.snapshot().playing);
    // repeat one: the same song again
    t.c.set_repeat(2);
    t.c.jump(0);
    let k = t.c.cur;
    ev(&mut t, k, DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
    assert_eq!(t.current(), "s0");
}

#[test]
fn three_errors_stop_playback() {
    let mut t = T::new("errors");
    t.c.online = true;
    t.c.play(songs(5), 0);
    let err = || DeckEvent::EndFile { reason: EndReason::Error, error: "loading failed".into() };
    ev(&mut t, 0, err());
    assert_eq!(t.c.snapshot().status, "Can't play T s0: loading failed");
    assert_eq!(t.current(), "s1");
    ev(&mut t, 0, err());
    ev(&mut t, 0, err());
    let s = t.c.snapshot();
    assert_eq!((t.current(), s.playing), ("s2".into(), false));
    assert!(t.c.decks[0].id.is_empty());
    t.clear();
    t.c.toggle(); // opens the song again
    assert!(t.has("0 load s2 paused=false"), "{:?}", t.log());
}

#[test]
fn error_before_online_waits() {
    let mut t = T::new("offline-error");
    t.c.play(songs(2), 0);
    ev(&mut t, 0, DeckEvent::EndFile { reason: EndReason::Error, error: "x".into() });
    assert_eq!(t.current(), "s0");
    assert_eq!(t.c.snapshot().status, "");
    assert!(t.c.decks[0].id.is_empty());
}

#[test]
fn drift_actions() {
    let mut t = T::new("drift");
    t.sync.0.lock().unwrap().listeners = true; // someone follows us: the deck keeps to the timeline
    t.c.play(songs(2), 0);
    t.at(10.0);
    let now = t.c.server_now();
    t.c.anchor = now - 9_800.0; // the deck is 200 ms ahead
    t.clear();
    t.c.tick();
    assert_eq!(t.log(), ["0 speed 0.98"]);
    t.c.anchor = now - 9_990.0;
    t.clear();
    t.c.tick();
    assert_eq!(t.log(), ["0 speed 1"]);
    t.c.anchor = now - 13_000.0;
    t.clear();
    t.c.tick();
    assert_eq!(t.log(), ["0 seek 13.0"]);
    // alone: the timeline moves to the deck
    t.sync.0.lock().unwrap().listeners = false;
    t.at(10.0);
    t.c.anchor = now - 2_000.0;
    let n = t.published().len();
    t.c.tick();
    assert_eq!(t.published().len(), n + 1);
    assert_eq!(t.c.anchor, now - 10_000.0);
}

#[test]
fn position_changed_only_on_change() {
    let mut t = T::new("position");
    t.c.play(songs(2), 0);
    t.c.take_changes();
    t.at(10.0);
    t.c.tick();
    assert!(t.c.take_changes().position);
    t.c.tick();
    assert!(!t.c.take_changes().position);
    ev(&mut t, 0, DeckEvent::Bitrate(Some(900_000.0)));
    assert!(t.c.take_changes().position);
    assert_eq!(t.c.snapshot().bitrate, 900);
}

fn station() -> Value {
    json!({"id": "1", "name": "Wave", "url": "https://radio.example.com/live?auth=pw", "homePage": "https://radio.example.com"})
}

#[test]
fn stream_end_file_error_stops_station() {
    let mut t = T::new("stream-error");
    t.c.play(songs(2), 0);
    t.at(30.0);
    t.c.start_station(&station());
    assert_eq!(t.station.0.lock().unwrap().clone(), ["open https://radio.example.com https://radio.example.com/live?auth=pw"]);
    assert_eq!(t.c.tick_ms(), 1000);
    t.clear();
    ev(&mut t, 0, DeckEvent::EndFile { reason: EndReason::Error, error: String::new() });
    let s = t.c.snapshot();
    assert_eq!(s.status, "Yayın açılamadı");
    assert!(s.station.is_empty() && !s.playing);
    assert!(t.has("0 load s0 paused=true"), "{:?}", t.log()); // back to the queue, where it was
    assert_eq!(s.position, 30.0);
    assert_eq!(t.station.0.lock().unwrap().last().unwrap(), "close");
}

// ---------------------------------------------------------------- Task 3: server side and listening together

fn failed(code: i64, msg: &str) -> Option<Value> {
    Some(json!({"status": "failed", "error": {"code": code, "message": msg}}))
}

fn answers(m: &str, _q: &[(String, String)]) -> Option<Value> {
    match m {
        "ping" | "savePlayQueue" | "scrobble" | "reportPlayback" | "setRating" => ok(json!({})),
        "getOpenSubsonicExtensions" => ok(json!({"openSubsonicExtensions": [{"name": "playbackReport", "versions": [1]}]})),
        "getPlayQueue" => ok(json!({})),
        _ => None,
    }
}

fn has_call(calls: &[String], prefix: &str) -> bool {
    calls.iter().any(|c| c.starts_with(prefix))
}

fn empty(name: &str) -> T {
    let dir = tmp(name);
    let settings: Arc<dyn Settings> = Arc::new(JsonSettings::open(dir.join("settings.json")));
    T::with(name, dir, settings)
}

#[test]
fn login_saves_after_ping_and_starts_sync() {
    let mut t = empty("login");
    t.c.login("music.example.com/", " ana ", "pw");
    assert_eq!(t.c.snapshot().status, "Bağlanıyor…");
    assert!(t.c.snapshot().connecting);
    assert!(t.settings.get_string("token").is_empty(), "saved only once the server answered");
    let calls = t.run(false, answers);
    assert!(has_call(&calls, "ping?") && has_call(&calls, "getPlayQueue?"), "{calls:?}");
    let s = t.c.snapshot();
    assert!(s.online && s.signed_in && !s.connecting);
    assert_eq!(s.status, "");
    assert_eq!(t.settings.get_string("url"), "https://music.example.com");
    assert_eq!(t.settings.get_string("user"), "ana");
    let (salt, token) = (t.settings.get_string("salt"), t.settings.get_string("token"));
    let want: String = md5::Md5::digest(format!("pw{salt}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(token, want);
    assert_eq!(t.sync.0.lock().unwrap().started, ["wss://music.example.com/re-with"]);
    assert!(t.c.playback_report);
}

#[test]
fn wrong_password_logs_out_with_reason() {
    let mut t = empty("badpass");
    t.c.login("https://music.example.com", "ana", "nope");
    t.run(false, |m, q| if m == "ping" { failed(40, "Wrong username or password") } else { answers(m, q) });
    let s = t.c.snapshot();
    assert_eq!(s.status, "Kullanıcı adı ya da şifre yanlış.");
    assert!(!s.online && !s.signed_in && !s.connecting && s.login_failed);
    assert!(t.settings.get_string("token").is_empty());
}

#[test]
fn server_down_retries_after_5s() {
    let mut t = T::new("down");
    t.c.start();
    let calls = t.run(false, |_, _| None);
    assert_eq!(t.c.snapshot().status, "Sunucuya ulaşılamıyor. · 5 sn sonra yeniden denenecek");
    assert!(t.c.snapshot().login_failed);
    assert!(calls.contains(&"timer 5000 (not run)".to_string()), "{calls:?}");
    assert_eq!(t.settings.get_string("token"), "t0ken"); // the saved login stays
    // the retry connects once the server is back
    let calls = t.run(true, answers);
    assert!(calls.is_empty(), "the 5 s timer was taken already: {calls:?}");
    t.c.start();
    t.run(true, answers);
    assert!(t.c.snapshot().online && !t.c.snapshot().login_failed);
}

#[test]
fn logout_cancels_late_replies() {
    let mut t = T::new("late");
    t.c.start();
    let jobs = t.c.take_jobs();
    t.c.logout();
    assert_eq!(t.c.snapshot().status, "Çıkış yapıldı");
    assert!(!t.c.snapshot().login_failed);
    for j in jobs {
        if let Job::Api { then, .. } = j {
            then(&mut t.c, Ok(json!({"status": "ok"})));
        }
    }
    let s = t.c.snapshot();
    assert!(!s.online && !s.signed_in);
    assert!(t.settings.get_string("token").is_empty());
    assert_eq!(t.sync.0.lock().unwrap().stopped, [true]);
}

fn write_local(t: &T, user: &str, ids: &[&str], index: i64, pos: f64) {
    let q: Vec<Value> = ids.iter().map(|i| song(i)).collect();
    let body = json!({"server": "https://music.example.com", "user": user, "queue": q, "index": index, "position": pos});
    std::fs::write(t.dir.join("queue.json"), body.to_string()).unwrap();
}

fn play_queue(m: &str, q: &[(String, String)]) -> Option<Value> {
    if m == "getPlayQueue" {
        return ok(json!({"playQueue": {"entry": [song("s5"), song("s6")], "current": "s6", "position": 12000}}));
    }
    answers(m, q)
}

#[test]
fn restore_queue_from_server_when_untouched() {
    let mut t = T::new("restore");
    write_local(&t, "ana", &["s0", "s1"], 1, 30.0);
    t.c.start();
    assert_eq!(t.current(), "s1");
    assert_eq!(t.c.snapshot().position, 30.0);
    assert!(t.has("0 load s1 paused=true"), "{:?}", t.log());
    t.run(false, play_queue);
    assert_eq!(t.ids(), ["s5", "s6"]);
    assert_eq!((t.current(), t.c.snapshot().position, t.c.snapshot().playing), ("s6".into(), 12.0, false));
    assert_eq!(ids_of(t.published().last().unwrap()), ["s5", "s6"]);

    // touched before the reply (played): the server's queue does not replace it
    let mut t = T::new("restore-touched");
    write_local(&t, "ana", &["s0", "s1"], 1, 30.0);
    t.c.start();
    t.c.toggle();
    t.run(false, play_queue);
    assert_eq!(t.ids(), ["s0", "s1"]);
}

#[test]
fn local_queue_json_round_trip() {
    let mut t = T::new("local");
    t.c.start();
    t.run(false, answers);
    t.c.play(songs(3), 1);
    let calls = t.run(true, answers);
    assert!(has_call(&calls, "savePlayQueue?id=s0&id=s1&id=s2&current=s1&position=0"), "{calls:?}");
    let saved: Value = serde_json::from_slice(&std::fs::read(t.dir.join("queue.json")).unwrap()).unwrap();
    assert_eq!((saved["user"].as_str(), saved["index"].as_i64()), (Some("ana"), Some(1)));
    // another user's queue is not opened
    let mut t2 = T::new("local2");
    write_local(&t2, "bob", &["s0"], 0, 0.0);
    t2.c.start();
    assert_eq!(t2.c.snapshot().index, -1);
}

#[test]
fn persist_is_debounced_and_skipped_while_following() {
    let mut t = T::new("debounce");
    t.c.start();
    t.run(false, answers);
    t.c.play(songs(3), 0);
    t.c.next();
    t.c.toggle();
    let calls = t.run(true, answers);
    assert_eq!(calls.iter().filter(|c| c.starts_with("savePlayQueue?")).count(), 1, "{calls:?}");
    t.follow("root");
    t.c.set_crossfade(3); // refused while following anyway
    t.c.persist_queue();
    assert!(t.c.take_jobs().is_empty());
}

#[test]
fn persist_now_sends_save_and_stopped_report() {
    let mut t = T::new("persist-now");
    t.c.start();
    t.run(false, answers);
    t.c.play(songs(2), 0);
    t.at(20.0);
    t.advance(1500.0);
    t.c.tick();
    t.advance(1500.0);
    t.c.tick();
    t.run(false, answers); // the playing report went out
    let names: Vec<String> = t.c.persist_now().into_iter().map(|j| match j {
        Job::Api { method, query, .. } => format!("{method}?{}", crate::queue::encode_query(&query)),
        _ => String::new(),
    }).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names[0].starts_with("reportPlayback?mediaId=s0&mediaType=song&positionMs=20000&state=stopped"), "{names:?}");
    assert!(names[1].starts_with("savePlayQueue?id=s0&id=s1&current=s0&position=20000"), "{names:?}");
    assert!(t.dir.join("queue.json").exists());
    assert!(t.c.persist_now().iter().all(|j| matches!(j, Job::Api { method, .. } if method == "savePlayQueue")), "stopped is sent once");
}

#[test]
fn scrobble_now_playing_then_submission_at_half() {
    let mut t = T::new("scrobble");
    t.c.online = true;
    t.c.play(songs(2), 0);
    t.at(1.0);
    t.c.tick();
    let calls = t.run(false, answers);
    assert!(has_call(&calls, "scrobble?id=s0&submission=false"), "{calls:?}");
    t.at(99.0);
    t.c.tick();
    assert!(!has_call(&t.run(false, answers), "scrobble"));
    t.at(100.0);
    t.c.tick();
    t.c.tick();
    let calls = t.run(false, answers);
    assert_eq!(calls.iter().filter(|c| c.starts_with("scrobble?id=s0&submission=true&time=")).count(), 1, "{calls:?}");
    // off: nothing
    t.c.set_option("scrobble", false.into());
    t.c.jump(1);
    t.at(150.0);
    t.c.tick();
    assert!(!t.run(false, answers).iter().any(|c| c.contains("submission=true")));
}

#[test]
fn report_playback_gate_holds() {
    let mut t = T::new("report");
    t.c.online = true;
    t.c.playback_report = true;
    t.c.play(songs(2), 0);
    t.at(5.0);
    t.c.tick();
    assert!(!has_call(&t.run(false, answers), "reportPlayback"));
    t.advance(1001.0);
    t.c.tick();
    let calls = t.run(false, answers);
    assert!(has_call(&calls, "reportPlayback?mediaId=s0&mediaType=song&positionMs=5000&state=playing"), "{calls:?}");
    assert!(!has_call(&calls, "scrobble?id=s0&submission=false"), "the report is the now playing: {calls:?}");
}

fn similar(ids: &[&str]) -> Option<Value> {
    ok(json!({"similarSongs": {"song": ids.iter().map(|i| song(i)).collect::<Vec<_>>()}}))
}

#[test]
fn extend_without_dj_appends_similar_then_random() {
    let mut t = T::new("extend");
    t.c.online = true;
    t.c.play(songs(1), 0);
    t.c.tick();
    t.run(false, |m, _| if m == "getSimilarSongs" { similar(&["s0", "x1", "x2"]) } else { None });
    assert_eq!(t.ids(), ["s0", "x1", "x2"]);
    // nothing new: random songs instead
    let mut t = T::new("extend-random");
    t.c.online = true;
    t.c.play(songs(1), 0);
    t.c.tick();
    let calls = t.run(false, |m, _| match m {
        "getSimilarSongs" => similar(&["s0"]),
        "getRandomSongs" => ok(json!({"randomSongs": {"song": [song("r1")]}})),
        _ => None,
    });
    assert!(has_call(&calls, "getRandomSongs?size=10"), "{calls:?}");
    assert_eq!(t.ids(), ["s0", "r1"]);
    // nothing at all: waits 15 s
    let mut t = T::new("extend-none");
    t.c.online = true;
    t.c.play(songs(1), 0);
    t.c.tick();
    t.run(false, |_, _| None);
    t.c.tick();
    assert!(t.c.take_jobs().is_empty());
    t.advance(15_000.0);
    t.c.tick();
    assert!(!t.c.take_jobs().is_empty());
}

#[test]
fn extend_dj_marks_picks() {
    let mut t = T::new("extend-dj");
    t.c.online = true;
    t.c.set_dj(true);
    t.c.play(songs(1), 0);
    t.c.tick();
    let pool: Vec<Value> = (0..30)
        .map(|i| {
            let mut s = song(&format!("p{i}"));
            s["artist"] = format!("Artist {i}").into(); // the picker spreads artists
            s
        })
        .collect();
    let calls = t.run(false, |m, _| match m {
        "getSimilarSongs" => ok(json!({"similarSongs": {"song": pool[..5]}})),
        "getRandomSongs" => ok(json!({"randomSongs": {"song": pool[5..]}})),
        _ => None,
    });
    assert!(has_call(&calls, "getSimilarSongs?id=s0&count=50"), "{calls:?}");
    assert!(has_call(&calls, "getRandomSongs?size=100&genre=Pop"), "{calls:?}");
    assert!(has_call(&calls, "getRandomSongs?size=400"), "{calls:?}");
    let q = t.c.snapshot().queue;
    assert_eq!(q.len(), 11, "radioBatch ahead");
    assert!(q[1..].iter().all(|s| s["dj"] == true));
}

#[test]
fn late_extend_after_new_play_dropped() {
    let mut t = T::new("extend-late");
    t.c.online = true;
    t.c.play(songs(1), 0);
    t.c.tick();
    let jobs = t.c.take_jobs();
    t.c.play(vec![song("n0"), song("n1"), song("n2")], 0);
    for j in jobs {
        if let Job::Api { then, .. } = j {
            then(&mut t.c, Ok(similar(&["x1"]).unwrap()));
        }
    }
    assert_eq!(t.ids(), ["n0", "n1", "n2"]);
}

fn stations(m: &str, q: &[(String, String)]) -> Option<Value> {
    if m == "getInternetRadioStations" {
        return ok(json!({"internetRadioStations": {"internetRadioStation": [
            {"id": "1", "name": "Wave", "streamUrl": "https://radio.example.com/live?auth=pw", "homePageUrl": "https://radio.example.com"}]}}));
    }
    answers(m, q)
}

fn subwave_now(title: &str) -> crate::subwave::SubwaveSnapshot {
    crate::subwave::SubwaveSnapshot {
        active: true,
        now: json!({"title": title, "artist": "DJ", "id": "lib1", "duration": 180}),
        cover_url: String::new(),
        online: true,
        peeked: Map::new(),
        lyrics: json!({}),
        lyrics_state: String::new(),
        like_enabled: false,
        liked: false,
        like_blocked_ms: 0,
        request: json!({}),
        started_at: 0,
        server_at: 0,
        local_at: 0,
        buffer_ms: 0,
    }
}

#[test]
fn station_opens_subwave_and_resumes_queue_on_stop() {
    let mut t = T::new("station");
    t.c.online = true;
    t.c.play(songs(2), 0);
    t.at(30.0);
    t.c.play_station("1");
    t.run(false, stations);
    let s = t.c.snapshot();
    assert_eq!(s.station["name"], "Wave");
    assert!(s.playing);
    assert!(t.has("0 load https://radio.example.com/live?auth=pw paused=false"), "{:?}", t.log());
    assert_eq!(t.station.0.lock().unwrap().clone(), ["open https://radio.example.com https://radio.example.com/live?auth=pw"]);
    let p = t.published().last().cloned().unwrap();
    assert_eq!(p["station"], "1");
    assert!(!Value::Object(p.clone()).to_string().contains("auth=pw"), "peers never see the stream URL");
    t.c.on_subwave(&subwave_now("Song A"), 5000);
    assert_eq!(t.published().last().unwrap()["radio"], json!({"name": "Wave", "title": "Song A", "artist": "DJ"}));
    assert_eq!((t.c.radio_id.as_str(), t.c.radio_pos_ms), ("lib1", 5000.0));
    t.c.toggle();
    let s = t.c.snapshot();
    assert!(s.station.is_empty() && !s.playing);
    assert_eq!(s.position, 30.0);
    assert_eq!(t.station.0.lock().unwrap().last().unwrap(), "close");
}

#[test]
fn logout_during_station_unloads_stream() {
    let mut t = T::new("station-logout");
    t.c.start();
    t.run(false, answers);
    t.c.play(songs(1), 0);
    t.c.play_station("1");
    t.run(false, stations);
    t.c.logout();
    let loads: Vec<String> = t.log().into_iter().filter(|l| l.starts_with("0 load")).collect();
    assert_eq!(loads.last().unwrap(), "0 load s0 paused=true", "{loads:?}");
    assert_eq!(t.station.0.lock().unwrap().last().unwrap(), "close");
}

#[test]
fn on_state_applies_root_and_walks_past_finished_tracks() {
    let mut t = T::new("on-state");
    t.follow("root");
    let now = t.c.server_now();
    let state = json!({"type": "state", "queue": songs(3), "index": 0, "playing": true, "anchor": now - 250_000.0,
                       "xfade": 6, "position": 0, "shuffle": true, "repeat": 1, "station": ""});
    t.c.on_sync(&SyncEvent::StateReceived(state.as_object().cloned().unwrap()));
    let s = t.c.snapshot();
    assert_eq!((t.current(), s.playing, s.shuffle, s.repeat), ("s1".into(), true, true, 1));
    assert!((s.position - 56.0).abs() < 1e-6, "{}", s.position);
    assert!(t.has("0 load s1 paused=false"), "{:?}", t.log());
    assert!(!t.settings.get_bool("shuffle", false), "the root's modes are not saved as ours");
    // the same song again (re-anchor): no reload, a seek only past half a second
    t.clear();
    t.at(56.2);
    let mut again = state.clone();
    again["anchor"] = (now - 250_200.0).into();
    t.c.on_sync(&SyncEvent::StateReceived(again.as_object().cloned().unwrap()));
    assert!(t.log().iter().all(|l| !l.contains("load") && !l.contains("seek")), "{:?}", t.log());
    // ended: our own modes back
    t.follow("");
    t.c.on_sync(&SyncEvent::Ended("host-left".into()));
    assert!(!t.c.snapshot().shuffle);
    assert_eq!(t.c.snapshot().repeat, 0);
}

#[test]
fn on_state_station_opens_same_station() {
    let mut t = T::new("on-state-station");
    t.c.online = true;
    t.follow("root");
    let state = json!({"queue": songs(2), "index": 1, "playing": true, "anchor": 0, "xfade": 6, "position": 12, "station": "1"});
    t.c.on_sync(&SyncEvent::StateReceived(state.as_object().cloned().unwrap()));
    assert_eq!(t.c.snapshot().index, 1);
    t.run(false, stations);
    assert_eq!(t.c.snapshot().station["id"], "1");
}

#[test]
fn exclusive_turned_off_by_peer_crossfade() {
    let mut t = T::new("peer-xfade");
    t.c.set_exclusive(true);
    t.follow("root");
    let state = json!({"queue": songs(2), "index": 0, "playing": false, "anchor": 0, "xfade": 6, "position": 0, "station": ""});
    t.c.on_sync(&SyncEvent::StateReceived(state.as_object().cloned().unwrap()));
    let s = t.c.snapshot();
    assert!(!s.exclusive);
    assert_eq!(s.crossfade, 6);
    assert_eq!(s.status, "Exclusive off: a peer turned crossfade on");
}

#[test]
fn commands_from_followers_act_here() {
    let mut t = T::new("commands");
    t.c.play(songs(3), 0);
    t.c.on_sync(&SyncEvent::CommandReceived { op: "jump".into(), args: json!({"i": 2}) });
    assert_eq!(t.current(), "s2");
    t.c.on_sync(&SyncEvent::CommandReceived { op: "add".into(), args: json!({"songs": [song("x")], "where": 2}) });
    assert_eq!(t.ids().last().unwrap(), "x");
    t.c.on_sync(&SyncEvent::Failed("denied".into()));
    assert_eq!(t.c.snapshot().status, "Host doesn't allow control");
    let n = t.published().len();
    t.c.on_sync(&SyncEvent::Connected);
    assert_eq!(t.published().len(), n + 1);
}

#[test]
fn request_replies_for_themes_and_gen_drops() {
    let mut t = T::new("request");
    assert!(!t.c.request(1, "savePlayQueue", Map::new(), false), "not readable");
    assert!(!t.c.request(1, "getAlbum", Map::new(), true), "not writable");
    assert!(t.c.request(2, "getAlbum", Map::new(), false));
    assert_eq!(t.c.take_replies(), [(2, None, true)], "offline: null at once");
    t.c.start();
    t.run(false, answers);
    assert!(t.c.request(3, "getInternetRadioStations", Map::new(), false));
    t.run(false, stations);
    let r = t.c.take_replies();
    assert_eq!(r.len(), 1);
    let (id, v, call) = r[0].clone();
    let st = &v.unwrap()["internetRadioStations"]["internetRadioStation"][0];
    assert_eq!((id, call, st.get("streamUrl").is_none(), st["originUrl"].as_str()), (3, true, true, Some("https://radio.example.com")));
    t.c.request(4, "getAlbum", Map::new(), false);
    let jobs = t.c.take_jobs();
    t.c.logout();
    for j in jobs {
        if let Job::Api { then, .. } = j {
            then(&mut t.c, Ok(json!({"status": "ok", "album": {}})));
        }
    }
    assert_eq!(t.c.take_replies(), [(4, None, false)]);
}

#[test]
fn set_rating_updates_queue_copy() {
    let mut t = T::new("rating");
    t.c.start();
    t.run(false, answers);
    t.c.play(songs(2), 0);
    let mut p = Map::new();
    p.insert("id".into(), "s1".into());
    p.insert("rating".into(), 4.into());
    t.c.request(1, "setRating", p, true);
    let n = t.published().len();
    t.run(false, answers);
    assert_eq!(t.c.snapshot().queue[1]["userRating"], 4);
    assert_eq!(t.published().len(), n + 1);
}

#[test]
fn demo_has_no_token() {
    let mut t = T::new("demo");
    t.c.demo();
    let s = t.c.snapshot();
    assert!(s.online && !s.signed_in);
    assert_eq!(s.queue.len(), 7);
    assert_eq!(s.status, "Demo · sunucu yok");
    assert_eq!(t.settings.get_string("token"), "t0ken", "in memory only");
    assert!(t.c.stream_url("x").is_empty(), "demo songs never reach a real server");
}

#[test]
fn download_one_at_a_time() {
    let mut t = T::new("download");
    t.c.start();
    t.run(false, answers);
    t.c.download("al1", t.dir.join("a.zip"));
    assert!(matches!(t.c.take_jobs().as_slice(), [Job::Download { id, .. }] if id == "al1"));
    t.c.download("al2", t.dir.join("b.zip"));
    assert_eq!(t.c.snapshot().status, "Bir indirme zaten sürüyor.");
    t.c.download_done(Ok("a.zip".into()));
    assert_eq!(t.c.snapshot().status, "İndirildi: a.zip");
    t.c.download("al2", t.dir.join("b.zip"));
    assert_eq!(t.c.take_jobs().len(), 1);
}

#[test]
fn android_look_options_survive_a_restart() {
    let t = T::new("look");
    let mut c = t.c;
    c.set_option("amoled", true.into());
    c.set_option("gridColumns", 3.into());
    c.set_option("albumOrder", "recent".into());
    c.set_option("coverTap", "queue".into());
    c.set_option("tabs", "library,radio".into());
    c.set_option("coverQuality", "low".into());
    let again = T::with("look", t.dir.clone(), t.settings.clone());
    let o = again.c.snapshot().options;
    assert_eq!(o["amoled"], json!(true));
    assert_eq!(o["gridColumns"], json!(3));
    assert_eq!(o["albumOrder"], json!("recent"));
    assert_eq!(o["coverTap"], json!("queue"));
    assert_eq!(o["tabs"], json!("library,radio"));
    assert_eq!(o["coverQuality"], json!("low"));
}

// ---------------------------------------------------------------- the song cache, offline, transcodes

/// Online with a cache of 1 GB; nothing on disk yet.
fn caching(name: &str) -> T {
    let mut t = T::new(name);
    t.c.start();
    t.run(false, |m, _| (m == "ping").then(|| ok(json!({}))).flatten());
    assert!(t.c.is_online());
    t.c.set_option("offlineGb", json!(1));
    t.c.set_option("autoContinue", json!(false));
    t.c.set_crossfade(0);
    t
}

fn cached(calls: &[String]) -> Vec<String> {
    let id = |c: &String| c.strip_prefix("cache id=").map(|q| q.split('&').next().unwrap().to_string());
    calls.iter().filter_map(id).collect()
}

#[test]
fn songs_coming_up_are_cached_then_the_one_heard() {
    let mut t = caching("fill");
    t.c.set_option("offlineAhead", json!(2));
    t.c.play(songs(5), 0);
    t.at(10.0);
    let mut got = vec![];
    for _ in 0..4 {
        t.advance(1500.0);
        t.c.tick();
        got.extend(cached(&t.run(false, |_, _| None)));
    }
    assert_eq!(got, ["s1", "s2"], "the next two; the playing one is not worth it yet");
    t.at(150.0);
    for _ in 0..3 {
        t.advance(1500.0);
        t.at(150.0);
        t.c.tick();
        got.extend(cached(&t.run(false, |_, _| None)));
    }
    assert_eq!(got, ["s1", "s2", "s0"]);
    assert_eq!(t.c.snapshot().cache_bytes, 12);
    // the next song plays from the file
    t.clear();
    t.c.next();
    assert!(t.log().iter().any(|l| l.starts_with("0 load ") && l.contains(".raw ")), "{:?}", t.log());
    // the clear button: all but what plays and the one after it
    t.c.clear_song_cache();
    assert_eq!(t.c.snapshot().cache_bytes, 8);
}

#[test]
fn off_or_paused_nothing_is_cached() {
    let mut t = caching("off");
    t.c.play(songs(5), 0);
    t.at(10.0);
    t.c.toggle();
    t.advance(1500.0);
    t.c.tick();
    assert!(cached(&t.run(false, |_, _| None)).is_empty(), "paused");
    t.c.toggle();
    t.c.set_option("offlineGb", json!(0));
    t.advance(1500.0);
    t.c.tick();
    assert!(cached(&t.run(false, |_, _| None)).is_empty(), "off");
}

#[test]
fn offline_the_held_songs_play() {
    let mut t = caching("offline");
    let hold = |t: &T, id: &str| std::fs::write(t.c.songs.path("https://music.example.com", id, "raw"), b"x").unwrap();
    for id in ["s1", "s3"] {
        hold(&t, id);
    }
    t.c.play(songs(5), 0);
    t.run(true, |_, _| ok(json!({}))); // the queue goes to disk
    let mut t = T::with("offline", t.dir.clone(), t.settings.clone());
    t.c.start();
    assert!(!t.c.snapshot().offline, "still connecting");
    t.run(false, |_, _| None);
    let s = t.c.snapshot();
    assert!(s.offline && !s.online && s.status.starts_with("Çevrimdışı"), "{}", s.status);
    t.c.jump(2);
    assert_eq!(t.current(), "s0", "not held: stays");
    t.c.toggle();
    assert_eq!((t.current(), t.c.snapshot().playing), ("s1".into(), true));
    t.c.next();
    assert_eq!(t.current(), "s3");
    t.c.prev();
    assert_eq!(t.current(), "s1");
    t.c.deck_event(0, DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
    assert_eq!(t.current(), "s3");
    let k = t.c.cur;
    t.c.deck_event(k, DeckEvent::EndFile { reason: EndReason::Eof, error: String::new() });
    assert!(!t.c.snapshot().playing, "nothing held after it");
    // the server is back
    t.c.connect_server();
    t.run(false, |m, _| (m == "ping").then(|| ok(json!({}))).flatten());
    assert!(t.c.is_online() && !t.c.snapshot().offline);
    assert_eq!(t.current(), "s3");
}

#[test]
fn without_held_songs_there_is_no_offline() {
    let mut t = caching("no-offline");
    t.c.play(songs(2), 0);
    t.run(true, |_, _| ok(json!({})));
    let mut t = T::with("no-offline", t.dir.clone(), t.settings.clone());
    t.c.start();
    t.run(false, |_, _| None);
    assert!(!t.c.snapshot().offline);
}

#[test]
fn a_live_transcode_seeks_by_a_new_stream() {
    let mut t = T::new("transcode");
    t.c.set_option("transcodeMobile", json!("opus-128"));
    t.c.play(songs(2), 0);
    assert!(!t.c.decks[0].live_transcode, "Wi-Fi: the original");
    t.c.set_mobile(true);
    t.clear();
    t.c.play(songs(2), 0);
    assert!(t.c.source("s0").url.contains("stream?id=s0&format=opus&maxBitRate=128&"), "{}", t.c.source("s0").url);
    t.c.deck_event(0, DeckEvent::FileLoaded);
    t.c.deck_event(0, DeckEvent::Duration(3.0));
    assert_eq!(t.c.snapshot().duration, 200.0, "the song's, not the stream's guess");
    t.c.seek(95.7);
    assert!(t.has("0 load s0 from 95 paused=false"), "{:?}", t.log());
    assert_eq!(t.c.snapshot().position, 95.7, "shown at once");
    t.c.deck_event(0, DeckEvent::FileLoaded);
    t.c.deck_event(0, DeckEvent::Position(Some(1.5)));
    assert_eq!(t.c.snapshot().position, 96.5);
    assert!(!t.log().iter().any(|l| l.contains("seek")), "{:?}", t.log());
    // small enough already: as it is
    let mut small = song("s9");
    small["bitRate"] = 96.into();
    t.c.play(vec![small], 0);
    assert!(!t.c.decks[0].live_transcode);
}

#[test]
fn the_audio_device_reaches_both_decks() {
    let mut t = T::new("device");
    t.c.set_option("audioDevice", json!("pipewire/dac"));
    assert!(t.has("0 device pipewire/dac") && t.has("1 device pipewire/dac"), "{:?}", t.log());
    let t = T::with("device", t.dir.clone(), t.settings.clone());
    assert!(t.has("0 device pipewire/dac") && t.has("1 device pipewire/dac"), "at start too: {:?}", t.log());
}
