//! SyncClient against the real relay (relay/), with a stub Navidrome that accepts token "good" only.
use axum::Router;
use axum::extract::Query;
use axum::routing::get;
use rela_core::sync::{SyncClient, SyncEvent, SyncListener, SyncSnapshot};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A relay on its own runtime: dropping it kills the relay and every connection, like a server restart.
struct Relay {
    rt: Option<tokio::runtime::Runtime>,
    port: u16,
    pings: Arc<AtomicUsize>,
}

impl Relay {
    fn start(port: u16, pings: Arc<AtomicUsize>) -> Relay {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let p = pings.clone();
        let port = rt.block_on(async move {
            let stub = Router::new().route(
                "/rest/ping",
                get(move |Query(q): Query<HashMap<String, String>>| {
                    p.fetch_add(1, Ordering::SeqCst);
                    async move {
                        axum::Json(if q.get("t").map(String::as_str) == Some("good") {
                            json!({"subsonic-response": {"status": "ok"}})
                        } else {
                            json!({"subsonic-response": {"status": "failed", "error": {"code": 40}}})
                        })
                    }
                }),
            );
            let sl = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let navidrome = format!("http://{}", sl.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(sl, stub).await.unwrap() });
            let rl = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
            let port = rl.local_addr().unwrap().port();
            tokio::spawn(async move { axum::serve(rl, re_with::app(re_with::Config { navidrome, waveforms: None, status_socket: None })).await.unwrap() });
            port
        });
        Relay { rt: Some(rt), port, pings }
    }
    fn url(&self) -> String {
        format!("ws://127.0.0.1:{}/", self.port)
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.rt.take().unwrap().shutdown_background();
    }
}

struct Collect(Mutex<Sender<(SyncEvent, SyncSnapshot)>>);
impl SyncListener for Collect {
    fn on_event(&self, event: SyncEvent, snapshot: SyncSnapshot) {
        let _ = self.0.lock().unwrap().send((event, snapshot));
    }
}

fn client(allow: bool) -> (SyncClient, Receiver<(SyncEvent, SyncSnapshot)>) {
    let (tx, rx) = channel();
    (SyncClient::new(allow, false, Arc::new(Collect(Mutex::new(tx)))), rx)
}

/// Reads events until one matches; panics after `secs`.
fn wait_for(rx: &Receiver<(SyncEvent, SyncSnapshot)>, secs: u64, what: &str, pred: impl Fn(&SyncEvent, &SyncSnapshot) -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    loop {
        let left = end.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok((e, s)) if pred(&e, &s) => return,
            Ok(_) => {}
            Err(_) => panic!("timed out waiting for {what}"),
        }
    }
}

fn lists(s: &SyncSnapshot, user: &str) -> bool {
    s.users.iter().any(|u| u["user"] == user)
}

fn state(qrev: i64) -> Map<String, Value> {
    json!({"qrev": qrev, "queue": [{"id": "x"}], "index": 0, "playing": true, "anchor": 0, "position": 0, "xfade": 0})
        .as_object()
        .unwrap()
        .clone()
}

#[test]
fn two_clients_see_each_other() {
    let relay = Relay::start(0, Default::default());
    let (a, ra) = client(false);
    let (b, rb) = client(false);
    a.start(relay.url(), "A".into(), "good".into(), "s".into());
    b.start(relay.url(), "B".into(), "good".into(), "s".into());
    wait_for(&ra, 5, "A lists B", |_, s| lists(s, "B"));
    wait_for(&rb, 5, "B lists A", |_, s| lists(s, "A"));
}

#[test]
fn follow_delivers_state_and_cmd() {
    let relay = Relay::start(0, Default::default());
    let (a, ra) = client(true);
    let (b, rb) = client(false);
    a.start(relay.url(), "A".into(), "good".into(), "s".into());
    wait_for(&ra, 5, "A connected", |e, _| *e == SyncEvent::Connected);
    a.publish(state(1));
    b.start(relay.url(), "B".into(), "good".into(), "s".into());
    wait_for(&rb, 5, "B lists A", |_, s| lists(s, "A"));
    b.follow("A");
    wait_for(&rb, 5, "B gets A's state", |e, s| {
        matches!(e, SyncEvent::StateReceived(m) if m["qrev"] == 1) && s.following == "A" && s.can_control
    });
    b.send("next", json!({}));
    wait_for(&ra, 5, "A gets B's command", |e, _| matches!(e, SyncEvent::CommandReceived { op, .. } if op == "next"));
}

#[test]
fn reconnects_and_refollows() {
    let relay = Relay::start(0, Default::default());
    let port = relay.port;
    let (a, ra) = client(false);
    let (b, rb) = client(false);
    a.start(relay.url(), "A".into(), "good".into(), "s".into());
    wait_for(&ra, 5, "A connected", |e, _| *e == SyncEvent::Connected);
    a.publish(state(1));
    b.start(relay.url(), "B".into(), "good".into(), "s".into());
    wait_for(&rb, 5, "B lists A", |_, s| lists(s, "A"));
    b.follow("A");
    wait_for(&ra, 5, "A sees B listening", |_, s| s.listeners == ["B"]);

    drop(relay); // server restart: every session is gone
    wait_for(&rb, 5, "B offline", |_, s| !s.available);
    let _relay = Relay::start(port, Default::default());
    wait_for(&rb, 20, "B back", |e, _| *e == SyncEvent::Connected);
    wait_for(&ra, 20, "A sees B listening again", |_, s| s.available && s.listeners == ["B"]);
}

#[test]
fn client_stops_after_auth_error() {
    let pings = Arc::new(AtomicUsize::new(0));
    let relay = Relay::start(0, pings.clone());
    let (a, ra) = client(false);
    a.start(relay.url(), "A".into(), "bad".into(), "s".into());
    wait_for(&ra, 5, "auth failure", |e, _| *e == SyncEvent::Failed("auth".into()));
    std::thread::sleep(Duration::from_secs(3)); // the backoff would have retried by now (1 s, 2 s)
    assert_eq!(relay.pings.load(Ordering::SeqCst), 1);
}

#[test]
fn stop_with_bye_reaches_relay() {
    let relay = Relay::start(0, Default::default());
    let (a, ra) = client(false);
    let (b, rb) = client(false);
    a.start(relay.url(), "A".into(), "good".into(), "s".into());
    wait_for(&ra, 5, "A connected", |e, _| *e == SyncEvent::Connected);
    a.publish(state(1));
    b.start(relay.url(), "B".into(), "good".into(), "s".into());
    wait_for(&rb, 5, "B lists A", |_, s| lists(s, "A"));
    b.follow("A");
    wait_for(&rb, 5, "B follows A", |_, s| s.following == "A");
    // Without a bye the relay keeps A's followers for its grace period (60 s); with it they hear at once.
    a.stop(true);
    wait_for(&rb, 1, "B told the host left", |e, _| *e == SyncEvent::Ended("host-left".into()));
}

#[test]
fn now_follows_the_relay_clock() {
    let relay = Relay::start(0, Default::default());
    let (a, ra) = client(false);
    a.start(relay.url(), "A".into(), "good".into(), "s".into());
    wait_for(&ra, 5, "A connected", |e, _| *e == SyncEvent::Connected);
    let local = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64;
    assert!((a.now() - local).abs() < 1000.0); // same machine: the relay clock is ours, give or take
}

// The relay pings every 20 s and drops a client that misses pongs: an idle client must still answer.
#[test]
fn answers_relay_pings() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let (pong_tx, pong_rx) = channel();
    let port = rt.block_on(async move {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (tcp, _) = l.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let _hello = ws.next().await;
            ws.send(Message::text(r#"{"type":"welcome","you":"a","now":0}"#)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await; // let the clock replies settle: idle from here
            ws.send(Message::Ping(b"x".to_vec().into())).await.unwrap();
            while let Some(Ok(m)) = ws.next().await {
                if matches!(m, Message::Pong(_)) {
                    let _ = pong_tx.send(());
                    return;
                }
            }
        });
        port
    });
    let (a, ra) = client(false);
    a.start(format!("ws://127.0.0.1:{port}/"), "a".into(), "t".into(), "s".into());
    wait_for(&ra, 5, "connected", |e, _| *e == SyncEvent::Connected);
    pong_rx.recv_timeout(Duration::from_secs(2)).expect("no pong for the relay's ping");
}
