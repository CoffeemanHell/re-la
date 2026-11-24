//! Login check: the relay has no accounts of its own, Navidrome's `ping` decides.

#[derive(Debug, PartialEq, Eq)]
pub enum Auth {
    Ok,
    /// Navidrome answered and refused these credentials.
    Denied,
    /// Navidrome didn't answer (down, restarting): the client should try again later.
    Unavailable,
}

/// Asks Navidrome whether these Subsonic token credentials are valid.
/// The token is used for this one request and never stored or logged.
pub async fn ping(http: &reqwest::Client, base: &str, u: &str, t: &str, s: &str) -> Auth {
    let url = format!("{}/rest/ping", base.trim_end_matches('/'));
    let params = [("u", u), ("t", t), ("s", s), ("v", "1.16.1"), ("c", "re-with"), ("f", "json")];
    let Ok(url) = reqwest::Url::parse_with_params(&url, &params) else { return Auth::Unavailable };
    let Ok(resp) = http.get(url).send().await else { return Auth::Unavailable };
    let Ok(body) = resp.bytes().await else { return Auth::Unavailable };
    match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(v) if v["subsonic-response"]["status"] == "ok" => Auth::Ok,
        Ok(v) if v["subsonic-response"]["status"] == "failed" => Auth::Denied,
        _ => Auth::Unavailable, // not a Subsonic answer (proxy error page, half-started server)
    }
}
