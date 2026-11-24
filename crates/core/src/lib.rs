//! re-la's core, shared by the desktop app (cxx-qt) and Android (UniFFI): no Qt, no Android in here.
#![forbid(unsafe_code)]
pub mod covers;
pub mod player;
pub mod queue;
pub mod radio;
pub mod settings;
pub mod subsonic;
pub mod subwave;
pub mod sync;
pub mod waveform;

/// What every HTTP client here starts from. With HTTP/2 all calls to a server share one connection: one that went
/// silent (another network, a proxy that forgot it) would fail them all, again and again. While calls wait, it is asked
/// for a sign of life, and after 15 s of silence replaced (the patience the downloads have too).
fn http_client() -> reqwest::ClientBuilder {
    let s = std::time::Duration::from_secs;
    reqwest::Client::builder().http2_keep_alive_interval(s(5)).http2_keep_alive_timeout(s(10))
}
