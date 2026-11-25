//! The cover cache against a fake getCoverArt (axum): freshness, revalidation, offline, cap, login-free keys.
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rela_core::covers::CoverCache;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Fake {
    cache_control: String,
    etag: Option<String>,
    size: usize,
    requests: Vec<Option<String>>, // If-None-Match of each request
}
type Shared = Arc<Mutex<Fake>>;

async fn cover(State(f): State<Shared>, headers: HeaderMap) -> Response {
    let mut f = f.lock().unwrap();
    let inm = headers.get("if-none-match").map(|v| v.to_str().unwrap().to_string());
    f.requests.push(inm.clone());
    if inm.is_some() && inm == f.etag {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let mut r = (StatusCode::OK, vec![7u8; f.size.max(1)]).into_response();
    if !f.cache_control.is_empty() {
        r.headers_mut().insert("cache-control", f.cache_control.parse().unwrap());
    }
    if let Some(e) = &f.etag {
        r.headers_mut().insert("etag", e.parse().unwrap());
    }
    r
}

struct Server {
    base: String,
    fake: Shared,
    task: tokio::task::JoinHandle<()>,
}

async fn server(cache_control: &str, etag: Option<&str>, size: usize) -> Server {
    let fake: Shared = Arc::new(Mutex::new(Fake { cache_control: cache_control.into(), etag: etag.map(str::to_string), size, requests: vec![] }));
    let app = Router::new().route("/rest/getCoverArt", get(cover)).with_state(fake.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Server { base, fake, task }
}

impl Server {
    fn url(&self, id: &str, token: &str) -> url::Url {
        url::Url::parse(&format!("{}/rest/getCoverArt?id={id}&size=256&u=me&t={token}&s=salt&v=1.16.1&c=re-la&f=json", self.base)).unwrap()
    }
    fn count(&self) -> usize {
        self.fake.lock().unwrap().requests.len()
    }
}

/// The cache folder of a test (its parent holds nothing else).
fn dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("re-la-covers-{}-{name}", std::process::id())).join("covers2")
}

/// The same folder, emptied: for the cache to open.
fn fresh(name: &str) -> PathBuf {
    let d = dir(name);
    let _ = std::fs::remove_dir_all(d.parent().unwrap());
    d
}

fn http() -> reqwest::Client {
    reqwest::Client::new()
}

#[tokio::test]
async fn fresh_hit_needs_no_request() {
    let s = server("public, max-age=3600", None, 100).await;
    let c = CoverCache::open(fresh("fresh"), 1 << 20);
    let a = c.get(&http(), &s.url("a", "t1"), false).await.unwrap();
    let b = c.get(&http(), &s.url("a", "t1"), false).await.unwrap();
    assert_eq!((a.len(), a, s.count()), (100, b, 1));
}

#[tokio::test]
async fn key_ignores_login() {
    let s = server("max-age=3600", None, 10).await;
    let c = CoverCache::open(fresh("login"), 1 << 20);
    c.get(&http(), &s.url("a", "tok-one"), false).await.unwrap();
    c.get(&http(), &s.url("a", "tok-two"), false).await.unwrap(); // a new login: same cover
    assert_eq!(s.count(), 1);
    let files: Vec<String> = std::fs::read_dir(dir("login")).unwrap().flatten()
        .map(|e| String::from_utf8_lossy(&std::fs::read(e.path()).unwrap()).into_owned())
        .collect();
    assert_eq!(files.len(), 2, "one entry: image + meta");
    assert!(files.iter().any(|t| t.contains("getCoverArt")), "the meta keeps the login-free key");
    assert!(!files.iter().any(|t| t.contains("tok-one") || t.contains("salt")), "the login must not be written down");
}

#[tokio::test]
async fn stale_revalidates_with_etag() {
    let s = server("no-cache", Some("\"v1\""), 10).await;
    let c = CoverCache::open(fresh("etag"), 1 << 20);
    let a = c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    let b = c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    assert_eq!(a, b);
    let reqs = s.fake.lock().unwrap().requests.clone();
    assert_eq!(reqs, [None, Some("\"v1\"".into())]);
}

#[tokio::test]
async fn offline_serves_stale() {
    let s = server("no-cache", None, 10).await;
    let c = CoverCache::open(fresh("offline"), 1 << 20);
    let a = c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    s.task.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(c.get(&http(), &s.url("a", "t"), false).await, Some(a));
}

#[tokio::test]
async fn cache_only_never_asks() {
    let s = server("no-cache", None, 10).await;
    let c = CoverCache::open(fresh("only"), 1 << 20);
    assert_eq!(c.get(&http(), &s.url("a", "t"), true).await, None);
    assert_eq!(s.count(), 0);
    c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    assert!(c.get(&http(), &s.url("a", "t"), true).await.is_some()); // stale is fine offline
    assert_eq!(s.count(), 1);
}

#[tokio::test]
async fn no_store_is_not_kept() {
    let s = server("no-store", None, 10).await;
    let c = CoverCache::open(fresh("nostore"), 1 << 20);
    c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    assert_eq!(s.count(), 2);
    assert_eq!(c.get(&http(), &s.url("a", "t"), true).await, None);
}

#[tokio::test]
async fn cap_evicts_oldest() {
    let s = server("max-age=3600", None, 1000).await;
    let c = CoverCache::open(fresh("cap"), 3500);
    for id in ["a", "b", "c", "d"] {
        c.get(&http(), &s.url(id, "t"), false).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await; // distinct access times
    }
    assert_eq!(c.get(&http(), &s.url("a", "t"), true).await, None, "the oldest went first");
    assert!(c.get(&http(), &s.url("d", "t"), true).await.is_some());
    let total: u64 = std::fs::read_dir(dir("cap")).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum();
    assert!(total <= 3500, "{total} bytes over the cap");
}

#[tokio::test]
async fn revalidating_keeps_the_image_file() {
    let s = server("no-cache", Some("\"v1\""), 10).await;
    let c = CoverCache::open(fresh("304"), 1 << 20);
    c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    let img = std::fs::read_dir(dir("304")).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "img")).unwrap();
    std::fs::write(&img, b"kept").unwrap();
    let link = img.with_extension("same");
    std::fs::hard_link(&img, &link).unwrap(); // a rewrite through a rename would leave this one behind
    c.get(&http(), &s.url("a", "t"), false).await.unwrap();
    std::fs::write(&link, b"linked").unwrap();
    assert_eq!(std::fs::read(&img).unwrap(), b"linked");
}

#[tokio::test]
async fn what_an_earlier_run_left_counts() {
    let s = server("max-age=3600", None, 1000).await;
    for id in ["a", "b", "c", "d"] {
        CoverCache::open(if id == "a" { fresh("run") } else { dir("run") }, 3500).get(&http(), &s.url(id, "t"), false).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let total: u64 = std::fs::read_dir(dir("run")).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum();
    assert!(total <= 3500, "{total} bytes over the cap");
}

#[test]
fn old_qt_cache_folder_is_removed() {
    let d = fresh("old");
    let old = d.parent().unwrap().join("covers");
    std::fs::create_dir_all(old.join("data8")).unwrap();
    std::fs::write(old.join("data8/x.d"), b"qt").unwrap();
    let _c = CoverCache::open(d, 1 << 20);
    assert!(!old.exists());
}
