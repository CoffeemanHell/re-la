//! What the relay (re-with) serves over HTTP besides the WebSocket.
use crate::subsonic::Login;
use crate::sync::relay_url;
use serde_json::Value;

/// `<relay>/<path…>` over HTTP(S): the relay's WebSocket address (own or the user's override) with its scheme swapped.
pub fn url(server: &str, override_url: &str, path: &[&str]) -> Option<url::Url> {
    let mut u = url::Url::parse(&relay_url(server, override_url)).ok()?;
    let scheme = if u.scheme() == "ws" { "http" } else { "https" };
    u.set_scheme(scheme).ok()?;
    u.path_segments_mut().ok()?.pop_if_empty().extend(path);
    Some(u)
}

/// The body of a 2xx answer. The login goes in headers, so it is in no URL (nor in a proxy's access log).
pub async fn get(http: &reqwest::Client, url: url::Url, login: &Login) -> Option<Vec<u8>> {
    let r = http
        .get(url)
        .header("X-Subsonic-User", &login.user)
        .header("X-Subsonic-Token", &login.token)
        .header("X-Subsonic-Salt", &login.salt)
        .send()
        .await
        .ok()?;
    Some(r.error_for_status().ok()?.bytes().await.ok()?.into())
}

/// The relay's list of Navidrome's transcoding profiles as ["opus-320"], of those a phone plays and a bitrate makes
/// smaller; None when that is no such list (an older relay, or none: Navidrome's web page).
pub fn transcodings(body: &[u8]) -> Option<Vec<String>> {
    let list: Vec<Value> = serde_json::from_slice(body).ok()?;
    let one = |t: &Value| {
        let (f, k) = (t["format"].as_str()?, t["kbps"].as_i64().filter(|k| *k > 0)?);
        crate::subsonic::TRANSCODES.contains(&f).then(|| format!("{f}-{k}"))
    };
    Some(list.iter().filter_map(one).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_cases() {
        let u = |s: &str, o: &str| url(s, o, &["waveform", "abc"]).map(|u| u.to_string());
        assert_eq!(u("https://m.example.com", "").as_deref(), Some("https://m.example.com/re-with/waveform/abc"));
        assert_eq!(u("http://10.0.0.2:4533/", "").as_deref(), Some("http://10.0.0.2:4533/re-with/waveform/abc"));
        assert_eq!(u("https://x", "wss://relay.example/").as_deref(), Some("https://relay.example/waveform/abc"));
        assert_eq!(u("https://x/music", "").as_deref(), Some("https://x/music/re-with/waveform/abc"));
        assert_eq!(url("https://x", "", &["waveform", "a/b"]).map(|u| u.path().to_string()).as_deref(), Some("/re-with/waveform/a%2Fb"));
        assert_eq!(u("not a url", ""), None);
    }

    #[test]
    fn transcodings_cases() {
        let t = |s: &str| transcodings(s.as_bytes());
        assert_eq!(t(r#"[{"format":"opus","kbps":320}]"#), Some(vec!["opus-320".into()]));
        // flac has no bitrate; wma is not for a phone; a bad entry is skipped
        assert_eq!(t(r#"[{"format":"flac","kbps":0},{"format":"wma","kbps":128},{"format":"mp3"},7,{"format":"mp3","kbps":192}]"#), Some(vec!["mp3-192".into()]));
        assert_eq!(t("[]"), Some(vec![]), "the server has none");
        assert_eq!(t("<!doctype html>"), None);
        assert_eq!(t("{}"), None);
    }
}
