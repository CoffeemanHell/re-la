//! Parsing and math for the SUB/WAVE listener API (no network).
//! All paths are under <homePage>/api. Maps keep the keys QML reads.
use percent_encoding::percent_decode_str;
use serde_json::{Value, json};

/// Qt's toString(): the string, else "".
fn text(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// Qt's toInt(): a whole number, else 0.
fn int(v: &Value) -> i64 {
    match v.as_f64() {
        Some(f) if f.fract() == 0.0 => f as i64,
        _ => 0,
    }
}

fn iso_ms(v: &Value) -> i64 {
    v.as_str()
        .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
        .map_or(0, |t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
}

fn cut(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// <homePage>/api<path>; None unless homePage is an http(s) URL with a host.
pub fn sw_api(home_page: &str, path: &str) -> Option<url::Url> {
    let base = home_page.trim().trim_end_matches('/');
    let url = url::Url::parse(&format!("{base}/api{path}")).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(|h| !h.is_empty())).then_some(url)
}

/// Station password from a stream URL: the userinfo password, else the `auth` query value (what SUB/WAVE streams
/// use); "" if none. Percent-decoded; a `+` stays a `+` (as QUrl decoded it).
pub fn sw_password(stream_url: &str) -> String {
    let Ok(url) = url::Url::parse(stream_url) else { return String::new() };
    let decode = |s: &str| percent_decode_str(s).decode_utf8_lossy().into_owned();
    if let Some(p) = url.password().filter(|p| !p.is_empty()) {
        return decode(p);
    }
    url.query()
        .unwrap_or("")
        .split('&')
        .find_map(|kv| kv.strip_prefix("auth="))
        .map(decode)
        .unwrap_or_default()
}

/// /now-playing answers like SUB/WAVE: `nowPlaying` and `dj` are both objects.
pub fn sw_detect(np: &Value) -> bool {
    np["nowPlaying"].is_object() && np["dj"].is_object()
}

/// What the station plays, from /now-playing and /state. Two replies about the same track give equal maps: the
/// server clock (sw_server_at) is left out because it changes on every poll.
pub fn sw_now(np: &Value, st: &Value) -> Value {
    let t = &np["nowPlaying"];
    let upcoming: Vec<Value> = st["upcoming"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|o| json!({"title": text(&o["title"]), "artist": text(&o["artist"]), "requestedBy": text(&o["requestedBy"])}))
                .collect()
        })
        .unwrap_or_default();
    json!({
        "id": text(&t["subsonic_id"]),
        "title": text(&t["title"]),
        "artist": text(&t["artist"]),
        "album": text(&t["album"]),
        "duration": int(&t["duration"]),
        "dj": text(&np["dj"]["name"]),
        "tagline": text(&np["dj"]["tagline"]),
        "show": text(&np["activeShow"]["name"]),
        "listeners": int(&np["listeners"]["current"]),
        "online": np["streamOnline"].as_bool().unwrap_or(false),
        "startedAt": iso_ms(&st["current"]["startedAt"]),
        "upcoming": upcoming,
    })
}

/// The server clock when it answered /now-playing (ms since epoch, 0 when unknown).
pub fn sw_server_at(np: &Value) -> i64 {
    iso_ms(&np["context"]["at"])
}

/// songId, synced (needs a start time on every line), offset (ms), lines [{start, value}], pending.
pub fn sw_lyrics(r: &Value) -> Value {
    let mut timed = true;
    let lines: Vec<Value> = r["lines"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|o| {
                    timed = timed && o["startMs"].is_number();
                    json!({"start": o["startMs"].as_f64().unwrap_or(0.0) as i64, "value": text(&o["text"])})
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "songId": text(&r["songId"]),
        "synced": r["synced"].as_bool().unwrap_or(false) && timed && !lines.is_empty(),
        "offset": int(&r["offsetMs"]),
        "lines": lines,
        "pending": r["pending"].as_bool().unwrap_or(false),
    })
}

/// Index of the last line with start - offset <= elapsed; -1 before the first line.
pub fn sw_line_at(lines: &[Value], elapsed_ms: i64, offset_ms: i64) -> i64 {
    lines.iter().take_while(|l| l["start"].as_i64().unwrap_or(0) - offset_ms <= elapsed_ms).count() as i64 - 1
}

/// How far listeners sit behind the live edge (`stream.bufferSeconds`) in ms: 0 when missing or not a number,
/// never negative, at most two minutes.
pub fn sw_buffer_ms(np: &Value) -> i64 {
    np["stream"]["bufferSeconds"].as_f64().map_or(0, |s| ((s * 1000.0).round() as i64).clamp(0, 120_000))
}

/// Time since the track started, on the server's clock (server_at/local_at = one sample of both clocks); 0 if
/// unknown. delay_ms (the stream buffer) is taken off: that is how far into the track the listener is.
pub fn sw_elapsed_ms(started_at: i64, server_at: i64, local_at: i64, local_now: i64, delay_ms: i64) -> i64 {
    if started_at == 0 {
        return 0;
    }
    (local_now + (server_at - local_at) - started_at - delay_ms).max(0)
}

/// `radio` of the sync state: {name, title, artist}, cut to 100 / 200 / 200 characters.
pub fn sw_radio_json(station: &str, now: &Value) -> Value {
    json!({"name": cut(station, 100), "title": cut(&text(&now["title"]), 200), "artist": cut(&text(&now["artist"]), 200)})
}

/// state: pending, done (resolved), rejected, failed, unknown; plus title, artist, queuePosition, message.
pub fn sw_request(r: &Value) -> Value {
    let state = match r["status"].as_str().unwrap_or("") {
        "resolved" => "done",
        s @ ("pending" | "rejected" | "failed") => s,
        _ => "unknown",
    };
    json!({
        "state": state,
        "title": text(&r["track"]["title"]),
        "artist": text(&r["track"]["artist"]),
        "queuePosition": int(&r["queuePosition"]),
        "message": text(&r["message"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lines(starts: &[i64]) -> Vec<Value> {
        starts.iter().map(|s| json!({"start": s, "value": "x"})).collect()
    }

    fn iso(s: &str) -> i64 {
        time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).unwrap().unix_timestamp_nanos() as i64 / 1_000_000
    }

    // password: only from the userinfo, percent-decoded
    #[test]
    fn password_from_stream_url() {
        assert_eq!(sw_password("https://x:p%40ss%3Aw@radio.example.com/stream.mp3"), "p@ss:w");
        assert_eq!(sw_password("https://radio.example.com/stream.mp3"), "");
        assert_eq!(sw_password("https://onlyuser@radio.example.com/s"), "");
        // SUB/WAVE streams carry the password as ?auth=
        assert_eq!(sw_password("https://radio.example.com/stream.flac?auth=s%40cret"), "s@cret");
        assert_eq!(sw_password("https://radio.example.com/stream.flac?auth="), "");
        assert_eq!(sw_password("https://x:fromuser@radio.example.com/s?auth=other"), "fromuser"); // userinfo wins
    }

    // api url: http(s) only
    #[test]
    fn api_url_http_only() {
        assert_eq!(sw_api("https://radio.example.com/", "/now-playing").unwrap().as_str(), "https://radio.example.com/api/now-playing");
        assert_eq!(sw_api("https://h/sub/", "/state").unwrap().as_str(), "https://h/sub/api/state");
        for bad in ["", "javascript:alert(1)", "ftp://h/", "/relative"] {
            assert!(sw_api(bad, "/x").is_none(), "{bad}");
        }
    }

    #[test]
    fn detection() {
        assert!(sw_detect(&json!({"nowPlaying": {}, "dj": {}})));
        assert!(!sw_detect(&json!({})));
        assert!(!sw_detect(&json!({"nowPlaying": null, "dj": {}})));
    }

    #[test]
    fn now_playing_and_state() {
        let np = json!({"nowPlaying": {"title": "The News", "artist": "Dead Confederate", "album": "Wrecking Ball",
            "subsonic_id": "abc", "duration": 431}, "dj": {"name": "wave", "tagline": "t"}, "activeShow": null,
            "listeners": {"current": 1}, "streamOnline": true, "context": {"at": "2026-11-11T18:29:05.950Z"}});
        let st = json!({"current": {"startedAt": "2026-11-11T18:24:15.623Z"},
            "upcoming": [{"title": "U", "artist": "V", "requestedBy": null}]});
        let now = sw_now(&np, &st);
        assert!(now["id"] == "abc" && now["title"] == "The News" && now["duration"] == 431);
        assert!(now["dj"] == "wave" && now["listeners"] == 1 && now["show"] == "" && now["online"] == true);
        assert_eq!(now["startedAt"], iso("2026-11-11T18:24:15.623Z"));
        assert!(sw_server_at(&np) > now["startedAt"].as_i64().unwrap());
        // the server clock changes on every poll: it must not make two replies about the same track differ
        let mut later = np.clone();
        later["context"]["at"] = json!("2026-11-11T18:29:10.950Z");
        assert!(sw_now(&later, &st) == now && sw_server_at(&later) != sw_server_at(&np));
        assert_eq!(sw_server_at(&json!({})), 0);
        assert!(now["upcoming"].as_array().unwrap().len() == 1 && now["upcoming"][0]["title"] == "U");
        assert_eq!(now["upcoming"][0]["requestedBy"], ""); // null -> "" as toString() gave
        let empty = sw_now(&json!({}), &json!({}));
        assert!(empty["id"] == "" && empty["startedAt"] == 0 && empty["upcoming"].as_array().unwrap().is_empty());
        assert_eq!(sw_now(&json!({"nowPlaying": {}, "dj": {}, "activeShow": {"name": "Gece"}}), &json!({}))["show"], "Gece");
    }

    #[test]
    fn lyrics() {
        let ly = sw_lyrics(&json!({"songId": "abc", "synced": true, "offsetMs": -200,
            "lines": [{"startMs": 1000, "text": "a"}, {"startMs": 5000, "text": "b"}]}));
        assert!(ly["synced"] == true && ly["offset"] == -200 && ly["lines"].as_array().unwrap().len() == 2);
        assert!(ly["lines"][0]["value"] == "a" && ly["lines"][1]["start"] == 5000);
        let mixed = sw_lyrics(&json!({"songId": "abc", "synced": true,
            "lines": [{"startMs": 1000, "text": "a"}, {"startMs": null, "text": "b"}]}));
        assert_eq!(mixed["synced"], false);
        let none = sw_lyrics(&json!({"songId": null, "lines": []}));
        assert!(none["songId"] == "" && none["lines"].as_array().unwrap().is_empty() && none["synced"] == false);
    }

    #[test]
    fn current_line() {
        let two = lines(&[1000, 5000]);
        assert!(sw_line_at(&two, 500, 0) == -1 && sw_line_at(&two, 1000, 0) == 0 && sw_line_at(&two, 4999, 0) == 0);
        assert!(sw_line_at(&two, 99999, 0) == 1 && sw_line_at(&two, 800, 200) == 0 && sw_line_at(&[], 100, 0) == -1);
    }

    // elapsed time on the server's clock
    #[test]
    fn elapsed_and_buffer() {
        assert_eq!(sw_elapsed_ms(10000, 20000, 5000, 6000, 0), 11000);
        assert_eq!(sw_elapsed_ms(30000, 20000, 5000, 6000, 0), 0);
        assert_eq!(sw_elapsed_ms(0, 20000, 5000, 6000, 0), 0);
        // listeners sit behind the live edge by the stream buffer: what they hear started that much later
        assert_eq!(sw_elapsed_ms(10000, 20000, 5000, 6000, 3000), 8000);
        assert_eq!(sw_elapsed_ms(10000, 20000, 5000, 6000, 30000), 0); // the track has not reached the listener yet
        assert_eq!(sw_buffer_ms(&json!({"stream": {"bufferSeconds": 22}})), 22000);
        assert_eq!(sw_buffer_ms(&json!({"stream": {"bufferSeconds": 1.5}})), 1500);
        assert!(sw_buffer_ms(&json!({})) == 0 && sw_buffer_ms(&json!({"stream": {}})) == 0 && sw_buffer_ms(&json!({"stream": null})) == 0);
        assert!(sw_buffer_ms(&json!({"stream": {"bufferSeconds": -5}})) == 0 && sw_buffer_ms(&json!({"stream": {"bufferSeconds": "22"}})) == 0);
        assert_eq!(sw_buffer_ms(&json!({"stream": {"bufferSeconds": 5000}})), 120000); // a broken value is capped at two minutes
    }

    #[test]
    fn request_status() {
        let done = sw_request(&json!({"status": "resolved", "track": {"title": "T", "artist": "A"}, "queuePosition": 2}));
        assert!(done["state"] == "done" && done["title"] == "T" && done["artist"] == "A" && done["queuePosition"] == 2);
        assert_eq!(sw_request(&json!({"status": "pending"}))["state"], "pending");
        let rej = sw_request(&json!({"status": "rejected", "message": "no"}));
        assert!(rej["state"] == "rejected" && rej["message"] == "no");
        assert_eq!(sw_request(&json!({"status": "failed"}))["state"], "failed");
        assert_eq!(sw_request(&json!({}))["state"], "unknown");
    }

    // what the sync state says about the station: bounded, empty without a track
    #[test]
    fn radio_json_is_bounded() {
        assert_eq!(sw_radio_json("SUB/WAVE", &json!({"title": "T", "artist": "A"})), json!({"name": "SUB/WAVE", "title": "T", "artist": "A"}));
        let big = sw_radio_json(&"ş".repeat(300), &json!({"title": "x".repeat(500), "artist": "y".repeat(250)}));
        assert_eq!(big["name"].as_str().unwrap().chars().count(), 100);
        assert_eq!(big["title"].as_str().unwrap().chars().count(), 200);
        assert_eq!(big["artist"].as_str().unwrap().chars().count(), 200);
        let idle = sw_radio_json("X", &json!({}));
        assert!(idle["name"] == "X" && idle["title"] == "" && idle["artist"] == "");
    }
}
