//! Waveforms end to end: a stub Navidrome (scan status, search3, download), real ffmpeg, the real scanner.
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use md5::{Digest, Md5};
use re_with::waveform::{Account, BINS, Navidrome, Store, WaveformConfig, start};
use re_with::{Config, app};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;

const PASSWORD: &str = "pw";

#[derive(Default)]
struct Lib {
    scanning: bool,
    last_scan: String,
    songs: Vec<(String, u64, String)>, // id, size, suffix
    audio: HashMap<String, Vec<u8>>,
    fail_search: bool,
    downloads_off: bool,
    downloads: usize,
    hidden: Vec<String>, // getSong says "not found" (another user's library)
    crash: Option<String>, // download of this id drops the connection (Navidrome going away)
    scan_on_list: bool,    // a scan starts while search3 is being read
}

type Shared = Arc<Mutex<Lib>>;
type Q = Query<HashMap<String, String>>;

fn authed(q: &HashMap<String, String>) -> bool {
    let (Some(t), Some(s)) = (q.get("t"), q.get("s")) else { return false };
    let want: String = Md5::digest(format!("{PASSWORD}{s}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    q.get("u").map(String::as_str) == Some("re-with") && *t == want
}

fn ok(body: serde_json::Value) -> Response {
    let mut r = json!({"subsonic-response": {"status": "ok", "version": "1.16.1"}});
    r["subsonic-response"].as_object_mut().unwrap().extend(body.as_object().unwrap().clone());
    Json(r).into_response()
}

fn failed(code: u32) -> Response {
    Json(json!({"subsonic-response": {"status": "failed", "error": {"code": code, "message": "no"}}})).into_response()
}

async fn scan_status(State(l): State<Shared>, Query(q): Q) -> Response {
    if !authed(&q) {
        return failed(40);
    }
    let l = l.lock().unwrap();
    ok(json!({"scanStatus": {"scanning": l.scanning, "count": l.songs.len(), "lastScan": l.last_scan}}))
}

async fn search3(State(l): State<Shared>, Query(q): Q) -> Response {
    if !authed(&q) {
        return failed(40);
    }
    let mut l = l.lock().unwrap();
    if l.fail_search {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let shifted = l.scan_on_list; // a scan starts now: rows move, the last song falls off this read
    if shifted {
        l.scanning = true;
    }
    let n: usize = q["songCount"].parse().unwrap();
    let off: usize = q.get("songOffset").map_or(0, |o| o.parse().unwrap());
    let page: Vec<_> = l
        .songs
        .iter()
        .skip(off)
        .take(n)
        .take(if shifted { l.songs.len() - 1 } else { usize::MAX })
        .map(|(id, size, suffix)| json!({"id": id, "title": id, "size": size, "suffix": suffix}))
        .collect();
    ok(json!({"searchResult3": {"song": page}}))
}

async fn download(State(l): State<Shared>, Query(q): Q) -> Response {
    if !authed(&q) {
        return failed(40);
    }
    let mut l = l.lock().unwrap();
    if l.downloads_off {
        return failed(50);
    }
    if l.crash.as_ref() == Some(&q["id"]) {
        drop(l);
        panic!("stub Navidrome drops the connection"); // hyper closes it without an answer
    }
    l.downloads += 1;
    match l.audio.get(&q["id"]) {
        Some(b) => ([("content-type", "audio/flac")], b.clone()).into_response(),
        None => failed(70),
    }
}

async fn get_song(State(l): State<Shared>, Query(q): Q) -> Response {
    if !authed(&q) {
        return failed(40);
    }
    let l = l.lock().unwrap();
    if l.hidden.contains(&q["id"]) || !l.songs.iter().any(|s| s.0 == q["id"]) {
        return failed(70);
    }
    ok(json!({"song": {"id": q["id"]}}))
}

async fn serve(router: Router) -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, router).await.unwrap() });
    format!("http://{addr}")
}

async fn navidrome(lib: Shared) -> String {
    let r = Router::new()
        .route("/rest/getScanStatus", get(scan_status))
        .route("/rest/search3", get(search3))
        .route("/rest/download", get(download))
        .route("/rest/getSong", get(get_song))
        .with_state(lib);
    serve(r).await
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rwf-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// One second of a sine in the format of `ext`, loud as `amp`.
fn audio(dir: &Path, ext: &str, amp: f32) -> Vec<u8> {
    let out = dir.join(format!("src-{amp}.{ext}"));
    let src = format!("aevalsrc={amp}*sin(440*2*PI*t):s=44100:d=1");
    let ok = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", &src])
        .arg(&out)
        .status()
        .expect("ffmpeg needed for these tests")
        .success();
    assert!(ok);
    std::fs::read(out).unwrap()
}

fn account() -> Account {
    Account { user: "re-with".into(), password: PASSWORD.into() }
}

async fn run(name: &str, lib: Shared) -> (PathBuf, Arc<Mutex<Store>>) {
    let base = navidrome(lib).await;
    let dir = temp(name).join("waveforms");
    let cfg = WaveformConfig { dir: dir.clone(), account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_millis(100) };
    let store = start(cfg, reqwest::Client::new(), base, Default::default()).unwrap();
    (dir, store)
}

fn song(lib: &mut Lib, src: &Path, id: &str, ext: &str, amp: f32) {
    let b = audio(src, ext, amp);
    lib.songs.push((id.into(), b.len() as u64, ext.into()));
    lib.audio.insert(id.into(), b);
}

/// Waits until the folder holds exactly `ids` (sorted), or panics after 20 s.
async fn until(dir: &Path, ids: &[&str]) {
    let want: Vec<String> = ids.iter().map(|i| format!("{i}.wf")).collect();
    let mut got = vec![];
    for _ in 0..200 {
        got = std::fs::read_dir(dir)
            .map(|d| d.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.ends_with(".wf")).collect())
            .unwrap_or_default();
        got.sort();
        if got == want {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("waveforms: want {want:?}, got {got:?}");
}

fn new_lib(src: &Path, songs: &[(&str, &str, f32)]) -> Shared {
    let mut l = Lib { last_scan: "2026-11-22T10:00:00Z".into(), ..Lib::default() };
    for (id, ext, amp) in songs {
        song(&mut l, src, id, ext, *amp);
    }
    Arc::new(Mutex::new(l))
}

#[tokio::test]
async fn processes_the_library_at_start() {
    let src = temp("start-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "m4a", 0.25)]);
    let (dir, store) = run("start", lib).await;
    until(&dir, &["a", "b"]).await;
    let a = re_with::waveform::decode(&store.lock().unwrap().read("a").unwrap()).unwrap().1;
    assert!((125..=130).contains(&a.peaks[2000]), "{}", a.peaks[2000]);
    assert!(std::fs::read_dir(dir.join(".work")).unwrap().next().is_none(), "work files left behind");
}

#[tokio::test]
async fn new_scan_processes_new_and_removes_gone() {
    let src = temp("rescan-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "flac", 0.25)]);
    let (dir, _store) = run("rescan", lib.clone()).await;
    until(&dir, &["a", "b"]).await;
    {
        let mut l = lib.lock().unwrap();
        l.songs.retain(|s| s.0 != "a");
        song(&mut l, &src, "c", "flac", 0.75);
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    until(&dir, &["b", "c"]).await;
}

#[tokio::test]
async fn no_rescan_while_scanning_or_same_last_scan() {
    let src = temp("quiet-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    let (dir, _store) = run("quiet", lib.clone()).await;
    until(&dir, &["a"]).await;
    {
        let mut l = lib.lock().unwrap();
        song(&mut l, &src, "b", "flac", 0.25); // same lastScan: not looked at
    }
    tokio::time::sleep(Duration::from_millis(600)).await;
    {
        let mut l = lib.lock().unwrap();
        l.scanning = true;
        l.last_scan = "2026-11-22T11:00:00Z".into(); // a new scan, still running
    }
    tokio::time::sleep(Duration::from_millis(600)).await;
    until(&dir, &["a"]).await;
    lib.lock().unwrap().scanning = false;
    until(&dir, &["a", "b"]).await;
}

#[tokio::test]
async fn failed_listing_deletes_nothing() {
    let src = temp("fail-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "flac", 0.25)]);
    let (dir, _store) = run("fail", lib.clone()).await;
    until(&dir, &["a", "b"]).await;
    {
        let mut l = lib.lock().unwrap();
        l.fail_search = true;
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    tokio::time::sleep(Duration::from_millis(800)).await;
    until(&dir, &["a", "b"]).await;
}

#[tokio::test]
async fn changed_size_reprocesses() {
    let src = temp("size-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    let (dir, store) = run("size", lib.clone()).await;
    until(&dir, &["a"]).await;
    {
        let mut l = lib.lock().unwrap();
        l.songs.clear();
        l.audio.clear();
        song(&mut l, &src, "a", "flac", 0.25);
        l.songs[0].1 += 1; // a new file under the same id
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    for _ in 0..200 {
        let p = store.lock().unwrap().read("a").and_then(|b| re_with::waveform::decode(&b)).map(|(_, l)| l.peaks[2000]);
        if p.is_some_and(|p| p < 70) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("a was not made again");
}

#[tokio::test]
async fn unsafe_id_is_skipped() {
    let src = temp("unsafe-src");
    let lib = new_lib(&src, &[("../evil", "flac", 0.5), ("a", "flac", 0.5)]);
    let (dir, _store) = run("unsafe", lib.clone()).await;
    until(&dir, &["a"]).await;
    assert_eq!(lib.lock().unwrap().downloads, 1, "the unsafe id was downloaded");
    assert!(!dir.parent().unwrap().join("evil.wf").exists());
}

#[tokio::test]
async fn downloads_disabled_stops_the_pass() {
    let src = temp("off-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    lib.lock().unwrap().downloads_off = true;
    let (dir, _store) = run("off", lib.clone()).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    until(&dir, &[]).await;
    {
        let mut l = lib.lock().unwrap();
        l.downloads_off = false;
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    until(&dir, &["a"]).await;
}

#[tokio::test]
async fn pages_through_the_library() {
    let mut l = Lib::default();
    for i in 0..1100 {
        l.songs.push((format!("s{i}"), i, "mp3".into()));
    }
    let base = navidrome(Arc::new(Mutex::new(l))).await;
    let nd = Navidrome::new(reqwest::Client::new(), base, account());
    let songs = nd.songs().await.unwrap();
    assert_eq!(songs.len(), 1100);
    assert_eq!(songs[1099].id, "s1099");
    assert_eq!(songs[1099].key.size, 1099);
    assert_eq!(songs[1099].key.suffix, "mp3");
}

/// The relay with waveforms on; returns its base URL and the waveform folder.
async fn relay(name: &str, lib: Shared) -> (String, PathBuf) {
    let base = navidrome(lib).await;
    let dir = temp(name).join("waveforms");
    let waveforms = WaveformConfig { dir: dir.clone(), account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_millis(100) };
    (serve(app(Config { navidrome: base, waveforms: Some(waveforms), status_socket: None })).await, dir)
}

/// A client's login: Subsonic token headers for user "alice" (the stub takes any user with the password).
async fn fetch(url: &str, login: bool, token_ok: bool) -> reqwest::Response {
    let mut r = reqwest::Client::new().get(url);
    if login {
        let s = "cafe";
        let pw = if token_ok { PASSWORD } else { "wrong" };
        let t: String = Md5::digest(format!("{pw}{s}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        r = r.header("X-Subsonic-User", "re-with").header("X-Subsonic-Token", t).header("X-Subsonic-Salt", s);
    }
    r.send().await.unwrap()
}

#[tokio::test]
async fn endpoint_serves_a_processed_song() {
    let src = temp("ep-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    let (base, dir) = relay("ep", lib).await;
    until(&dir, &["a"]).await;
    let r = fetch(&format!("{base}/waveform/a"), true, true).await;
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["cache-control"], "private, max-age=86400");
    let b = r.bytes().await.unwrap();
    assert_eq!(b.len(), 2 + 2 * BINS);
    assert_eq!(u16::from_le_bytes([b[0], b[1]]) as usize, BINS);
}

#[tokio::test]
async fn endpoint_status_codes() {
    let src = temp("codes-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    lib.lock().unwrap().songs.push(("later".into(), 1, "flac".into())); // listed, no audio: never made
    let (base, dir) = relay("codes", lib.clone()).await;
    until(&dir, &["a"]).await;
    assert_eq!(fetch(&format!("{base}/waveform/a"), false, true).await.status(), 401);
    assert_eq!(fetch(&format!("{base}/waveform/a"), true, false).await.status(), 401);
    assert_eq!(fetch(&format!("{base}/waveform/later"), true, true).await.status(), 404);
    lib.lock().unwrap().hidden.push("a".into());
    assert_eq!(fetch(&format!("{base}/waveform/a"), true, true).await.status(), 404);
    assert_eq!(fetch(&format!("{base}/waveform/..%2Fa"), true, true).await.status(), 404);
}

#[tokio::test]
async fn endpoint_without_navidrome_is_503() {
    let dir = temp("down").join("waveforms");
    let waveforms = WaveformConfig { dir, account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_secs(60) };
    let base = serve(app(Config { navidrome: "http://127.0.0.1:1".into(), waveforms: Some(waveforms), status_socket: None })).await;
    assert_eq!(fetch(&format!("{base}/waveform/a"), true, true).await.status(), 503);
}

#[tokio::test]
async fn endpoint_without_module_is_404() {
    let src = temp("off-ep-src");
    let base = serve(app(Config { navidrome: navidrome(new_lib(&src, &[])).await, waveforms: None, status_socket: None })).await;
    assert_eq!(fetch(&format!("{base}/waveform/a"), true, true).await.status(), 404);
}

#[tokio::test]
async fn navidrome_going_away_mid_pass_is_retried_without_a_new_scan() {
    let src = temp("away-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "flac", 0.25)]);
    lib.lock().unwrap().crash = Some("b".into());
    let (dir, _store) = run("away", lib.clone()).await;
    until(&dir, &["a"]).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    lib.lock().unwrap().crash = None; // back, same lastScan
    until(&dir, &["a", "b"]).await;
}

#[tokio::test]
async fn a_scan_starting_during_the_listing_deletes_nothing() {
    let src = temp("mid-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "flac", 0.25)]);
    let (dir, _store) = run("mid", lib.clone()).await;
    until(&dir, &["a", "b"]).await;
    {
        let mut l = lib.lock().unwrap();
        l.scan_on_list = true;
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    tokio::time::sleep(Duration::from_millis(600)).await;
    until(&dir, &["a", "b"]).await;
    {
        let mut l = lib.lock().unwrap();
        l.scan_on_list = false;
        l.scanning = false;
    }
    tokio::time::sleep(Duration::from_millis(400)).await;
    until(&dir, &["a", "b"]).await;
}

/// One snapshot from the status socket, or None while it is not there yet.
async fn snapshot(sock: &Path) -> Option<serde_json::Value> {
    use tokio::io::AsyncReadExt;
    let mut s = tokio::net::UnixStream::connect(sock).await.ok()?;
    let mut b = vec![];
    s.read_to_end(&mut b).await.ok()?;
    serde_json::from_slice(&b).ok()
}

#[tokio::test]
async fn status_socket_shows_the_pass() {
    let src = temp("status-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5), ("b", "flac", 0.25)]);
    let base = navidrome(lib).await;
    let root = temp("status");
    let (dir, sock) = (root.join("waveforms"), root.join("status.sock"));
    std::fs::write(&sock, b"stale").unwrap(); // left by a crash: must not stop the start
    let waveforms = WaveformConfig { dir, account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_millis(100) };
    serve(app(Config { navidrome: base, waveforms: Some(waveforms), status_socket: Some(sock.clone()) })).await;
    let mut last = None;
    for _ in 0..200 {
        last = snapshot(&sock).await;
        if last.as_ref().is_some_and(|v| v["waveforms"]["phase"] == "current") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let v = last.expect("no snapshot");
    let w = &v["waveforms"];
    assert_eq!(w["phase"], "current", "{v}");
    assert_eq!((w["total"].as_u64(), w["done"].as_u64(), w["failed"].as_u64()), (Some(2), Some(2), Some(0)));
    assert_eq!(w["stored"], 2);
    assert_eq!(w["scanning"], false);
    assert_eq!(v["sync"]["online"], 0);
    assert!(v["version"].is_string() && v["uptime_s"].is_u64());
}

#[tokio::test]
async fn status_without_waveforms_says_off() {
    let root = temp("status-off");
    let sock = root.join("status.sock");
    serve(app(Config { navidrome: "http://127.0.0.1:1".into(), waveforms: None, status_socket: Some(sock.clone()) })).await;
    let v = snapshot(&sock).await.expect("no snapshot");
    assert_eq!(v["waveforms"]["phase"], "off");
}

/// Polls the status socket until the waveform phase is `want`, or panics after 20 s.
async fn until_phase(sock: &Path, want: &str) -> serde_json::Value {
    let mut last = None;
    for _ in 0..200 {
        last = snapshot(sock).await;
        if last.as_ref().is_some_and(|v| v["waveforms"]["phase"] == want) {
            return last.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("phase: want {want}, got {last:?}");
}

#[tokio::test]
async fn a_pass_cut_by_navidrome_is_not_shown_as_up_to_date() {
    let src = temp("status-cut-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    let base = navidrome(lib.clone()).await;
    let root = temp("status-cut");
    let sock = root.join("status.sock");
    let waveforms = WaveformConfig { dir: root.join("waveforms"), account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_millis(100) };
    serve(app(Config { navidrome: base, waveforms: Some(waveforms), status_socket: Some(sock.clone()) })).await;
    until_phase(&sock, "current").await;
    {
        let mut l = lib.lock().unwrap();
        song(&mut l, &src, "b", "flac", 0.25);
        l.crash = Some("b".into()); // Navidrome goes away in the middle of the next pass
        l.last_scan = "2026-11-22T11:00:00Z".into();
    }
    until_phase(&sock, "unreachable").await;
    {
        let mut l = lib.lock().unwrap();
        (l.crash, l.scanning) = (None, true); // back, and scanning: the pass waits for the scan
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let v = snapshot(&sock).await.unwrap();
    assert_eq!(v["waveforms"]["phase"], "waiting", "{v}");
}

#[tokio::test]
async fn a_new_pass_does_not_show_the_last_ones_numbers() {
    let src = temp("status-fresh-src");
    let lib = new_lib(&src, &[("a", "flac", 0.5)]);
    lib.lock().unwrap().songs.push(("broken".into(), 1, "flac".into())); // listed, no audio: fails
    let base = navidrome(lib.clone()).await;
    let root = temp("status-fresh");
    let sock = root.join("status.sock");
    let waveforms = WaveformConfig { dir: root.join("waveforms"), account: account(), ffmpeg: "ffmpeg".into(), poll: Duration::from_millis(100) };
    serve(app(Config { navidrome: base, waveforms: Some(waveforms), status_socket: Some(sock.clone()) })).await;
    let v = until_phase(&sock, "current").await;
    assert_eq!((v["waveforms"]["total"].as_u64(), v["waveforms"]["failed"].as_u64()), (Some(2), Some(1)));
    {
        let mut l = lib.lock().unwrap();
        l.last_scan = "2026-11-22T11:00:00Z".into();
        l.fail_search = true; // the next pass never gets past its listing
    }
    let v = until_phase(&sock, "unreachable").await;
    let w = &v["waveforms"];
    assert_eq!((w["total"].as_u64(), w["done"].as_u64(), w["failed"].as_u64()), (Some(0), Some(0), Some(0)), "{v}");
    assert!(w["last_error"].is_null(), "{v}");
}
