//! The Subsonic client against a fake server (axum): token auth, failures, the QML guards.
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rela_core::queue::{NO_SERVER, NOT_SUBSONIC};
use rela_core::subsonic::{Login, Subsonic};
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<(String, String)>>>; // (method, raw query)

async fn handle(State(log): State<Log>, Path(method): Path<String>, RawQuery(q): RawQuery, body: axum::body::Bytes) -> Response {
    log.lock().unwrap().push((method.clone(), q.unwrap_or_default()));
    let ok = |extra: Value| {
        let mut r = json!({"status": "ok", "version": "1.16.1"});
        r.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        axum::Json(json!({"subsonic-response": r})).into_response()
    };
    match method.as_str() {
        "getRandomSongs" => ok(json!({"randomSongs": {"song": [{"id": "r1"}]}})),
        "getTranscodeDecision" => {
            let wanted: Value = serde_json::from_slice(&body).unwrap();
            let format = wanted["transcodingProfiles"][0]["audioCodec"].as_str().unwrap().to_string();
            ok(json!({"transcodeDecision": {"canDirectPlay": false, "canTranscode": format == "opus" || format == "mp3"}}))
        }
        "ping" | "updatePlaylist" => ok(json!({})),
        "getAlbum" => ok(json!({"album": {"id": "1", "name": "A"}})),
        "getInternetRadioStations" => ok(json!({"internetRadioStations": {"internetRadioStation": [
            {"id": "1", "name": "W", "streamUrl": "https://r.example.com/s?auth=pw"}]}})),
        "wrong" => axum::Json(json!({"subsonic-response": {"status": "failed", "error": {"code": 40, "message": "Wrong username or password"}}})).into_response(),
        "html" => (StatusCode::OK, "<html>not subsonic</html>").into_response(),
        "gateway" => (StatusCode::BAD_GATEWAY, "bad gateway").into_response(),
        _ => (StatusCode::NOT_FOUND, "<html>404</html>").into_response(),
    }
}

async fn server() -> (String, Log) {
    let log: Log = Default::default();
    let app = Router::new().route("/rest/{method}", axum::routing::any(handle)).with_state(log.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://{addr}"), log)
}

fn params(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

fn md5_hex(s: &str) -> String {
    use md5::{Digest, Md5};
    Md5::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test]
async fn url_has_token_auth() {
    let login = Login::from_password(" music.example.com/ ", " ali ", "secret").unwrap();
    assert_eq!(login.server, "https://music.example.com");
    assert_eq!(login.user, "ali");
    assert_eq!(login.token, md5_hex(&format!("secret{}", login.salt)));
    assert!(!login.salt.is_empty());
    let s = Subsonic::new(login.clone());
    let u = s.url("getAlbum", &[("id".into(), "1".into())]);
    assert!(u.as_str().starts_with("https://music.example.com/rest/getAlbum?id=1&"));
    let q: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.into(), v.into())).collect();
    for (k, v) in [("u", "ali"), ("t", login.token.as_str()), ("s", login.salt.as_str()), ("v", "1.16.1"), ("c", "re-la"), ("f", "json")] {
        assert!(q.contains(&(k.into(), v.into())), "{k}");
    }
    assert!(!u.as_str().contains("secret") && !q.iter().any(|(k, _)| k == "p"));
    assert!(Login::from_password("ftp://x", "a", "b").is_err());
}

#[tokio::test]
async fn ok_reply_is_the_response_object() {
    let (url, _) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    let r = s.call("ping", &[]).await.unwrap();
    assert_eq!(r["version"], "1.16.1");
}

#[tokio::test]
async fn subsonic_error_code_and_message() {
    let (url, _) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    let f = s.call("wrong", &[]).await.unwrap_err();
    assert_eq!((f.code, f.message.as_str()), (40, "Wrong username or password"));
    assert_eq!(f.login_text(), "Kullanıcı adı ya da şifre yanlış.");
    assert_eq!(f.status_text("wrong", &[]), "wrong: Wrong username or password");
}

#[tokio::test]
async fn no_server_and_not_subsonic() {
    let (url, _) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    assert_eq!(s.call("html", &[]).await.unwrap_err().code, NOT_SUBSONIC);
    assert_eq!(s.call("nothing", &[]).await.unwrap_err().code, NOT_SUBSONIC); // 404 page
    let g = s.call("gateway", &[]).await.unwrap_err();
    assert_eq!((g.code, g.http), (NO_SERVER, 502));
    assert_eq!(g.status_text("gateway", &[]), "gateway: HTTP 502");
    let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
    let s = Subsonic::new(Login::from_password(&format!("http://{dead}"), "a", "b").unwrap());
    let d = s.call("ping", &[]).await.unwrap_err();
    assert_eq!((d.code, d.http), (NO_SERVER, 0));
    assert_eq!(d.login_text(), "Sunucuya ulaşılamıyor.");
}

#[tokio::test]
async fn errors_never_carry_secrets() {
    let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
    let login = Login::from_password(&format!("http://{dead}"), "a", "b").unwrap();
    let s = Subsonic::new(login.clone());
    let f = s.call("ping", &[]).await.unwrap_err();
    let text = f.status_text("ping", &[&login.token, &login.salt]);
    assert!(!text.contains(&login.token) && !text.contains(&login.salt), "{text}");
    assert!(text.starts_with("ping: "), "{text}");
    // the control: the raw error does quote the request URL, token and all; only the cleaned text may be shown
    assert!(f.net_error.contains(&login.token), "{}", f.net_error);
}

#[tokio::test]
async fn get_only_reads_set_only_writes() {
    let (url, log) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    assert!(s.get("savePlayQueue", &params(json!({}))).await.is_none());
    assert!(s.set("getAlbum", &params(json!({"id": "1"}))).await.is_none());
    assert!(log.lock().unwrap().is_empty(), "nothing guarded reaches the server");
    let a = s.get("getAlbum", &params(json!({"id": "1"}))).await.unwrap();
    assert_eq!(a["album"]["name"], "A");
    assert!(s.get("wrong", &params(json!({}))).await.is_none()); // a failure is None, as done(null) was
}

#[tokio::test]
async fn get_strips_station_stream_urls() {
    let (url, _) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    let r = s.get("getInternetRadioStations", &params(json!({}))).await.unwrap();
    let st = &r["internetRadioStations"]["internetRadioStation"][0];
    assert!(st.get("streamUrl").is_none() && st["originUrl"] == "https://r.example.com");
}

#[tokio::test]
async fn list_params_repeat() {
    let (url, log) = server().await;
    let s = Subsonic::new(Login::from_password(&url, "a", "b").unwrap());
    s.set("updatePlaylist", &params(json!({"playlistId": "7", "songIdToAdd": ["a", "b"], "name": "R&B + Şarkı"}))).await.unwrap();
    let (_, q) = log.lock().unwrap().last().unwrap().clone();
    assert_eq!(q.matches("songIdToAdd=").count(), 2);
    assert!(q.contains("name=R%26B%20%2B%20%C5%9Eark%C4%B1"), "{q}");
}

#[tokio::test]
async fn only_the_formats_the_server_transcodes_to() {
    let (url, log) = server().await;
    let c = Subsonic::new(Login::from_password(&url, "ana", "pw").unwrap());
    assert_eq!(c.transcode_formats().await, ["opus", "mp3"]);
    let asked = log.lock().unwrap().iter().find(|(m, _)| m == "getTranscodeDecision").unwrap().1.clone();
    assert!(asked.starts_with("mediaId=r1&mediaType=song&"), "{asked}");
}
