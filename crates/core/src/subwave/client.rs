//! SUB/WAVE listener client. Player says which station opened
//! (open/close); if its home page answers like SUB/WAVE the client polls /now-playing and /state, fetches lyrics and
//! hearts, and follows song requests. Runs on its own thread; the front end gets snapshots.
use super::parse::*;
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

#[derive(Debug, Clone)]
pub struct Timings {
    pub poll: Duration,
    pub retry: Duration,
    pub tick: Duration,
    pub timeout: Duration,
    pub lyrics_retry: Duration,
    pub request_poll: Duration,
    pub request_give_up: Duration,
}

impl Default for Timings {
    fn default() -> Self {
        let ms = Duration::from_millis;
        Timings {
            poll: ms(5000),
            retry: ms(15000),
            tick: ms(250),
            timeout: ms(10000),
            lyrics_retry: ms(5000),
            request_poll: ms(3000),
            request_give_up: ms(120000),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SubwaveSnapshot {
    pub active: bool,
    pub now: Value,
    pub cover_url: String,
    pub online: bool,
    pub peeked: Map<String, Value>,
    pub lyrics: Value,
    pub lyrics_state: String,
    pub like_enabled: bool,
    pub liked: bool,
    pub like_blocked_ms: i64,
    pub request: Value,
    pub started_at: i64,
    pub server_at: i64,
    pub local_at: i64,
    pub buffer_ms: i64,
}

impl SubwaveSnapshot {
    /// What the listener hears: ms since the track started on the server's clock, less the stream buffer.
    pub fn elapsed_ms(&self, local_now: i64) -> i64 {
        sw_elapsed_ms(self.started_at, self.server_at, self.local_at, local_now, self.buffer_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubwaveEvent {
    Changed,
    /// Every `tick` while a station is active: time for elapsedChanged.
    Tick,
}

pub trait SubwaveListener: Send + Sync {
    fn on_event(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot);
}

enum Cmd {
    Open(String, String),
    Close,
    Probe(String),
    Like,
    Request(String, String),
}

pub struct SubwaveClient {
    tx: mpsc::UnboundedSender<Cmd>,
    cover: Arc<Mutex<Option<String>>>, // home page while active
}

pub fn local_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl SubwaveClient {
    pub fn new(listener: Arc<dyn SubwaveListener>, timings: Timings) -> SubwaveClient {
        let (tx, rx) = mpsc::unbounded_channel();
        let cover = Arc::new(Mutex::new(None));
        let c = cover.clone();
        std::thread::Builder::new()
            .name("re-la-subwave".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
                rt.block_on(Worker::new(listener, timings, c).run(rx));
            })
            .expect("subwave thread");
        SubwaveClient { tx, cover }
    }

    /// A station opened: `stream_url` may carry its password (userinfo or ?auth=); it never leaves the client.
    pub fn open(&self, home_page: String, stream_url: String) {
        let _ = self.tx.send(Cmd::Open(home_page, stream_url));
    }
    pub fn close(&self) {
        let _ = self.tx.send(Cmd::Close);
    }
    /// One /now-playing per home page and session, for the Radio page.
    pub fn probe(&self, home_page: String) {
        let _ = self.tx.send(Cmd::Probe(home_page));
    }
    pub fn like(&self) {
        let _ = self.tx.send(Cmd::Like);
    }
    pub fn send_request(&self, text: String, name: String) {
        let _ = self.tx.send(Cmd::Request(text, name));
    }
    /// Cover of a track on the active station; "" when none is active.
    pub fn cover_url(&self, subsonic_id: &str) -> String {
        let home = self.cover.lock().unwrap().clone();
        home.and_then(|h| sw_api(&h, &format!("/cover/{subsonic_id}"))).map(|u| u.to_string()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fail {
    NetDown, // no answer at all
    Bad,     // answered, but not with a JSON object (or an error status)
}

/// An HTTP answer: status (0 = none), the parsed body, Retry-After.
struct Http {
    status: u16,
    body: Result<Value, Fail>,
    raw: Option<Value>, // the JSON object even on an error status (refusal messages)
    retry_after: Option<String>,
}

enum Reply {
    Now { generation: u64, first: bool, r: Http },
    State { generation: u64, np: Value, local_at: i64, r: Http },
    Probe { home: String, r: Http },
    Lyrics { generation: u64, id: String, r: Http },
    Like { generation: u64, id: String, r: Http },
    LikePost { generation: u64, r: Http },
    RequestPost { seq: u64, r: Http },
    RequestPoll { seq: u64, r: Http },
}

async fn fetch(req: reqwest::RequestBuilder) -> Http {
    match req.send().await {
        Err(_) => Http { status: 0, body: Err(Fail::NetDown), raw: None, retry_after: None },
        Ok(resp) => {
            let status = resp.status().as_u16();
            let retry_after = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(str::to_string);
            let bytes = resp.bytes().await.unwrap_or_default();
            let raw = serde_json::from_slice::<Value>(&bytes).ok().filter(Value::is_object);
            let body = match &raw {
                Some(v) if status < 400 => Ok(v.clone()),
                _ => Err(Fail::Bad),
            };
            Http { status, body, raw, retry_after }
        }
    }
}

struct Worker {
    l: Arc<dyn SubwaveListener>,
    t: Timings,
    cover: Arc<Mutex<Option<String>>>,
    net: reqwest::Client,
    station_net: reqwest::Client, // password header, never redirected to another origin
    replies: (mpsc::UnboundedSender<Reply>, mpsc::UnboundedReceiver<Reply>),
    generation: u64, // bumped on open/close: a late reply for an older station is dropped
    home: String,
    password: String,
    active: bool,
    online: bool,
    now: Value,
    peeked: Map<String, Value>,
    probed: HashSet<String>,
    track_id: String,
    lyrics: Value,
    lyrics_state: String,
    request: Value,
    request_id: String,
    request_seq: u64,
    request_deadline: Instant,
    like_enabled: bool,
    liked: bool,
    like_blocked_ms: i64,
    server_at: i64,
    local_at: i64,
    buffer_ms: i64,
    poll_at: Option<Instant>,
    tick_at: Option<Instant>,
    lyrics_at: Option<Instant>,
    request_at: Option<Instant>,
}

fn far() -> Instant {
    Instant::now() + Duration::from_secs(365 * 24 * 3600)
}

impl Worker {
    fn new(l: Arc<dyn SubwaveListener>, t: Timings, cover: Arc<Mutex<Option<String>>>) -> Worker {
        let net = crate::http_client().timeout(t.timeout).build().expect("http client");
        let station_net = crate::http_client()
            .timeout(t.timeout)
            .redirect(reqwest::redirect::Policy::custom(|a| {
                let same = a.previous().first().is_some_and(|p| p.origin() == a.url().origin());
                if same && a.previous().len() < 10 { a.follow() } else { a.stop() }
            }))
            .build()
            .expect("http client");
        Worker {
            l,
            t,
            cover,
            net,
            station_net,
            replies: mpsc::unbounded_channel(),
            generation: 0,
            home: String::new(),
            password: String::new(),
            active: false,
            online: true,
            now: json!({}),
            peeked: Map::new(),
            probed: HashSet::new(),
            track_id: String::new(),
            lyrics: json!({}),
            lyrics_state: String::new(),
            request: json!({}),
            request_id: String::new(),
            request_seq: 0,
            request_deadline: Instant::now(),
            like_enabled: false,
            liked: false,
            like_blocked_ms: 0,
            server_at: 0,
            local_at: 0,
            buffer_ms: 0,
            poll_at: None,
            tick_at: None,
            lyrics_at: None,
            request_at: None,
        }
    }

    fn snapshot(&self) -> SubwaveSnapshot {
        let id = self.now["id"].as_str().unwrap_or("");
        let cover_url = if self.active && !id.is_empty() {
            sw_api(&self.home, &format!("/cover/{id}")).map(|u| u.to_string()).unwrap_or_default()
        } else {
            String::new()
        };
        SubwaveSnapshot {
            active: self.active,
            now: self.now.clone(),
            cover_url,
            online: self.online,
            peeked: self.peeked.clone(),
            lyrics: self.lyrics.clone(),
            lyrics_state: self.lyrics_state.clone(),
            like_enabled: self.like_enabled,
            liked: self.liked,
            like_blocked_ms: self.like_blocked_ms,
            request: self.request.clone(),
            started_at: self.now["startedAt"].as_i64().unwrap_or(0),
            server_at: self.server_at,
            local_at: self.local_at,
            buffer_ms: self.buffer_ms,
        }
    }

    fn emit(&self, e: SubwaveEvent) {
        *self.cover.lock().unwrap() = self.active.then(|| self.home.clone());
        self.l.on_event(e, self.snapshot());
    }

    fn changed(&self) {
        self.emit(SubwaveEvent::Changed);
    }

    fn spawn(&self, f: impl Future<Output = Reply> + Send + 'static) {
        let tx = self.replies.0.clone();
        tokio::spawn(async move {
            let _ = tx.send(f.await);
        });
    }

    fn get(&self, url: url::Url) -> reqwest::RequestBuilder {
        self.net.get(url)
    }

    /// GET with the station password as a header (none when empty), same-origin redirects only.
    fn station_get(&self, url: url::Url) -> reqwest::RequestBuilder {
        let req = self.station_net.get(url);
        if self.password.is_empty() { req } else { req.header("x-station-auth", &self.password) }
    }

    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        loop {
            let at = |o: Option<Instant>| o.unwrap_or_else(far);
            tokio::select! {
                cmd = rx.recv() => match cmd {
                    None => return,
                    Some(Cmd::Open(home, stream)) => self.open(home, &stream),
                    Some(Cmd::Close) => self.close(),
                    Some(Cmd::Probe(home)) => self.probe(home),
                    Some(Cmd::Like) => self.like(),
                    Some(Cmd::Request(text, name)) => self.send_request(&text, &name),
                },
                Some(r) = self.replies.1.recv() => self.reply(r),
                _ = sleep_until(at(self.poll_at)) => { self.poll_at = None; let first = !self.active; self.poll(first) }
                _ = sleep_until(at(self.tick_at)) => self.tick(),
                _ = sleep_until(at(self.lyrics_at)) => { self.lyrics_at = None; self.fetch_lyrics() }
                _ = sleep_until(at(self.request_at)) => { self.request_at = None; self.poll_request() }
            }
        }
    }

    fn tick(&mut self) {
        self.tick_at = Some(Instant::now() + self.t.tick);
        self.emit(SubwaveEvent::Tick);
        if self.like_blocked_ms > 0 {
            self.like_blocked_ms -= self.t.tick.as_millis() as i64;
            if self.like_blocked_ms <= 0 {
                self.like_blocked_ms = 0;
                self.changed();
            }
        }
    }

    fn clear(&mut self) {
        self.generation += 1;
        self.poll_at = None;
        self.tick_at = None;
        self.active = false;
        self.online = true;
        self.now = json!({});
        self.server_at = 0;
        self.local_at = 0;
        self.buffer_ms = 0;
        self.track_id.clear();
        self.lyrics = json!({});
        self.lyrics_state.clear();
        self.lyrics_at = None;
        self.like_enabled = false;
        self.liked = false;
        self.like_blocked_ms = 0;
        self.request_seq += 1;
        self.request_at = None;
        self.request = json!({});
    }

    fn open(&mut self, home: String, stream_url: &str) {
        self.clear();
        self.home = home;
        self.password = sw_password(stream_url);
        self.changed();
        self.poll(true);
    }

    fn close(&mut self) {
        let was = self.active || !self.home.is_empty();
        self.clear();
        self.home.clear();
        self.password.clear();
        if was {
            self.changed();
        }
    }

    /// first: the station is not known to be SUB/WAVE yet. A page that answers with anything else is a plain radio
    /// and is left alone; a dead connection is retried.
    fn poll(&mut self, first: bool) {
        let Some(url) = sw_api(&self.home, "/now-playing") else { return };
        let (generation, req) = (self.generation, self.get(url));
        self.spawn(async move { Reply::Now { generation, first, r: fetch(req).await } });
    }

    fn probe(&mut self, home: String) {
        let Some(url) = sw_api(&home, "/now-playing") else { return };
        if !self.probed.insert(home.clone()) {
            return;
        }
        let req = self.get(url);
        self.spawn(async move { Reply::Probe { home, r: fetch(req).await } });
    }

    /// 401: private station, 403: lyrics off, 503 or no connection: try again (never shown as "no lyrics").
    fn fetch_lyrics(&mut self) {
        self.lyrics_at = None;
        let id = self.track_id.clone();
        let url = sw_api(&self.home, &format!("/lyrics/current?songId={id}"));
        self.lyrics = json!({});
        self.lyrics_state = if id.is_empty() || url.is_none() { String::new() } else { "loading".into() };
        self.changed();
        let Some(url) = url.filter(|_| !self.lyrics_state.is_empty()) else { return };
        let (generation, req) = (self.generation, self.station_get(url));
        self.spawn(async move { Reply::Lyrics { generation, id, r: fetch(req).await } });
    }

    fn fetch_like(&mut self) {
        self.liked = false;
        let Some(url) = sw_api(&self.home, "/like") else { return };
        let (generation, id, req) = (self.generation, self.track_id.clone(), self.get(url));
        self.spawn(async move { Reply::Like { generation, id, r: fetch(req).await } });
    }

    fn like(&mut self) {
        let url = sw_api(&self.home, "/like");
        if !self.active || !self.like_enabled || self.like_blocked_ms > 0 || self.track_id.is_empty() {
            return;
        }
        let Some(url) = url else { return };
        let (generation, req) = (self.generation, self.net.post(url).json(&json!({"songId": self.track_id})));
        self.spawn(async move { Reply::LikePost { generation, r: fetch(req).await } });
    }

    fn send_request(&mut self, text: &str, name: &str) {
        let trimmed = text.trim().to_string();
        let url = sw_api(&self.home, "/request");
        if !self.active || trimmed.is_empty() || self.request["state"] == "pending" {
            return; // one at a time
        }
        let Some(url) = url else { return };
        self.request_seq += 1;
        self.request_at = None;
        self.request = json!({"state": "pending"});
        self.changed();
        let (seq, req) = (self.request_seq, self.net.post(url).json(&json!({"text": trimmed, "name": name.trim()})));
        self.spawn(async move { Reply::RequestPost { seq, r: fetch(req).await } });
    }

    /// Follows a request until it has an answer: stops on a final state, on 404 (unknown id) and at the give-up time.
    fn poll_request(&mut self) {
        let Some(url) = sw_api(&self.home, &format!("/request/{}", self.request_id)) else { return };
        let (seq, req) = (self.request_seq, self.get(url));
        self.spawn(async move { Reply::RequestPoll { seq, r: fetch(req).await } });
    }

    fn reply(&mut self, reply: Reply) {
        match reply {
            Reply::Now { generation, first, r } => {
                if generation != self.generation || (first && r.body == Err(Fail::Bad) && r.status < 500 && r.status != 429) {
                    return;
                }
                match r.body {
                    Ok(np) if sw_detect(&np) => {
                        let local_at = local_now();
                        let Some(url) = sw_api(&self.home, "/state") else { return };
                        let req = self.get(url);
                        self.spawn(async move { Reply::State { generation, np, local_at, r: fetch(req).await } });
                    }
                    Ok(_) if first => {} // a plain radio: left alone
                    _ => {
                        if self.online {
                            self.online = false;
                            self.changed();
                        }
                        self.poll_at = Some(Instant::now() + self.t.retry);
                    }
                }
            }
            Reply::State { generation, np, local_at, r } => {
                if generation != self.generation {
                    return;
                }
                // a failed /state only costs the start time and the upcoming list
                let now = sw_now(&np, &r.body.unwrap_or_else(|_| json!({})));
                let (was_active, was_online) = (self.active, self.online);
                self.active = true;
                self.online = true;
                self.local_at = local_at;
                self.server_at = sw_server_at(&np);
                self.buffer_ms = sw_buffer_ms(&np);
                if now != self.now || !was_active || !was_online {
                    self.now = now;
                    self.changed();
                }
                if !was_active {
                    self.tick_at = Some(Instant::now() + self.t.tick);
                }
                let id = self.now["id"].as_str().unwrap_or("").to_string();
                if id != self.track_id {
                    self.track_id = id;
                    self.fetch_lyrics();
                    self.fetch_like();
                }
                self.poll_at = Some(Instant::now() + self.t.poll);
            }
            Reply::Probe { home, r } => {
                if r.body == Err(Fail::NetDown) {
                    self.probed.remove(&home); // try again on the next visit
                    return;
                }
                let peek = match r.body {
                    Ok(np) if sw_detect(&np) => {
                        let n = sw_now(&np, &json!({}));
                        json!({"title": n["title"], "artist": n["artist"], "dj": n["dj"]})
                    }
                    _ => json!({}),
                };
                self.peeked.insert(home, peek);
                self.changed();
            }
            Reply::Lyrics { generation, id, r } => {
                if generation != self.generation || id != self.track_id {
                    return;
                }
                self.lyrics_state = match (r.status, &r.body) {
                    (401, _) => "auth".into(),
                    (403, _) => "off".into(),
                    (503, _) | (_, Err(Fail::NetDown)) => "retry".into(),
                    (_, Err(_)) => "none".into(),
                    (_, Ok(obj)) => {
                        self.lyrics = sw_lyrics(obj);
                        if self.lyrics["lines"].as_array().is_some_and(|l| !l.is_empty()) {
                            "ok".into()
                        } else if self.lyrics["pending"] == true {
                            "retry".into()
                        } else {
                            "none".into()
                        }
                    }
                };
                if self.lyrics_state == "retry" {
                    self.lyrics_at = Some(Instant::now() + self.t.lyrics_retry);
                }
                self.changed();
            }
            Reply::Like { generation, id, r } => {
                if generation != self.generation || id != self.track_id {
                    return;
                }
                match r.body {
                    Ok(obj) => {
                        self.like_enabled = obj["enabled"].as_bool().unwrap_or(false);
                        self.liked = obj["liked"].as_bool().unwrap_or(false);
                    }
                    Err(_) if r.status == 403 => self.like_enabled = false,
                    Err(_) => {}
                }
                self.changed();
            }
            Reply::LikePost { generation, r } => {
                if generation != self.generation {
                    return;
                }
                match r.status {
                    403 => self.like_enabled = false,
                    429 => {
                        let secs = r.retry_after.and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
                        self.like_blocked_ms = secs.clamp(1, 86_400) * 1000;
                    }
                    409 => return self.fetch_like(), // the track changed under us
                    200 => self.liked = r.body.ok().and_then(|o| o["liked"].as_bool()).unwrap_or(true),
                    _ => {}
                }
                self.changed();
            }
            Reply::RequestPost { seq, r } => {
                if seq != self.request_seq {
                    return;
                }
                let obj = r.body.clone().unwrap_or_else(|_| json!({}));
                self.request_id = obj["requestId"].as_str().unwrap_or("").to_string();
                let success = obj["success"].as_bool().unwrap_or(true);
                if r.status >= 400 || r.status == 0 || !success || self.request_id.is_empty() {
                    self.request = json!({"state": "failed", "message": r.message()});
                    self.changed();
                    return;
                }
                self.request_deadline = Instant::now() + self.t.request_give_up;
                self.request_at = Some(Instant::now() + self.t.request_poll);
            }
            Reply::RequestPoll { seq, r } => {
                if seq != self.request_seq {
                    return;
                }
                if r.status == 404 {
                    self.request = json!({"state": "unknown"});
                    self.changed();
                    return;
                }
                if let Ok(obj) = &r.body {
                    self.request = sw_request(obj);
                    self.changed();
                    if self.request["state"] != "pending" {
                        return;
                    }
                }
                if Instant::now() < self.request_deadline {
                    self.request_at = Some(Instant::now() + self.t.request_poll);
                } else if self.request["state"] == "pending" {
                    self.request = json!({"state": "unknown"});
                    self.changed();
                }
            }
        }
    }
}

impl Http {
    /// The server's message on a refused request: `message`, else `error`, else "".
    fn message(&self) -> String {
        self.raw.as_ref().map_or(String::new(), |o| {
            o["message"].as_str().or_else(|| o["error"].as_str()).unwrap_or("").to_string()
        })
    }
}
