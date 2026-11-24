//! `GET /waveform/{id}`: a made song's levels for a logged-in client that can see the song.
use super::{Store, body, safe_id};
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Ctx {
    store: Option<Arc<Mutex<Store>>>,
    http: reqwest::Client,
    navidrome: String,
}

/// `store` None: waveforms are off and every request is a 404.
pub fn router(store: Option<Arc<Mutex<Store>>>, http: reqwest::Client, navidrome: String) -> Router {
    Router::new().route("/waveform/{id}", get(waveform)).with_state(Ctx { store, http, navidrome })
}

enum Can {
    See,
    NotFound,
    Denied,
    Unavailable,
}

/// The caller's own login asks Navidrome for the song: one call checks both the login and that this user's
/// libraries hold it. The token goes to 127.0.0.1 only and is never kept or logged.
async fn can_see(c: &Ctx, id: &str, h: &HeaderMap) -> Can {
    let get = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).unwrap_or_default();
    let (u, t, s) = (get("x-subsonic-user"), get("x-subsonic-token"), get("x-subsonic-salt"));
    if u.is_empty() || t.is_empty() || s.is_empty() {
        return Can::Denied;
    }
    let q = [("id", id), ("u", u), ("t", t), ("s", s), ("v", "1.16.1"), ("c", "re-with"), ("f", "json")];
    let Ok(url) = reqwest::Url::parse_with_params(&format!("{}/rest/getSong", c.navidrome.trim_end_matches('/')), &q) else {
        return Can::Unavailable;
    };
    let Ok(resp) = c.http.get(url).send().await else { return Can::Unavailable };
    let Ok(b) = resp.bytes().await else { return Can::Unavailable };
    let Ok(v) = serde_json::from_slice::<Value>(&b) else { return Can::Unavailable };
    let r = &v["subsonic-response"];
    match (r["status"].as_str(), r["error"]["code"].as_u64()) {
        (Some("ok"), _) => Can::See,
        (Some("failed"), Some(70)) => Can::NotFound,
        (Some("failed"), _) => Can::Denied,
        _ => Can::Unavailable,
    }
}

async fn waveform(State(c): State<Ctx>, Path(id): Path<String>, h: HeaderMap) -> Response {
    let Some(store) = &c.store else { return StatusCode::NOT_FOUND.into_response() };
    if !safe_id(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match can_see(&c, &id, &h).await {
        Can::See => {}
        Can::NotFound => return StatusCode::NOT_FOUND.into_response(),
        Can::Denied => return StatusCode::UNAUTHORIZED.into_response(),
        Can::Unavailable => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    let file = store.lock().unwrap().read(&id);
    match file.as_deref().and_then(body) {
        Some(b) => (
            [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "private, max-age=86400")],
            b.to_vec(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
