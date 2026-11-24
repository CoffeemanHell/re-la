//! End to end: stub Navidrome + the real relay router + real WebSocket clients.
use axum::{Json, Router, extract::Query, routing::get};
use futures_util::{SinkExt, StreamExt};
use re_with::{Config, app};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn serve(router: Router) -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, router).await.unwrap() });
    addr.to_string()
}

/// Relay wired to a stub Navidrome that accepts token "good" only. Returns the ws:// url.
async fn relay() -> String {
    let stub = Router::new().route(
        "/rest/ping",
        get(|Query(q): Query<HashMap<String, String>>| async move {
            Json(if q.get("t").map(String::as_str) == Some("good") {
                json!({"subsonic-response": {"status": "ok"}})
            } else {
                json!({"subsonic-response": {"status": "failed", "error": {"code": 40}}})
            })
        }),
    );
    let navidrome = format!("http://{}", serve(stub).await);
    format!("ws://{}/", serve(app(Config { navidrome, waveforms: None, status_socket: None })).await)
}

async fn send(ws: &mut Ws, v: Value) {
    ws.send(Message::text(v.to_string())).await.unwrap();
}

/// Next JSON message, skipping "users" broadcasts. None when the socket closed.
async fn next(ws: &mut Ws) -> Option<Value> {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next()).await.expect("timed out")?;
        match m.ok()? {
            Message::Text(t) => {
                // every frame the relay sends must be a protocol message the clients can parse
                serde_json::from_str::<rela_proto::Out>(&t).unwrap_or_else(|e| panic!("not an Out: {t}: {e}"));
                let v: Value = serde_json::from_str(&t).unwrap();
                if v["type"] != "users" {
                    return Some(v);
                }
            }
            Message::Close(_) => return None,
            _ => {}
        }
    }
}

async fn login(url: &str, user: &str) -> Ws {
    let (mut ws, _) = connect_async(url).await.unwrap();
    send(&mut ws, json!({"type": "hello", "v": 1, "u": user, "t": "good", "s": "salt"})).await;
    assert_eq!(next(&mut ws).await.unwrap()["type"], "welcome");
    ws
}

#[tokio::test]
async fn hello_ok_gets_welcome() {
    let url = relay().await;
    login(&url, "ali").await;
}

#[tokio::test]
async fn bad_token_gets_auth_error() {
    let url = relay().await;
    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, json!({"type": "hello", "v": 1, "u": "ali", "t": "bad", "s": "salt"})).await;
    assert_eq!(next(&mut ws).await.unwrap()["code"], "auth");
    assert!(next(&mut ws).await.is_none());
}

#[tokio::test]
async fn first_message_must_be_hello() {
    let url = relay().await;
    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, json!({"type": "follow", "user": "x"})).await;
    assert_eq!(next(&mut ws).await.unwrap()["code"], "auth");
    assert!(next(&mut ws).await.is_none());
}

#[tokio::test]
async fn wrong_version_rejected() {
    let url = relay().await;
    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, json!({"type": "hello", "v": 2, "u": "ali", "t": "good", "s": "salt"})).await;
    assert_eq!(next(&mut ws).await.unwrap()["code"], "version");
}

#[tokio::test]
async fn garbage_is_ignored() {
    let url = relay().await;
    let mut ws = login(&url, "ali").await;
    ws.send(Message::text("not json")).await.unwrap();
    send(&mut ws, json!({"type": "nope"})).await;
    send(&mut ws, json!({"type": "time", "c": 1.0})).await;
    let t = next(&mut ws).await.unwrap();
    assert_eq!(t["type"], "time");
    assert_eq!(t["c"], 1.0);
}

#[tokio::test]
async fn follow_delivers_state_and_cmd() {
    let url = relay().await;
    let mut a = login(&url, "a").await;
    let mut b = login(&url, "b").await;
    send(&mut a, json!({"type": "state", "qrev": 1, "queue": [{"id": "x"}], "index": 0, "playing": true,
                        "anchor": 0, "position": 0, "xfade": 0, "allowControl": true})).await;
    send(&mut b, json!({"type": "follow", "user": "a"})).await;
    assert_eq!(next(&mut b).await.unwrap()["type"], "following");
    let s = next(&mut b).await.unwrap();
    assert_eq!(s["type"], "state");
    assert_eq!(s["queue"][0]["id"], "x");
    send(&mut b, json!({"type": "cmd", "op": "next", "args": {}, "qrev": 1})).await;
    let c = next(&mut a).await.unwrap();
    assert_eq!(c["type"], "cmd");
    assert_eq!(c["from"], "b");
}

#[tokio::test]
async fn navidrome_down_is_not_auth_error() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let navidrome = format!("http://{}", closed.local_addr().unwrap());
    drop(closed); // nothing listens there now
    let url = format!("ws://{}/", serve(app(Config { navidrome, waveforms: None, status_socket: None })).await);
    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, json!({"type": "hello", "v": 1, "u": "ali", "t": "good", "s": "salt"})).await;
    assert_eq!(next(&mut ws).await.unwrap()["code"], "unavailable");
}

#[tokio::test]
async fn second_login_replaces_first() {
    let url = relay().await;
    let mut first = login(&url, "ali").await;
    let _second = login(&url, "Ali").await;
    assert_eq!(next(&mut first).await.unwrap()["code"], "replaced");
    assert!(next(&mut first).await.is_none());
}

#[tokio::test]
async fn follower_that_stops_reading_is_dropped() {
    let url = relay().await;
    let a = login(&url, "a").await;
    let mut b = login(&url, "b").await;
    send(&mut b, json!({"type": "follow", "user": "a"})).await;
    assert_eq!(next(&mut b).await.unwrap()["type"], "following");
    // From here b reads nothing. Each ~400 KB state goes to b: its socket buffers fill, then its outbox.
    let (mut sink, mut stream) = a.split();
    let title = "x".repeat(400);
    let queue: Vec<Value> = (0..1000).map(|i| json!({"id": i.to_string(), "title": title})).collect();
    let flood = async move {
        for qrev in 0..400 {
            let st = json!({"type": "state", "qrev": qrev, "index": 0, "playing": true, "anchor": 0, "position": 0, "xfade": 0, "queue": queue});
            sink.send(Message::text(st.to_string())).await.unwrap();
        }
        std::future::pending::<()>().await
    };
    let dropped = async move {
        let mut seen = false; // the list from before b joined may still be unread
        while let Some(Ok(m)) = stream.next().await {
            if let Message::Text(t) = m {
                let v: Value = serde_json::from_str(&t).unwrap();
                if v["type"] != "users" {
                    continue;
                }
                let has_b = v["users"].as_array().unwrap().iter().any(|u| u["user"] == "b");
                if seen && !has_b {
                    return;
                }
                seen |= has_b;
            }
        }
        panic!("a closed");
    };
    let done = async { tokio::select! { _ = dropped => {}, _ = flood => {} } };
    tokio::time::timeout(Duration::from_secs(30), done).await.expect("b was never dropped");
    drop(b);
}
