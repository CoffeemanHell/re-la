//! SubwaveClient against a fake SUB/WAVE station (axum): detection, polling, lyrics, hearts, requests.
use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use rela_core::subwave::{SubwaveClient, SubwaveEvent, SubwaveListener, SubwaveSnapshot, Timings};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Answer {
    status: u16,
    body: String,
    headers: Vec<(&'static str, String)>,
    delay_ms: u64,
}

fn ok(v: Value) -> Answer {
    Answer { status: 200, body: v.to_string(), headers: vec![], delay_ms: 0 }
}

fn code(status: u16, v: Value) -> Answer {
    Answer { status, body: v.to_string(), headers: vec![], delay_ms: 0 }
}

/// One fake station. Answers are looked up by "METHOD path" (query left out); every request is logged.
#[derive(Default)]
struct Fake {
    answers: HashMap<String, Vec<Answer>>, // a list: each request takes the next, the last one repeats
    log: Vec<(String, Option<String>)>,    // ("GET /api/x?y", x-station-auth)
}

type Shared = Arc<Mutex<Fake>>;

async fn handle(State(f): State<Shared>, headers: HeaderMap, req: Request) -> Response {
    let key = format!("{} {}", req.method(), req.uri().path());
    let full = format!("{} {}", req.method(), req.uri());
    let auth = headers.get("x-station-auth").map(|h| h.to_str().unwrap().to_string());
    let answer = {
        let mut f = f.lock().unwrap();
        f.log.push((full, auth));
        let list = f.answers.entry(key).or_default();
        if list.len() > 1 { Some(list.remove(0)) } else { list.first().cloned() }
    };
    let Some(a) = answer else { return StatusCode::NOT_FOUND.into_response() };
    if a.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(a.delay_ms)).await;
    }
    let mut r = (StatusCode::from_u16(a.status).unwrap(), a.body).into_response();
    for (k, v) in a.headers {
        r.headers_mut().insert(k, v.parse().unwrap());
    }
    r
}

struct Station {
    _rt: tokio::runtime::Runtime,
    home: String,
    fake: Shared,
}

impl Station {
    fn start() -> Station {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let fake: Shared = Default::default();
        let f = fake.clone();
        let port = rt.block_on(async move {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = l.local_addr().unwrap().port();
            tokio::spawn(async move { axum::serve(l, Router::new().fallback(handle).with_state(f)).await.unwrap() });
            port
        });
        Station { _rt: rt, home: format!("http://127.0.0.1:{port}"), fake }
    }
    fn set(&self, key: &str, answers: Vec<Answer>) {
        self.fake.lock().unwrap().answers.insert(key.into(), answers);
    }
    fn count(&self, prefix: &str) -> usize {
        self.fake.lock().unwrap().log.iter().filter(|(p, _)| p.starts_with(prefix)).count()
    }
    fn log(&self) -> Vec<(String, Option<String>)> {
        self.fake.lock().unwrap().log.clone()
    }
    /// A SUB/WAVE station playing `title` (subsonic id `id`), with hearts on.
    fn sw(&self, title: &str, id: &str) {
        self.set("GET /api/now-playing", vec![ok(np(title, id))]);
        self.set("GET /api/state", vec![ok(json!({"current": {"startedAt": "2026-11-11T18:24:15.623Z"}, "upcoming": []}))]);
        self.set("GET /api/like", vec![ok(json!({"enabled": true, "liked": false}))]);
        self.set("GET /api/lyrics/current", vec![ok(json!({"songId": id, "synced": true, "lines": [{"startMs": 0, "text": "la"}]}))]);
    }
}

fn np(title: &str, id: &str) -> Value {
    json!({"nowPlaying": {"title": title, "artist": "A", "subsonic_id": id, "duration": 100}, "dj": {"name": "wave"},
           "listeners": {"current": 1}, "streamOnline": true, "context": {"at": "2026-11-11T18:29:05.950Z"}})
}

fn fast() -> Timings {
    Timings {
        poll: Duration::from_millis(100),
        retry: Duration::from_millis(300),
        tick: Duration::from_millis(50),
        timeout: Duration::from_secs(2),
        lyrics_retry: Duration::from_millis(100),
        request_poll: Duration::from_millis(100),
        request_give_up: Duration::from_secs(1),
    }
}

struct Collect(Mutex<Sender<(SubwaveEvent, SubwaveSnapshot)>>);
impl SubwaveListener for Collect {
    fn on_event(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot) {
        let _ = self.0.lock().unwrap().send((event, snapshot));
    }
}

fn client() -> (SubwaveClient, Receiver<(SubwaveEvent, SubwaveSnapshot)>) {
    let (tx, rx) = channel();
    (SubwaveClient::new(Arc::new(Collect(Mutex::new(tx))), fast()), rx)
}

/// Reads snapshots until one matches; panics after `secs`. Returns the match.
fn wait_for(rx: &Receiver<(SubwaveEvent, SubwaveSnapshot)>, secs: f64, what: &str, pred: impl Fn(&SubwaveSnapshot) -> bool) -> SubwaveSnapshot {
    let end = Instant::now() + Duration::from_secs_f64(secs);
    loop {
        match rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
            Ok((_, s)) if pred(&s) => return s,
            Ok(_) => {}
            Err(_) => panic!("timed out waiting for {what}"),
        }
    }
}

/// The last snapshot seen within `secs` (events keep coming while active: ticks).
fn last(rx: &Receiver<(SubwaveEvent, SubwaveSnapshot)>, secs: f64) -> Option<SubwaveSnapshot> {
    let end = Instant::now() + Duration::from_secs_f64(secs);
    let mut l = None;
    while let Ok((_, s)) = rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
        l = Some(s);
    }
    l
}

#[test]
fn detects_and_polls() {
    let st = Station::start();
    st.sw("One", "abc");
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/stream".into());
    let s = wait_for(&rx, 3.0, "active", |s| s.active && s.now["title"] == "One");
    assert!(s.online);
    assert_eq!(s.cover_url, format!("{}/api/cover/abc", st.home));
    assert_eq!(c.cover_url("x"), format!("{}/api/cover/x", st.home));
    st.sw("Two", "def");
    wait_for(&rx, 3.0, "the next track from a later poll", |s| s.now["title"] == "Two" && s.lyrics_state == "ok");
    c.close();
    wait_for(&rx, 3.0, "closed", |s| !s.active && s.now == json!({}));
    assert_eq!(c.cover_url("x"), "");
}

#[test]
fn plain_radio_is_left_alone() {
    for answer in [ok(json!({"status": "ok"})), Answer { status: 200, body: "<html>".into(), headers: vec![], delay_ms: 0 }] {
        let st = Station::start();
        st.set("GET /api/now-playing", vec![answer]);
        let (c, rx) = client();
        c.open(st.home.clone(), "http://radio.example.com/stream".into());
        let l = last(&rx, 0.8);
        assert!(l.is_none_or(|s| !s.active), "a plain radio never turns active");
        assert_eq!(st.count("GET /api/now-playing"), 1, "asked once, never again");
    }
}

#[test]
fn busy_at_first_is_asked_again() {
    for status in [503, 429] {
        let st = Station::start();
        st.sw("One", "abc");
        st.set("GET /api/now-playing", vec![code(status, json!({})), ok(np("One", "abc"))]);
        let (c, rx) = client();
        c.open(st.home.clone(), "http://radio.example.com/stream".into());
        wait_for(&rx, 3.0, "active", |s| s.active);
    }
}

#[test]
fn server_down_goes_offline_and_retries() {
    let st = Station::start();
    st.sw("One", "abc");
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/stream".into());
    wait_for(&rx, 3.0, "active", |s| s.active);
    st.set("GET /api/now-playing", vec![code(500, json!({}))]);
    wait_for(&rx, 3.0, "offline", |s| !s.online);
    let before = st.count("GET /api/now-playing");
    std::thread::sleep(Duration::from_millis(1000));
    let tries = st.count("GET /api/now-playing") - before;
    assert!((2..=5).contains(&tries), "retry every 300 ms, not every 100 ms poll: {tries} tries in 1 s");
    st.sw("Back", "abc");
    wait_for(&rx, 3.0, "online again", |s| s.online && s.now["title"] == "Back");
}

#[test]
fn late_reply_for_old_station_is_dropped() {
    let slow = Station::start();
    slow.sw("Old", "a");
    slow.set("GET /api/now-playing", vec![Answer { delay_ms: 500, ..ok(np("Old", "a")) }]);
    let fast_st = Station::start();
    fast_st.sw("New", "b");
    let (c, rx) = client();
    c.open(slow.home.clone(), "http://radio.example.com/s".into());
    c.open(fast_st.home.clone(), "http://radio.example.com/s".into());
    wait_for(&rx, 3.0, "the new station", |s| s.now["title"] == "New");
    // the old station's reply comes in meanwhile: not even for a moment may it show
    let end = Instant::now() + Duration::from_millis(900);
    while let Ok((_, s)) = rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
        assert_eq!(s.now["title"], "New", "a late reply from the old station showed");
    }
}

fn lyrics_state_for(answers: Vec<Answer>, want: &str) -> (Station, SubwaveClient, Receiver<(SubwaveEvent, SubwaveSnapshot)>) {
    let st = Station::start();
    st.sw("One", "abc");
    st.set("GET /api/lyrics/current", answers);
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/stream".into());
    wait_for(&rx, 3.0, want, |s| s.lyrics_state == want);
    (st, c, rx)
}

#[test]
fn lyrics_states() {
    lyrics_state_for(vec![code(401, json!({}))], "auth");
    lyrics_state_for(vec![code(403, json!({}))], "off");
    lyrics_state_for(vec![ok(json!({"songId": "abc", "lines": []}))], "none");
    // 503 and "pending" are retried until the lyrics are there
    let (_st, _c, rx) = lyrics_state_for(
        vec![code(503, json!({})), ok(json!({"songId": "abc", "lines": [], "pending": true})),
             ok(json!({"songId": "abc", "lines": [{"startMs": 0, "text": "la"}]}))],
        "retry",
    );
    let s = wait_for(&rx, 3.0, "lyrics after retries", |s| s.lyrics_state == "ok");
    assert_eq!(s.lyrics["lines"][0]["value"], "la");
}

#[test]
fn lyrics_send_password_header_only() {
    let st = Station::start();
    st.sw("One", "abc");
    let (c, rx) = client();
    c.open(st.home.clone(), "https://radio.example.com/stream.flac?auth=sekret".into());
    wait_for(&rx, 3.0, "lyrics", |s| s.lyrics_state == "ok");
    let log = st.log();
    for (path, auth) in &log {
        assert!(!path.contains("sekret"), "password in a URL: {path}");
        if path.starts_with("GET /api/lyrics/") {
            assert_eq!(auth.as_deref(), Some("sekret"));
        } else {
            assert_eq!(auth, &None, "password sent to {path}");
        }
    }
}

#[test]
fn station_request_stays_on_origin() {
    let other = Station::start();
    other.sw("Elsewhere", "zzz");
    let st = Station::start();
    st.sw("One", "abc");
    let to = format!("{}/api/lyrics/current?songId=abc", other.home);
    st.set("GET /api/lyrics/current", vec![Answer { status: 302, body: String::new(), headers: vec![("location", to)], delay_ms: 0 }]);
    let (c, rx) = client();
    c.open(st.home.clone(), "https://radio.example.com/s?auth=sekret".into());
    wait_for(&rx, 3.0, "lyrics given up", |s| s.lyrics_state == "none");
    assert_eq!(other.count("GET"), 0, "followed a redirect to another origin with the password");
}

#[test]
fn like_429_blocks_then_clears() {
    let st = Station::start();
    st.sw("One", "abc");
    st.set("POST /api/like", vec![Answer { status: 429, body: "{}".into(), headers: vec![("retry-after", "1".into())], delay_ms: 0 }]);
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/s".into());
    wait_for(&rx, 3.0, "hearts on", |s| s.like_enabled);
    c.like();
    wait_for(&rx, 3.0, "blocked", |s| s.like_blocked_ms == 1000);
    c.like(); // blocked: not sent
    wait_for(&rx, 3.0, "unblocked", |s| s.like_blocked_ms == 0);
    assert_eq!(st.count("POST /api/like"), 1);
    st.set("POST /api/like", vec![ok(json!({"liked": true}))]);
    c.like();
    wait_for(&rx, 3.0, "liked", |s| s.liked);
}

#[test]
fn like_409_refetches() {
    let st = Station::start();
    st.sw("One", "abc");
    st.set("POST /api/like", vec![code(409, json!({}))]);
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/s".into());
    wait_for(&rx, 3.0, "hearts on", |s| s.like_enabled);
    assert_eq!(st.count("GET /api/like"), 1);
    c.like();
    let end = Instant::now() + Duration::from_secs(2);
    while st.count("GET /api/like") < 2 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(st.count("GET /api/like"), 2, "the track changed under us: ask again");
}

#[test]
fn request_flow() {
    let st = Station::start();
    st.sw("One", "abc");
    st.set("POST /api/request", vec![ok(json!({"success": true, "requestId": "r1"}))]);
    st.set("GET /api/request/r1", vec![ok(json!({"status": "pending"})), ok(json!({"status": "resolved", "track": {"title": "T", "artist": "A"}}))]);
    let (c, rx) = client();
    c.open(st.home.clone(), "http://radio.example.com/s".into());
    wait_for(&rx, 3.0, "active", |s| s.active);
    c.send_request("  bir şarkı  ".into(), " ali ".into());
    wait_for(&rx, 3.0, "pending", |s| s.request["state"] == "pending");
    let s = wait_for(&rx, 3.0, "done", |s| s.request["state"] == "done");
    assert_eq!(s.request["title"], "T");
    // unknown id
    st.set("GET /api/request/r1", vec![code(404, json!({}))]);
    c.send_request("x".into(), "".into());
    wait_for(&rx, 3.0, "unknown", |s| s.request["state"] == "unknown");
    // refused
    st.set("POST /api/request", vec![code(500, json!({"message": "busy"}))]);
    c.send_request("x".into(), "".into());
    let s = wait_for(&rx, 3.0, "failed", |s| s.request["state"] == "failed");
    assert_eq!(s.request["message"], "busy");
    // no answer within the give-up time: unknown
    st.set("POST /api/request", vec![ok(json!({"requestId": "r2"}))]);
    st.set("GET /api/request/r2", vec![ok(json!({"status": "pending"}))]);
    c.send_request("x".into(), "".into());
    wait_for(&rx, 3.0, "given up", |s| s.request["state"] == "unknown");
}

#[test]
fn probe_peeks_once_per_home() {
    let st = Station::start();
    st.sw("One", "abc");
    let (c, rx) = client();
    c.probe(st.home.clone());
    c.probe(st.home.clone());
    let s = wait_for(&rx, 3.0, "peeked", |s| s.peeked.contains_key(&st.home));
    assert_eq!(s.peeked[&st.home], json!({"title": "One", "artist": "A", "dj": "wave"}));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(st.count("GET /api/now-playing"), 1);
}
