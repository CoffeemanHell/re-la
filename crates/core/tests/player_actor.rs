//! The player actor (its own thread) against a fake Subsonic server (axum) and a fake deck.
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rela_core::player::{Changes, Deck, DeckSink, Player, PlayerListener, PlayerSnapshot};
use rela_core::settings::{JsonSettings, Settings};
use rela_core::subwave::{SubwaveEvent, SubwaveSnapshot};
use rela_core::sync::{SyncEvent, SyncSnapshot};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Log = Arc<Mutex<Vec<(String, String)>>>; // (method, raw query)

const COVER: &[u8] = b"\x89PNG fake cover";

async fn handle(State(log): State<Log>, Path(method): Path<String>, RawQuery(q): RawQuery) -> Response {
    log.lock().unwrap().push((method.clone(), q.unwrap_or_default()));
    let ok = |extra: Value| {
        let mut r = json!({"status": "ok", "version": "1.16.1"});
        r.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        axum::Json(json!({"subsonic-response": r})).into_response()
    };
    let song = |id: &str| json!({"id": id, "title": id, "duration": 200});
    match method.as_str() {
        "ping" | "savePlayQueue" | "scrobble" => ok(json!({})),
        "getOpenSubsonicExtensions" => ok(json!({"openSubsonicExtensions": []})),
        "getPlayQueue" => ok(json!({"playQueue": {"entry": [song("s1"), song("s2")], "current": "s2", "position": 7000}})),
        "stream" => ([(header::CONTENT_TYPE, "audio/flac")], "audio").into_response(),
        "getAlbum" => ok(json!({"album": {"id": "al1", "name": "Album"}})),
        "getCoverArt" => ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "max-age=0")], COVER).into_response(),
        _ => (StatusCode::NOT_FOUND, "<html>404</html>").into_response(),
    }
}

/// The relay's waveform endpoint: logs the URL and the login headers; "w1" is made, anything else is not.
async fn waveform(State(log): State<Log>, Path(id): Path<String>, headers: axum::http::HeaderMap, RawQuery(q): RawQuery) -> Response {
    let h = |k: &str| headers.get(k).map_or("", |v| v.to_str().unwrap()).to_string();
    log.lock().unwrap().push((format!("waveform/{id}"), format!("{}|{}|{}|{}", q.unwrap_or_default(), h("x-subsonic-user"), h("x-subsonic-token"), h("x-subsonic-salt"))));
    if id != "w1" {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut b = 4096u16.to_le_bytes().to_vec();
    b.extend(std::iter::repeat_n(255u8, 4096));
    b.extend(std::iter::repeat_n(51u8, 4096));
    b.into_response()
}

fn server() -> (String, Log, tokio::runtime::Runtime) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let log: Log = Default::default();
    let app = Router::new()
        .route("/rest/{method}", get(handle))
        .route("/re-with/waveform/{id}", get(waveform))
        .with_state(log.clone());
    let l = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let addr = l.local_addr().unwrap();
    rt.spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://{addr}"), log, rt)
}

struct FakeDeck(usize, Arc<Mutex<Vec<String>>>);

impl Deck for FakeDeck {
    fn load(&mut self, url: &str, paused: bool) {
        let id = url::Url::parse(url).ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "id").map(|(_, v)| v.to_string()));
        self.1.lock().unwrap().push(format!("{} load {} paused={paused}", self.0, id.unwrap_or_default()));
    }
    fn append(&mut self, _: &str) {}
    fn clear_appended(&mut self) {}
    fn drop_first(&mut self) {}
    fn set_paused(&mut self, _: bool) {}
    fn seek(&mut self, _: f64) {}
    fn set_volume(&mut self, _: f64) {}
    fn set_speed(&mut self, _: f64) {}
    fn stop(&mut self) {}
    fn set_exclusive(&mut self, _: bool) {}
}

#[derive(Default)]
struct Front {
    last: Mutex<Option<PlayerSnapshot>>,
    replies: Mutex<Vec<(u64, Option<Value>, bool)>>,
}

impl PlayerListener for Front {
    fn on_player(&self, _: Changes, s: PlayerSnapshot) {
        *self.last.lock().unwrap() = Some(s);
    }
    fn on_reply(&self, id: u64, reply: Option<Value>, call: bool) {
        self.replies.lock().unwrap().push((id, reply, call));
    }
    fn on_sync(&self, _: SyncEvent, _: SyncSnapshot) {}
    fn on_subwave(&self, _: SubwaveEvent, _: SubwaveSnapshot) {}
}

fn wait_for(what: &str, cond: impl Fn() -> bool) {
    for _ in 0..400 {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("re-la-actor-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn start(name: &str) -> (Player, Arc<Front>, Arc<Mutex<Vec<String>>>, PathBuf) {
    let dir = tmp(name);
    let settings: Arc<dyn Settings> = Arc::new(JsonSettings::open(dir.join("settings.json")));
    let front = Arc::new(Front::default());
    let decks: Arc<Mutex<Vec<String>>> = Arc::default();
    let d2 = decks.clone();
    let make = move |i: usize, _sink: DeckSink| -> Box<dyn Deck> { Box::new(FakeDeck(i, d2.clone())) };
    let player = Player::new(settings, dir.clone(), make, front.clone());
    (player, front, decks, dir)
}

fn online_with_queue(front: &Front) -> bool {
    front.last.lock().unwrap().as_ref().is_some_and(|s| s.online && s.queue.len() == 2)
}

#[test]
fn login_restores_queue_and_persist_now_saves() {
    let (url, log, _rt) = server();
    let (player, front, decks, dir) = start("login");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online with the server's queue", || online_with_queue(&front));
    assert!(decks.lock().unwrap().contains(&"0 load s2 paused=true".to_string()), "{:?}", decks.lock().unwrap());
    let s = front.last.lock().unwrap().clone().unwrap();
    assert_eq!((s.index, s.position), (1, 7.0));

    player.call(|c| {
        c.request(5, "getAlbum", Map::new(), false);
    });
    wait_for("the theme's reply", || !front.replies.lock().unwrap().is_empty());
    let (id, reply, call) = front.replies.lock().unwrap()[0].clone();
    assert_eq!((id, call, reply.unwrap()["album"]["name"].as_str()), (5, true, Some("Album")));

    player.persist_now();
    let saves: Vec<String> = log.lock().unwrap().iter().filter(|(m, _)| m == "savePlayQueue").map(|(_, q)| q.clone()).collect();
    assert_eq!(saves.len(), 1, "{saves:?}");
    assert!(saves[0].starts_with("id=s1&id=s2&current=s2&position=7000&"), "{saves:?}");
    assert!(dir.join("queue.json").exists());
}

fn cover(player: &Player, id: &str) -> Option<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel();
    player.cover(id.into(), 300, move |b| {
        let _ = tx.send(b);
    });
    rx.recv_timeout(Duration::from_secs(10)).expect("the cover callback")
}

#[test]
fn cover_comes_from_cache_offline() {
    let (url, log, _rt) = server();
    let (player, front, _, _) = start("cover");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.online));
    assert_eq!(cover(&player, "al1"), Some(COVER.to_vec()));
    let asked = || log.lock().unwrap().iter().filter(|(m, _)| m == "getCoverArt").count();
    assert_eq!(asked(), 1);
    player.call(|c| c.logout());
    wait_for("offline", || front.last.lock().unwrap().as_ref().is_some_and(|s| !s.online && !s.signed_in));
    assert_eq!(cover(&player, "al1"), Some(COVER.to_vec()));
    assert_eq!(asked(), 1, "offline: never the network");
}

#[test]
fn sw_cover_without_station_is_none() {
    let (url, log, _rt) = server();
    let (player, front, _, _) = start("swcover");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.online));
    assert_eq!(cover(&player, "sw:al1"), None, "no SUB/WAVE open: no URL, not a Subsonic cover");
    assert_eq!(log.lock().unwrap().iter().filter(|(m, _)| m == "getCoverArt").count(), 0);
}

fn waveform_of(player: &Player, id: &str) -> Option<rela_core::waveform::Waveform> {
    let (tx, rx) = std::sync::mpsc::channel();
    player.waveform(id.into(), move |w| {
        let _ = tx.send(w);
    });
    rx.recv_timeout(Duration::from_secs(10)).expect("the waveform callback")
}

#[test]
fn waveform_comes_from_the_relay_with_the_login_in_headers() {
    let (url, log, _rt) = server();
    let (player, front, _, _) = start("waveform");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.online));
    let w = waveform_of(&player, "w1").expect("a waveform");
    assert_eq!((w.peaks.len(), w.rms.len()), (4096, 4096));
    assert_eq!((w.peaks[0], w.rms[0]), (1.0, 0.2));
    let asked: Vec<String> = log.lock().unwrap().iter().filter(|(m, _)| m == "waveform/w1").map(|(_, q)| q.clone()).collect();
    assert_eq!(asked.len(), 1);
    let parts: Vec<&str> = asked[0].split('|').collect();
    assert_eq!(parts[0], "", "nothing in the URL: {}", asked[0]);
    assert_eq!(parts[1], "ana");
    assert_eq!(parts[2].len(), 32, "an md5 token in the header");
    assert!(!parts[3].is_empty());
    assert_eq!(waveform_of(&player, "w2"), None, "not made yet");
    // from disk now, also without the server
    assert_eq!(waveform_of(&player, "w1").map(|w| w.peaks.len()), Some(4096));
    player.call(|c| c.logout());
    assert_eq!(waveform_of(&player, "w1").map(|w| w.peaks.len()), Some(4096));
    assert_eq!(log.lock().unwrap().iter().filter(|(m, _)| m == "waveform/w1").count(), 1);
}

#[test]
fn songs_coming_up_land_on_disk() {
    let (url, log, _rt) = server();
    let (player, front, _, dir) = start("songs");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.online));
    player.call(|c| {
        c.set_option("offlineGb", json!(1));
        c.set_option("offlineAhead", json!(1));
        c.play(vec![json!({"id": "s1", "duration": 200}), json!({"id": "s2", "duration": 200}), json!({"id": "s3"})], 0);
    });
    wait_for("the song", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.cache_bytes == 5));
    let files: Vec<String> = std::fs::read_dir(dir.join("songs")).unwrap().flatten().map(|e| e.file_name().into_string().unwrap()).collect();
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(files[0].ends_with(".raw"), "{files:?}");
    assert!(log.lock().unwrap().iter().any(|(m, q)| m == "stream" && q.starts_with("id=s2&format=raw&")));
}

#[test]
fn waveform_without_a_song_or_offline_asks_nothing() {
    let (url, log, _rt) = server();
    let (player, front, _, _) = start("waveform-none");
    assert_eq!(waveform_of(&player, "w1"), None, "signed out");
    player.call(move |c| c.login(&url, "ana", "pw"));
    wait_for("online", || front.last.lock().unwrap().as_ref().is_some_and(|s| s.online));
    assert_eq!(waveform_of(&player, "sw:w1"), None);
    assert_eq!(waveform_of(&player, ""), None);
    assert_eq!(log.lock().unwrap().iter().filter(|(m, _)| m.starts_with("waveform/")).count(), 0);
}
