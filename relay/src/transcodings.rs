//! `GET /transcodings`: Navidrome's transcoding profiles, for a logged-in client. Only Navidrome's own web API lists them
//! (the Subsonic API also makes formats that have no profile), and that takes a password: re-with's own account asks.
use crate::auth::{self, Auth};
use crate::waveform::Navidrome;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Each question is a login, and Navidrome allows few of them (5 in 20 s).
const FRESH: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct Ctx {
    nd: Option<Arc<Navidrome>>,
    http: reqwest::Client,
    navidrome: String,
    last: Arc<Mutex<Option<(Instant, Value)>>>,
}

/// `nd` None: re-with has no account and every request is a 404.
pub fn router(nd: Option<Navidrome>, http: reqwest::Client, navidrome: String) -> Router {
    let ctx = Ctx { nd: nd.map(Arc::new), http, navidrome, last: Arc::default() };
    Router::new().route("/transcodings", get(transcodings)).with_state(ctx)
}

async fn transcodings(State(c): State<Ctx>, h: HeaderMap) -> Response {
    let Some(nd) = &c.nd else { return StatusCode::NOT_FOUND.into_response() };
    let get = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).unwrap_or_default();
    match auth::ping(&c.http, &c.navidrome, get("x-subsonic-user"), get("x-subsonic-token"), get("x-subsonic-salt")).await {
        Auth::Ok => {}
        Auth::Denied => return StatusCode::UNAUTHORIZED.into_response(),
        Auth::Unavailable => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    let mut last = c.last.lock().await;
    let list = match &*last {
        Some((at, v)) if at.elapsed() < FRESH => v.clone(),
        _ => match nd.transcodings().await {
            Some(v) => last.insert((Instant::now(), v)).1.clone(),
            None => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
    };
    ([(header::CACHE_CONTROL, "no-store")], Json(list)).into_response()
}
