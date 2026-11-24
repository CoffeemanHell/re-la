//! The relay connection: owns the socket and the timers on its own thread (tokio, current-thread runtime) and runs
//! every relay message and user action through `SyncMachine`. Front ends never block on it.
use super::machine::{Effect, SyncEvent, SyncMachine};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Map, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until, timeout};
use tokio_tungstenite::tungstenite::Message;

const CLOCK_EVERY: Duration = Duration::from_millis(30_000);
const REFOLLOW_EVERY: Duration = Duration::from_millis(2000);
const BACKOFF_MS: [u64; 4] = [1000, 2000, 5000, 10000];
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const BYE_WAIT: Duration = Duration::from_millis(500);

/// What the front end shows, taken right after each event.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncSnapshot {
    pub available: bool,
    pub users: Vec<Value>,
    pub listeners: Vec<String>,
    pub following: String,
    pub can_control: bool,
    pub allow_control: bool,
    pub hidden: bool,
}

/// Called on the client's thread.
pub trait SyncListener: Send + Sync {
    fn on_event(&self, event: SyncEvent, snapshot: SyncSnapshot);
}

#[derive(Clone)]
struct Login {
    url: String,
    user: String,
    token: String,
    salt: String,
}

enum Cmd {
    Start(Login),
    Stop { bye: bool, done: std::sync::mpsc::Sender<()> },
    Publish(Map<String, Value>),
    Follow(String),
    Unfollow,
    Send(String, Value),
    AllowControl(bool),
    Hidden(bool),
    Demo,
}

pub struct SyncClient {
    tx: mpsc::UnboundedSender<Cmd>,
    offset: Arc<AtomicI64>,
}

fn local_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl SyncClient {
    /// Idle until `start`. The thread ends when the client is dropped.
    pub fn new(allow_control: bool, hidden: bool, listener: Arc<dyn SyncListener>) -> SyncClient {
        let (tx, rx) = mpsc::unbounded_channel();
        let offset = Arc::new(AtomicI64::new(0));
        let off = offset.clone();
        std::thread::Builder::new()
            .name("re-la-sync".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
                rt.block_on(Worker { m: SyncMachine::new(allow_control, hidden), l: listener, offset: off }.run(rx));
            })
            .expect("sync thread");
        SyncClient { tx, offset }
    }

    pub fn start(&self, url: String, user: String, token: String, salt: String) {
        let _ = self.tx.send(Cmd::Start(Login { url, user, token, salt }));
    }

    /// Disconnect. With `bye`, followers hear at once that we left; returns once it is out (at most 500 ms).
    pub fn stop(&self, bye: bool) {
        let (done, wait) = std::sync::mpsc::channel();
        if self.tx.send(Cmd::Stop { bye, done }).is_ok() {
            let _ = wait.recv_timeout(BYE_WAIT);
        }
    }

    pub fn publish(&self, state: Map<String, Value>) {
        let _ = self.tx.send(Cmd::Publish(state));
    }
    pub fn follow(&self, user: &str) {
        let _ = self.tx.send(Cmd::Follow(user.into()));
    }
    pub fn unfollow(&self) {
        let _ = self.tx.send(Cmd::Unfollow);
    }
    pub fn send(&self, op: &str, args: Value) {
        let _ = self.tx.send(Cmd::Send(op.into(), args));
    }
    pub fn set_allow_control(&self, on: bool) {
        let _ = self.tx.send(Cmd::AllowControl(on));
    }
    pub fn set_hidden(&self, on: bool) {
        let _ = self.tx.send(Cmd::Hidden(on));
    }
    pub fn demo(&self) {
        let _ = self.tx.send(Cmd::Demo);
    }

    /// Relay clock in ms: the local clock plus the measured offset (0 before the first sample).
    pub fn now(&self) -> f64 {
        (local_now() + self.offset.load(Ordering::Relaxed)) as f64
    }
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// How a connection ended.
enum End {
    Lost,
    Stopped,
    GiveUp,
    Restart(Login),
}

struct Worker {
    m: SyncMachine,
    l: Arc<dyn SyncListener>,
    offset: Arc<AtomicI64>,
}

impl Worker {
    fn snapshot(&self) -> SyncSnapshot {
        let m = &self.m;
        SyncSnapshot {
            available: m.available(),
            users: m.users().to_vec(),
            listeners: m.listeners().to_vec(),
            following: m.following().to_string(),
            can_control: m.can_control(),
            allow_control: m.allow_control(),
            hidden: m.hidden(),
        }
    }

    /// Effects with no socket: events go out, the rest has nothing to act on.
    fn emit(&self, fx: Vec<Effect>) {
        self.offset.store(self.m.offset(), Ordering::Relaxed);
        for e in fx {
            if let Effect::Event(ev) = e {
                self.l.on_event(ev, self.snapshot());
            }
        }
    }

    /// A user action that needs no socket of its own. Returns the effects to apply.
    fn action(&mut self, cmd: Cmd) -> Vec<Effect> {
        match cmd {
            Cmd::Publish(s) => self.m.publish(s),
            Cmd::Follow(u) => self.m.follow(&u),
            Cmd::Unfollow => self.m.unfollow(),
            Cmd::Send(op, args) => self.m.send(&op, args),
            Cmd::AllowControl(on) => self.m.set_allow_control(on),
            Cmd::Hidden(on) => self.m.set_hidden(on),
            Cmd::Demo => self.m.demo(),
            Cmd::Start(_) | Cmd::Stop { .. } => vec![],
        }
    }

    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        let mut login: Option<Login> = None;
        let mut backoff = 0usize;
        loop {
            let Some(cur) = login.clone() else {
                // not started (or stopped / gave up): wait for a start, keep track of the rest
                match rx.recv().await {
                    None => return,
                    Some(Cmd::Start(l)) => {
                        login = Some(l);
                        backoff = 0;
                    }
                    Some(Cmd::Stop { done, .. }) => {
                        let fx = self.m.stop();
                        self.emit(fx);
                        let _ = done.send(());
                    }
                    Some(cmd) => {
                        let fx = self.action(cmd);
                        self.emit(fx);
                    }
                }
                continue;
            };
            let end = match timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(cur.url.as_str())).await {
                Ok(Ok((ws, _))) => self.session(ws, &cur, &mut rx, &mut backoff).await,
                _ => End::Lost,
            };
            let fx = self.m.disconnected();
            self.emit(fx);
            match end {
                End::Stopped | End::GiveUp => login = None,
                End::Restart(l) => {
                    login = Some(l);
                    backoff = 0;
                }
                End::Lost => {
                    // Socket gone or never came up: keep playing locally, try again 1 / 2 / 5 / 10 s later.
                    let wait = Instant::now() + Duration::from_millis(BACKOFF_MS[backoff.min(BACKOFF_MS.len() - 1)]);
                    backoff += 1;
                    loop {
                        tokio::select! {
                            _ = sleep_until(wait) => break,
                            cmd = rx.recv() => match cmd {
                                None => return,
                                Some(Cmd::Start(l)) => { login = Some(l); backoff = 0; break }
                                Some(Cmd::Stop { done, .. }) => {
                                    let fx = self.m.stop();
                                    self.emit(fx);
                                    let _ = done.send(());
                                    login = None;
                                    break;
                                }
                                Some(cmd) => { let fx = self.action(cmd); self.emit(fx); }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn session(&mut self, ws: Ws, login: &Login, rx: &mut mpsc::UnboundedReceiver<Cmd>, backoff: &mut usize) -> End {
        let (mut out, mut inc) = ws.split();
        let hello = self.m.hello(&login.user, &login.token, &login.salt);
        if out.send(Message::text(serde_json::to_string(&hello).unwrap_or_default())).await.is_err() {
            return End::Lost;
        }
        let far = || Instant::now() + Duration::from_secs(365 * 24 * 3600);
        let (mut clock_at, mut refollow_at) = (far(), far());
        loop {
            let fx = tokio::select! {
                msg = inc.next() => match msg {
                    Some(Ok(Message::Text(t))) => self.m.on_text(&t, local_now()),
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return End::Lost,
                    Some(Ok(_)) => continue, // ping/pong/binary
                },
                _ = sleep_until(clock_at) => { clock_at = Instant::now() + CLOCK_EVERY; self.m.clock_tick(local_now()) }
                _ = sleep_until(refollow_at) => { refollow_at = far(); self.m.refollow_tick() }
                cmd = rx.recv() => match cmd {
                    None => return End::Stopped,
                    Some(Cmd::Start(l)) => return End::Restart(l),
                    Some(Cmd::Stop { bye, done }) => {
                        if bye {
                            let _ = out.send(Message::text(serde_json::to_string(&rela_proto::In::Bye).unwrap_or_default())).await;
                            let _ = out.flush().await;
                        }
                        let _ = out.close().await;
                        let fx = self.m.stop();
                        self.emit(fx);
                        let _ = done.send(());
                        return End::Stopped;
                    }
                    Some(cmd) => self.action(cmd),
                }
            };
            self.offset.store(self.m.offset(), Ordering::Relaxed);
            let mut give_up = false;
            for e in fx {
                match e {
                    Effect::Send(m) => {
                        let text = serde_json::to_string(&m).unwrap_or_default();
                        if out.send(Message::text(text)).await.is_err() {
                            return End::Lost;
                        }
                    }
                    Effect::Event(ev) => {
                        if ev == SyncEvent::Connected {
                            *backoff = 0;
                        }
                        self.l.on_event(ev, self.snapshot());
                    }
                    Effect::StartClockTimer => clock_at = Instant::now() + CLOCK_EVERY,
                    Effect::StartRefollowTimer => refollow_at = Instant::now() + REFOLLOW_EVERY,
                    Effect::StopRefollowTimer => refollow_at = far(),
                    Effect::GiveUp => give_up = true,
                }
            }
            if give_up {
                let _ = out.close().await;
                return End::GiveUp;
            }
        }
    }
}
