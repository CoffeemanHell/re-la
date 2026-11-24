//! A song's waveform from the relay (re-with works it out once on the server): peaks and RMS, 0..1.
use crate::subsonic::Login;
use crate::sync::relay_url;

#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub peaks: Vec<f32>,
    pub rms: Vec<f32>, // each <= its peak
}

/// `<relay>/waveform/<id>` over HTTP(S): the relay's WebSocket address (own or the user's override) with its scheme
/// swapped.
pub fn url(server: &str, override_url: &str, id: &str) -> Option<url::Url> {
    let mut u = url::Url::parse(&relay_url(server, override_url)).ok()?;
    let scheme = if u.scheme() == "ws" { "http" } else { "https" };
    u.set_scheme(scheme).ok()?;
    u.path_segments_mut().ok()?.pop_if_empty().push("waveform").push(id);
    Some(u)
}

/// The relay's body: `bins` u16 LE, `bins` peak bytes, `bins` RMS bytes.
pub fn parse(b: &[u8]) -> Option<Waveform> {
    let bins = u16::from_le_bytes(b.get(..2)?.try_into().ok()?) as usize;
    if bins == 0 || b.len() != 2 + 2 * bins {
        return None;
    }
    let f = |x: &[u8]| x.iter().map(|&v| f32::from(v) / 255.0).collect();
    Some(Waveform { peaks: f(&b[2..2 + bins]), rms: f(&b[2 + bins..]) })
}

/// The login goes in headers, so it is in no URL (nor in a proxy's access log).
pub async fn fetch(http: &reqwest::Client, url: url::Url, login: &Login) -> Option<Waveform> {
    let r = http
        .get(url)
        .header("X-Subsonic-User", &login.user)
        .header("X-Subsonic-Token", &login.token)
        .header("X-Subsonic-Salt", &login.salt)
        .send()
        .await
        .ok()?;
    if !r.status().is_success() {
        return None;
    }
    parse(&r.bytes().await.ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_cases() {
        let u = |s: &str, o: &str| url(s, o, "abc").map(|u| u.to_string());
        assert_eq!(u("https://m.example.com", "").as_deref(), Some("https://m.example.com/re-with/waveform/abc"));
        assert_eq!(u("http://10.0.0.2:4533/", "").as_deref(), Some("http://10.0.0.2:4533/re-with/waveform/abc"));
        assert_eq!(u("https://x", "wss://relay.example/").as_deref(), Some("https://relay.example/waveform/abc"));
        assert_eq!(u("https://x/music", "").as_deref(), Some("https://x/music/re-with/waveform/abc"));
        assert_eq!(url("https://x", "", "a/b").map(|u| u.path().to_string()).as_deref(), Some("/re-with/waveform/a%2Fb"));
        assert_eq!(u("not a url", ""), None);
    }

    #[test]
    fn parse_cases() {
        let mut b = 2u16.to_le_bytes().to_vec();
        b.extend([255, 0, 51, 0]);
        assert_eq!(parse(&b), Some(Waveform { peaks: vec![1.0, 0.0], rms: vec![0.2, 0.0] }));
        assert_eq!(parse(&b[..5]), None);
        assert_eq!(parse(&[0, 0]), None);
        assert_eq!(parse(&[]), None);
    }
}
