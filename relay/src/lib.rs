//! WebSocket front of the hub: auth on the first message, then every message goes through `Hub`.
#![forbid(unsafe_code)]
pub mod auth;
pub mod hub;
pub mod status;
pub mod waveform;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use hub::{ConnId, Hub, Out};
use rela_proto::In;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::AbortHandle;
use tokio::time::{Instant, interval, interval_at, timeout};

pub const MAX_MESSAGE: usize = 1 << 20;
const PING_EVERY: Duration = Duration::from_secs(20);
/// Messages waiting for one slow socket. More than that and the connection is dropped, so a client that stops
/// reading never holds the relay's memory (a state can be up to 1 MB).
const OUTBOX: usize = 64;

pub struct Config {
    pub navidrome: String,
    /// None: no waveforms (the endpoint answers 404).
    pub waveforms: Option<waveform::WaveformConfig>,
    /// Where `re-with status` reads the state; None: nowhere.
    pub status_socket: Option<std::path::PathBuf>,
}

struct Shared {
    hub: Mutex<Hub>,
    conns: Mutex<HashMap<ConnId, (Sender<Message>, AbortHandle)>>, // outbox and its writer task
    next_id: AtomicU64,
    http: reqwest::Client,
    navidrome: String,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn text(v: serde_json::Value) -> Message {
    Message::Text(v.to_string().into())
}

fn error(code: &str) -> Message {
    text(json!({"type": "error", "code": code, "message": code}))
}

impl Shared {
    /// A full outbox drops the connection at once: its writer is aborted, so its loop ends and cleans up.
    fn dispatch(&self, out: Vec<Out>) {
        let mut conns = self.conns.lock().unwrap();
        for o in out {
            let (id, m) = match o {
                Out::To(id, v) => (id, text(v)),
                Out::Close(id) => (id, Message::Close(None)),
            };
            if conns.get(&id).is_some_and(|(tx, _)| tx.try_send(m).is_err_and(|e| matches!(e, TrySendError::Full(_))))
                && let Some((_, writer)) = conns.remove(&id)
            {
                writer.abort();
            }
        }
    }
}

/// Router with the WebSocket endpoint at `/` and waveforms at `/waveform/{id}` (Caddy strips the `/re-with`
/// prefix). Must be called inside a tokio runtime: it starts the grace-period ticker and the waveform scanner.
pub fn app(cfg: Config) -> Router {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().expect("http client");
    let progress = Arc::new(Mutex::new(status::Progress::default()));
    let store = cfg.waveforms.and_then(|w| match waveform::start(w, http.clone(), cfg.navidrome.clone(), progress.clone()) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("waveforms off: {e}");
            None
        }
    });
    let waveforms = waveform::route::router(store.clone(), http.clone(), cfg.navidrome.clone());
    let shared = Arc::new(Shared {
        hub: Mutex::new(Hub::new()),
        conns: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(1),
        http,
        navidrome: cfg.navidrome,
    });
    if let Some(path) = &cfg.status_socket {
        let (s, started) = (shared.clone(), std::time::Instant::now());
        let snapshot = move || {
            let (online, groups) = s.hub.lock().unwrap().summary();
            let stored = store.as_ref().map(|st| st.lock().unwrap().ids().len());
            json!({
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_s": started.elapsed().as_secs(),
                "rss_kb": status::rss_kb(),
                "sync": {"online": online, "groups": groups},
                "waveforms": progress.lock().unwrap().json(stored),
            })
        };
        if let Err(e) = status::serve(path, snapshot) {
            eprintln!("status off: {}: {e}", path.display());
        }
    }
    let ticker = shared.clone();
    tokio::spawn(async move {
        let mut every = interval(Duration::from_secs(1));
        loop {
            every.tick().await;
            let out = ticker.hub.lock().unwrap().tick(now());
            ticker.dispatch(out);
        }
    });
    Router::new().route("/", get(upgrade)).with_state(shared).merge(waveforms)
}

async fn upgrade(State(s): State<Arc<Shared>>, ws: WebSocketUpgrade) -> Response {
    // Frames up to 2 MB are read so an oversize message gets a "too-large" answer instead of a dropped socket.
    ws.max_message_size(2 * MAX_MESSAGE).on_upgrade(move |socket| connection(s, socket))
}

async fn reject(tx: &mut futures_util::stream::SplitSink<WebSocket, Message>, code: &str) {
    let _ = tx.send(error(code)).await;
    let _ = tx.send(Message::Close(None)).await;
}

async fn connection(s: Arc<Shared>, socket: WebSocket) {
    let (mut tx, mut rx) = socket.split();
    let first = match timeout(Duration::from_secs(10), rx.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => serde_json::from_str::<In>(&t).ok(),
        _ => None,
    };
    let Some(In::Hello { v, u, t, s: salt, hidden }) = first else { return reject(&mut tx, "auth").await };
    if v != 1 {
        return reject(&mut tx, "version").await;
    }
    match auth::ping(&s.http, &s.navidrome, &u, &t, &salt).await {
        auth::Auth::Ok => {}
        auth::Auth::Denied => return reject(&mut tx, "auth").await,
        auth::Auth::Unavailable => return reject(&mut tx, "unavailable").await, // client retries
    }
    drop((t, salt));

    let id = s.next_id.fetch_add(1, Ordering::Relaxed);
    let (out_tx, mut out_rx) = channel::<Message>(OUTBOX);
    let mut writer = tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            let close = matches!(m, Message::Close(_));
            if tx.send(m).await.is_err() || close {
                break;
            }
        }
    });
    s.conns.lock().unwrap().insert(id, (out_tx, writer.abort_handle()));
    let out = s.hub.lock().unwrap().join(id, &u, hidden, now());
    s.dispatch(out);

    let mut ping = interval_at(Instant::now() + PING_EVERY, PING_EVERY);
    let mut missed = 0;
    loop {
        tokio::select! {
            m = rx.next() => match m {
                Some(Ok(Message::Text(t))) => {
                    if t.len() > MAX_MESSAGE {
                        s.dispatch(vec![Out::To(id, json!({"type": "error", "code": "too-large", "message": "too-large"}))]);
                        continue;
                    }
                    let Ok(msg) = serde_json::from_str::<In>(&t) else { continue }; // unknown or malformed: ignored
                    let out = s.hub.lock().unwrap().handle(id, msg, now());
                    s.dispatch(out);
                }
                Some(Ok(Message::Pong(_))) => missed = 0,
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            _ = ping.tick() => {
                if missed >= 2 {
                    break;
                }
                missed += 1;
                match s.conns.lock().unwrap().get(&id) {
                    Some((c, _)) => drop(c.try_send(Message::Ping(Default::default()))),
                    None => break,
                }
            }
            _ = &mut writer => break, // closed by the hub (replaced) or the peer went away
        }
    }
    writer.abort(); // it may be stuck on a peer that stopped reading
    s.conns.lock().unwrap().remove(&id);
    let out = s.hub.lock().unwrap().disconnect(id, now());
    s.dispatch(out);
}
