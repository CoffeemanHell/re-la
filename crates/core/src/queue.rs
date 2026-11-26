//! Queue and API helpers: pure functions over songs as Subsonic JSON objects.
pub use rela_proto::MAX_QUEUE;
use crate::i18n::tr;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

/// Login failure codes besides Subsonic's own: no answer (network, timeout, HTTP 5xx) / an answer that is not Subsonic.
pub const NO_SERVER: i64 = -1;
pub const NOT_SUBSONIC: i64 = -2;

/// QVariant::toString() of a JSON value: strings as they are, numbers and bools as text, the rest "".
pub fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn id_of(v: &Value) -> &str {
    v["id"].as_str().unwrap_or("")
}

/// Subsonic methods QML may call through Player.get: reads only (themes are user-supplied code). The method becomes
/// a URL path segment, so only letters and digits.
pub fn readable(method: &str) -> bool {
    !method.is_empty() && method.chars().all(|c| c.is_ascii_alphanumeric()) && (method.starts_with("get") || method == "search3")
}

/// Methods QML may call through Player.set: favorites and playlist edits, nothing else.
pub fn writable(method: &str) -> bool {
    matches!(method, "star" | "unstar" | "createPlaylist" | "updatePlaylist" | "deletePlaylist" | "setRating" | "createShare")
}

/// Query pairs from QML params: list values repeat the key (songIdToAdd=a&songIdToAdd=b).
pub fn subsonic_query(params: &Map<String, Value>) -> Vec<(String, String)> {
    let mut q = vec![];
    for (k, v) in params {
        match v {
            Value::Array(a) => q.extend(a.iter().map(|x| (k.clone(), text(x)))),
            _ => q.push((k.clone(), text(v))),
        }
    }
    q
}

const UNRESERVED: &percent_encoding::AsciiSet =
    &percent_encoding::NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// The query string, fully encoded: values reach the server exactly (no bare `+`, no stray `%`).
pub fn encode_query(pairs: &[(String, String)]) -> String {
    let enc = |s: &str| percent_encoding::utf8_percent_encode(s, UNRESERVED).to_string();
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

/// url without the Subsonic login (u, t, s, p): what the cover cache keys on (the token is as good as the password).
pub fn without_login(url: &url::Url) -> url::Url {
    let mut u = url.clone();
    let kept: Vec<&str> = url
        .query()
        .unwrap_or("")
        .split('&')
        .filter(|kv| !kv.is_empty() && !matches!(kv.split('=').next(), Some("u" | "t" | "s" | "p")))
        .collect();
    u.set_query((!kept.is_empty()).then(|| kept.join("&")).as_deref());
    u
}

/// What a stream plays, from mpv's own properties and its bitrate in bit/s: "FLAC · 44,1 kHz · stereo · 890 kbps".
/// Parts mpv does not know (yet) are left out.
pub fn stream_quality_text(props: &HashMap<String, String>, bits_per_second: f64) -> String {
    let mut parts: Vec<String> = vec![];
    let codec = props.get("audio-codec-name").map(|c| c.trim().to_lowercase()).unwrap_or_default();
    if !codec.is_empty() {
        let name = match codec.as_str() {
            "flac" => "FLAC".into(),
            "mp3" => "MP3".into(),
            "opus" => "Opus".into(),
            "vorbis" => "Vorbis".into(),
            "aac" => "AAC".into(),
            "alac" => "ALAC".into(),
            _ => codec.to_uppercase(),
        };
        parts.push(name);
    }
    if let Some(rate) = props.get("audio-params/samplerate").and_then(|r| r.trim().parse::<f64>().ok()).filter(|r| *r > 0.0) {
        let k = rate / 1000.0;
        let int_digits = (k.abs().log10().floor() as i32 + 1).max(1);
        let decimals = (4 - int_digits).max(0) as usize;
        let mut t = format!("{k:.decimals$}");
        if t.contains('.') {
            t = t.trim_end_matches('0').trim_end_matches('.').to_string();
        }
        parts.push(format!("{} kHz", t.replace('.', ",")));
    }
    match props.get("audio-params/channel-count").and_then(|c| c.trim().parse::<i64>().ok()).unwrap_or(0) {
        1 => parts.push("mono".into()),
        2 => parts.push("stereo".into()),
        n if n > 2 => parts.push(format!("{n} kanal")),
        _ => {}
    }
    if bits_per_second >= 500.0 {
        parts.push(format!("{} kbps", (bits_per_second / 1000.0).round() as i64)); // 0 = not measured yet
    }
    parts.join(" · ")
}

/// scheme://host[:port] of a stream URL, without user info, path, query or fragment; "" unless http(s).
pub fn stream_origin(stream_url: &str) -> String {
    match url::Url::parse(stream_url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty()) => u.origin().ascii_serialization(),
        _ => String::new(),
    }
}

/// {id, name, url, homePage} of one getInternetRadioStations entry; {} when missing or without a stream URL.
pub fn find_station(stations: &[Value], id: &str) -> Value {
    for o in stations {
        let url = text(&o["streamUrl"]);
        if text(&o["id"]) == id && !url.is_empty() {
            let home = text(&o["homePageUrl"]);
            let home = if home.is_empty() { stream_origin(&url) } else { home };
            return json!({"id": id, "name": text(&o["name"]), "url": url, "homePage": home});
        }
    }
    json!({})
}

/// At most MAX_QUEUE songs around start; start moves to the same song inside the window.
pub fn queue_window(songs: &[Value], start: &mut usize) -> Vec<Value> {
    if songs.len() <= MAX_QUEUE {
        return songs.to_vec();
    }
    let begin = (*start).min(songs.len() - MAX_QUEUE);
    *start -= begin;
    songs[begin..begin + MAX_QUEUE].to_vec()
}

/// A Subsonic reply as themes may see it: station stream URLs (they can carry a password) replaced by their origin.
pub fn for_themes(method: &str, mut reply: Value) -> Value {
    if method != "getInternetRadioStations" || reply.get("internetRadioStations").is_none() {
        return reply;
    }
    if let Some(list) = reply["internetRadioStations"]["internetRadioStation"].as_array_mut() {
        for o in list.iter_mut().filter_map(Value::as_object_mut) {
            let origin = stream_origin(o.get("streamUrl").and_then(Value::as_str).unwrap_or(""));
            o.insert("originUrl".into(), origin.into());
            o.remove("streamUrl");
        }
    }
    reply
}

/// Inserts songs (those with an id) into queue, up to MAX_QUEUE in total. where 0/1: right after cur (at 0 when
/// cur < 0), 2: at the end. Returns the index of the first inserted song, -1 when nothing was inserted.
pub fn insert_songs(queue: &mut Vec<Value>, cur: i64, songs: &[Value], where_: i32) -> i64 {
    let mut add = vec![];
    for s in songs {
        if !id_of(s).is_empty() && queue.len() + add.len() < MAX_QUEUE {
            add.push(s.clone());
        }
    }
    if add.is_empty() {
        return -1;
    }
    let at = if where_ == 2 { queue.len() } else { (cur + 1).clamp(0, queue.len() as i64) as usize };
    queue.splice(at..at, add);
    at as i64
}

/// Shuffle mode: shuffles queue[cur+1..] only; the current song and the played part stay.
pub fn shuffle_upcoming(queue: &mut [Value], cur: i64, rng: &mut fastrand::Rng) {
    let from = (cur + 1).max(0) as usize;
    if from >= queue.len() {
        return;
    }
    rng.shuffle(&mut queue[from..]);
}

/// A whole list in random order ("Karıştır" on an album or playlist).
pub fn shuffled(mut songs: Vec<Value>) -> Vec<Value> {
    fastrand::shuffle(&mut songs);
    songs
}

/// Shuffle off: songs of upcoming found in before (by id, each once) in before's order, then the rest as they are.
pub fn unshuffle(upcoming: &[Value], before: &[Value]) -> Vec<Value> {
    let mut rest = upcoming.to_vec();
    let mut out = vec![];
    for b in before {
        if let Some(k) = rest.iter().position(|r| id_of(r) == id_of(b)) {
            out.push(rest.remove(k));
        }
    }
    out.extend(rest);
    out
}

/// Index that plays after idx: repeat 0 off (-1 past the end), 1 all (wraps), 2 one (same song).
pub fn next_index(idx: i64, size: i64, repeat: i32) -> i64 {
    if size <= 0 {
        return -1;
    }
    match repeat {
        2 => idx,
        1 => (idx + 1) % size,
        _ if idx + 1 < size => idx + 1,
        _ => -1,
    }
}

/// A play counts once half the song (at most 4 minutes) has played.
pub fn scrobble_due(pos: f64, dur: f64) -> bool {
    dur > 0.0 && pos >= (dur / 2.0).min(240.0)
}

/// What the server last heard through reportPlayback. base: ms clock time the song started when playing, else the
/// position in ms.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayReport {
    pub id: String,
    pub state: String,
    pub base: f64,
}

pub fn play_report(id: &str, state: &str, pos_ms: f64, clock_ms: f64) -> PlayReport {
    PlayReport { id: id.into(), state: state.into(), base: if state == "playing" { clock_ms - pos_ms } else { pos_ms } }
}

/// Report again on a new song or state, or when the position jumped (seek) more than 2 s.
pub fn report_due(last: &PlayReport, now: &PlayReport) -> bool {
    now.id != last.id || now.state != last.state || (now.base - last.base).abs() > 2000.0
}

/// A change goes out once it has held for hold_ms, so a burst of skips or seeks sends one report, for where it ended.
#[derive(Debug, Clone)]
pub struct ReportGate {
    pub sent: PlayReport,
    pub pending: PlayReport,
    pub pending_since: f64, // ms; -1: nothing waiting
}

impl Default for ReportGate {
    fn default() -> Self {
        ReportGate { sent: PlayReport::default(), pending: PlayReport::default(), pending_since: -1.0 }
    }
}

impl ReportGate {
    pub fn due(&mut self, now: &PlayReport, now_ms: f64, hold_ms: f64) -> bool {
        if !report_due(&self.sent, now) {
            self.pending_since = -1.0; // back where the server already is
            return false;
        }
        if self.pending_since < 0.0 || report_due(&self.pending, now) {
            self.pending = now.clone(); // a new change: wait for it to hold
            self.pending_since = now_ms;
            return false;
        }
        if now_ms - self.pending_since < hold_ms {
            return false;
        }
        self.sent = now.clone();
        self.pending_since = -1.0;
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RadioStep {
    /// tell the server the track began (servers without reportPlayback)
    pub now_playing: bool,
    /// the play counts now
    pub count: bool,
}

/// Follows the track a radio station plays, for reporting it to Navidrome and counting plays. A play counts once
/// half of it (at most 4 minutes) was heard, and only for a track followed from its start.
#[derive(Debug, Clone, Default)]
pub struct RadioPlays {
    pub id: String,
    pub from_start: bool,
    pub now_sent: bool,
    pub counted: bool,
}

impl RadioPlays {
    pub fn update(&mut self, track_id: &str, pos_ms: f64, dur_sec: f64) -> RadioStep {
        if track_id.is_empty() || pos_ms <= 0.0 {
            return RadioStep::default();
        }
        if track_id != self.id {
            // a new track reached the listener: followed from its start only if we are near it
            self.id = track_id.into();
            self.from_start = pos_ms < 10000.0;
            self.now_sent = false;
            self.counted = false;
        }
        let mut s = RadioStep::default();
        if !self.now_sent {
            s.now_playing = true;
            self.now_sent = true;
        }
        if self.from_start && !self.counted && scrobble_due(pos_ms / 1000.0, dur_sec) {
            s.count = true;
            self.counted = true;
        }
        s
    }

    pub fn reset(&mut self) {
        *self = RadioPlays::default();
    }
}

/// What to do about drift = deck position - where the shared timeline says it should be (seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    Keep,
    Steady,
    Slow,
    Fast,
    Seek,
    Reanchor,
}

/// shared (someone follows us, or we follow): Seek past 2 s, nudge the speed past 150 ms, Steady under 30 ms, Keep in
/// between. Alone nobody hears the difference: the timeline moves to the deck (Reanchor), the music never skips.
pub fn drift_action(drift: f64, shared: bool) -> Drift {
    let d = drift.abs();
    if !shared {
        return if d > 0.15 { Drift::Reanchor } else { Drift::Steady };
    }
    if d > 2.0 {
        Drift::Seek
    } else if d > 0.15 {
        if drift > 0.0 { Drift::Slow } else { Drift::Fast }
    } else if d < 0.03 {
        Drift::Steady
    } else {
        Drift::Keep
    }
}

/// Status text for a failed call: the Subsonic message, else "HTTP <code>", else the network error with the secrets
/// cut out (an error text can quote the request URL, which carries the token).
pub fn failure_text(subsonic_msg: &str, http_status: u16, net_error: &str, secrets: &[&str]) -> String {
    if !subsonic_msg.is_empty() {
        return subsonic_msg.into();
    }
    if http_status > 0 {
        return format!("HTTP {http_status}");
    }
    secrets.iter().filter(|s| !s.is_empty()).fold(net_error.to_string(), |t, s| t.replace(s, "…"))
}

/// Sets userRating on every queue entry of song id; returns how many entries changed.
pub fn apply_rating(queue: &mut [Value], id: &str, rating: i64) -> usize {
    let mut n = 0;
    for s in queue.iter_mut().filter(|s| id_of(s) == id) {
        s["userRating"] = rating.into();
        n += 1;
    }
    n
}

/// Marks every queue entry of song id as a favorite or not; returns how many entries changed.
pub fn apply_star(queue: &mut [Value], id: &str, on: bool) -> usize {
    let mut n = 0;
    for s in queue.iter_mut().filter(|s| id_of(s) == id).filter_map(Value::as_object_mut) {
        if on {
            s.insert("starred".into(), true.into()); // the server's is a date; only its being there matters
        } else {
            s.remove("starred");
        }
        n += 1;
    }
    n
}

/// DJ off: songs of similar whose id is not in queue (nor repeated), in the server's order.
pub fn similar_append(queue: &[Value], similar: &[Value]) -> Vec<Value> {
    let mut have: HashSet<String> = queue.iter().map(|s| id_of(s).to_string()).collect();
    similar.iter().filter(|s| !id_of(s).is_empty() && have.insert(id_of(s).to_string())).cloned().collect()
}

/// Server address as typed -> "https://host[:port][/path]" without a trailing slash; "" when unusable.
pub fn normalize_server(input: &str) -> String {
    let mut url = input.trim().to_string();
    if url.is_empty() {
        return url;
    }
    if !url.contains("://") {
        url.insert_str(0, "https://");
    }
    while url.ends_with('/') {
        url.pop();
    }
    match url::Url::parse(&url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty()) => url,
        _ => String::new(),
    }
}

/// The text for a failed login (code: Subsonic error code or one of NO_SERVER / NOT_SUBSONIC).
pub fn login_error(code: i64, server_msg: &str) -> String {
    match code {
        NO_SERVER => tr("Sunucuya ulaşılamıyor.", "Can't reach the server.").into(),
        NOT_SUBSONIC => tr("Bu adres bir Navidrome/Subsonic sunucusu değil.", "This address is not a Navidrome/Subsonic server.").into(),
        40 | 41 => tr("Kullanıcı adı ya da şifre yanlış.", "Wrong username or password.").into(),
        20 | 30 => tr("Sunucu sürümü uyumsuz.", "Incompatible server version.").into(),
        _ if server_msg.is_empty() => tr("Bağlanılamadı.", "Could not connect.").into(),
        _ => server_msg.into(),
    }
}

/// File name to save a download under. kind: song (original file), album / playlist (zip).
pub fn download_name(item: &Value, kind: &str) -> String {
    let name = match kind {
        "album" => format!("{} - {}.zip", text(&item["artist"]), text(&item["name"])),
        "playlist" => format!("{}.zip", text(&item["name"])),
        _ => {
            let path = text(&item["path"]);
            if !path.is_empty() {
                path.rsplit('/').next().unwrap_or("").to_string()
            } else if !text(&item["title"]).is_empty() {
                format!("{}.{}", text(&item["title"]), text(&item["suffix"]))
            } else {
                String::new()
            }
        }
    };
    if name.is_empty() || name == ".zip" || name == " - .zip" {
        return "re-la".into();
    }
    name.chars().map(|c| if r#"/\:*?"<>|"#.contains(c) { '_' } else { c }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn song(id: &str) -> Value {
        json!({"id": id, "title": id})
    }
    fn songs(ids: &str) -> Vec<Value> {
        ids.split(',').filter(|s| !s.is_empty()).map(song).collect()
    }
    fn ids(q: &[Value]) -> String {
        q.iter().map(|s| s["id"].as_str().unwrap_or("")).collect::<Vec<_>>().join(",")
    }

    #[test]
    fn readable_methods() {
        for m in ["getAlbum", "getAlbumList2", "search3"] {
            assert!(readable(m), "{m}");
        }
        for m in ["star", "deletePlaylist", "savePlayQueue", "", "get/../savePlayQueue", "getAlbum?x=1"] {
            assert!(!readable(m), "{m}");
        }
    }

    #[test]
    fn insert_songs_cases() {
        let mut q = songs("a,b,c");
        assert_eq!(insert_songs(&mut q, 0, &songs("x,y"), 1), 1);
        assert_eq!(ids(&q), "a,x,y,b,c");
        let mut q = songs("a,b");
        assert_eq!(insert_songs(&mut q, 0, &songs("x,y"), 2), 2);
        assert_eq!(ids(&q), "a,b,x,y");
        let mut q = vec![];
        assert_eq!(insert_songs(&mut q, -1, &songs("x"), 1), 0);
        assert_eq!(ids(&q), "x");
        let mut q = songs("a,b,c");
        assert_eq!(insert_songs(&mut q, 0, &songs("x"), 0), 1);
        assert_eq!(ids(&q), "a,x,b,c");
        let mut q = songs("a");
        assert_eq!(insert_songs(&mut q, 0, &[], 2), -1);
        assert_eq!(insert_songs(&mut q, 0, &[json!({"title": "no id"})], 2), -1);
        assert_eq!(ids(&q), "a");
    }

    #[test]
    fn writable_methods() {
        for m in ["star", "unstar", "createPlaylist", "updatePlaylist", "deletePlaylist", "setRating", "createShare"] {
            assert!(writable(m), "{m}");
        }
        for m in ["savePlayQueue", "deleteUser", "getAlbum", "star/../x", "", "deleteShare", "scrobble"] {
            assert!(!writable(m), "{m}");
        }
    }

    #[test]
    fn query_and_login_free_urls() {
        let p = json!({"playlistId": "7", "songIdToAdd": ["a", "b"], "name": "R&B + Şarkı"});
        let q = subsonic_query(p.as_object().unwrap());
        let vals = |k: &str| q.iter().filter(|(a, _)| a == k).map(|(_, v)| v.clone()).collect::<Vec<_>>();
        assert_eq!(vals("songIdToAdd"), ["a", "b"]);
        assert_eq!(vals("playlistId"), ["7"]);
        assert!(encode_query(&q).contains("name=R%26B%20%2B%20%C5%9Eark%C4%B1"));
        let cover = url::Url::parse("https://m.example.com/rest/getCoverArt?id=al-1&size=256&u=me&t=tok&s=salt&p=pw&v=1.16.1&c=re-la&f=json").unwrap();
        assert_eq!(without_login(&cover).as_str(), "https://m.example.com/rest/getCoverArt?id=al-1&size=256&v=1.16.1&c=re-la&f=json");
        let other = url::Url::parse("https://m.example.com/rest/getCoverArt?id=al-1&size=256&u=you&t=x&s=y&v=1.16.1&c=re-la&f=json").unwrap();
        assert_eq!(without_login(&cover), without_login(&other));
        // how the server reads a value: + is a space, then percent-decoding
        let server = |enc: &str| percent_encoding::percent_decode_str(&enc.replace('+', " ")).decode_utf8_lossy().into_owned();
        for v in ["x+y 50%", "Top %41", "%2B", "R&B + Şarkı"] {
            let enc = encode_query(&[("name".into(), v.into())]);
            assert_eq!(server(enc.strip_prefix("name=").unwrap()), v);
        }
    }

    #[test]
    fn stations() {
        let st: Vec<Value> = serde_json::from_str(r#"[{"id":"1","name":"SUB/WAVE","streamUrl":"https://r/s.mp3?auth=x","homePageUrl":"https://r.example.com"},{"id":"2","name":"Bos","streamUrl":""},{"id":"3","name":"Plain","streamUrl":"https://p/s"}]"#).unwrap();
        assert_eq!(find_station(&st, "1"), json!({"id": "1", "name": "SUB/WAVE", "url": "https://r/s.mp3?auth=x", "homePage": "https://r.example.com"}));
        assert_eq!(find_station(&st, "3")["homePage"], "https://p"); // no home page set: the stream's own origin
        assert_eq!(stream_origin("https://u:pw@radio.example.com:8443/stream.flac?auth=secret#x"), "https://radio.example.com:8443");
        assert!(stream_origin("http://h/s") == "http://h" && stream_origin("ftp://h/s").is_empty() && stream_origin("").is_empty());
        assert!(find_station(&st, "2") == json!({}) && find_station(&st, "9") == json!({}));
        let rs = json!({"internetRadioStations": {"internetRadioStation": [{"id": "1", "name": "W", "streamUrl": "https://r?auth=x", "homePageUrl": "h"}]}});
        let s0 = &for_themes("getInternetRadioStations", rs.clone())["internetRadioStations"]["internetRadioStation"][0];
        assert!(s0.get("streamUrl").is_none() && s0["name"] == "W" && s0["homePageUrl"] == "h"); // the stream URL may hold a password
        assert!(s0["originUrl"] == "https://r" && !s0["originUrl"].as_str().unwrap().contains("auth")); // only the origin goes to themes
        assert_eq!(for_themes("getAlbum", rs.clone()), rs);
    }

    #[test]
    fn queue_window_and_limit() {
        let many: Vec<Value> = (0..1200).map(|i| song(&i.to_string())).collect();
        let mut start = 1100;
        let win = queue_window(&many, &mut start);
        assert!(win.len() == MAX_QUEUE && start == 900 && win[start]["id"] == "1100");
        let mut start2 = 2;
        assert!(queue_window(&songs("a,b,c"), &mut start2).len() == 3 && start2 == 2);
        let mut full = many[..MAX_QUEUE - 1].to_vec();
        assert!(insert_songs(&mut full, 0, &songs("x,y"), 2) == (MAX_QUEUE - 1) as i64 && full.len() == MAX_QUEUE);
        assert_eq!(insert_songs(&mut full, 0, &songs("z"), 2), -1); // full: nothing fits
    }

    // shuffleUpcoming: played part and current stay, same set of songs
    #[test]
    fn shuffle_and_unshuffle() {
        let mut sq = songs("a,b,c,d,e,f");
        let mut rng = fastrand::Rng::with_seed(42);
        shuffle_upcoming(&mut sq, 1, &mut rng);
        assert!(ids(&sq).starts_with("a,b,"));
        let mut rest: Vec<String> = ids(&sq[2..]).split(',').map(str::to_string).collect();
        rest.sort();
        assert_eq!(rest.join(","), "c,d,e,f");
        let mut one = songs("a");
        shuffle_upcoming(&mut one, 0, &mut rng); // nothing upcoming: no crash
        // shuffled: the same songs, in another order (100! orders: the same one again does not happen)
        let list: Vec<Value> = (0..100).map(|i| song(&i.to_string())).collect();
        let mixed = shuffled(list.clone());
        assert_ne!(mixed, list);
        let sorted = |l: &[Value]| l.iter().map(|s| s["id"].to_string()).collect::<std::collections::BTreeSet<_>>();
        assert_eq!((mixed.len(), sorted(&mixed)), (100, sorted(&list)));
        assert_eq!(shuffled(vec![]), Vec::<Value>::new());
        assert_eq!(ids(&one), "a");
        // unshuffle: old order back, added at the end, removed dropped, duplicates each placed once
        assert_eq!(ids(&unshuffle(&songs("d,x,c"), &songs("c,d,e"))), "c,d,x");
        assert_eq!(ids(&unshuffle(&songs("b,a,b"), &songs("a,b,b"))), "a,b,b");
        assert_eq!(ids(&unshuffle(&songs("y"), &[])), "y");
    }

    #[test]
    fn next_index_and_scrobble() {
        assert_eq!(next_index(0, 3, 0), 1);
        assert_eq!(next_index(2, 3, 0), -1);
        assert_eq!(next_index(2, 3, 1), 0);
        assert_eq!(next_index(1, 3, 2), 1);
        assert_eq!(next_index(0, 1, 1), 0);
        assert_eq!(next_index(-1, 0, 1), -1);
        // half the song, at most 4 minutes in
        assert!(!scrobble_due(59.0, 120.0) && scrobble_due(60.0, 120.0));
        assert!(!scrobble_due(239.0, 1200.0) && scrobble_due(240.0, 1200.0));
        assert!(!scrobble_due(10.0, 0.0));
    }

    fn r(id: &str, state: &str, base: f64) -> PlayReport {
        PlayReport { id: id.into(), state: state.into(), base }
    }

    // a new song, a new state or a jump in position; not every tick
    #[test]
    fn report_due_cases() {
        let playing = r("a", "playing", 1000.0);
        assert!(report_due(&PlayReport::default(), &playing)); // nothing reported yet
        assert!(!report_due(&playing, &r("a", "playing", 1500.0))); // time runs on: base barely moves
        assert!(report_due(&playing, &r("a", "playing", 9000.0))); // seek
        assert!(report_due(&playing, &r("a", "paused", 1000.0))); // pause
        assert!(report_due(&playing, &r("b", "playing", 1000.0))); // next song
        assert!(!report_due(&r("a", "stopped", 0.0), &r("a", "stopped", 0.0))); // stopped once is enough
    }

    // a burst of skips sends one report, for the song it ended on, once that held for 1 s
    #[test]
    fn report_gate() {
        let mut g = ReportGate::default();
        assert!(!g.due(&r("a", "playing", 0.0), 0.0, 1000.0));
        assert!(!g.due(&r("b", "playing", 0.0), 300.0, 1000.0)); // skipped on: the wait starts again
        assert!(!g.due(&r("c", "playing", 0.0), 600.0, 1000.0));
        assert!(!g.due(&r("c", "playing", 500.0), 1500.0, 1000.0)); // c held for 0.9 s (time runs on: same report)
        assert!(g.due(&r("c", "playing", 500.0), 1600.0, 1000.0)); // 1 s: sent
        assert_eq!(g.sent.id, "c");
        assert!(!g.due(&r("c", "playing", 900.0), 5000.0, 1000.0)); // nothing new
        assert!(!g.due(&r("c", "paused", 0.0), 6000.0, 1000.0));
        assert!(!g.due(&r("c", "playing", 900.0), 6500.0, 1000.0)); // paused and resumed within the second
        assert!(!g.due(&r("c", "playing", 900.0), 9000.0, 1000.0));
    }

    // the Subsonic message, else the HTTP code; never the request URL (it carries the token)
    #[test]
    fn failure_text_cases() {
        assert_eq!(failure_text("Song not found", 404, "x", &["tok"]), "Song not found");
        assert_eq!(
            failure_text("", 502, "Error transferring https://m/rest/scrobble?u=a&t=tok&s=salt - server replied: Bad Gateway", &["tok", "salt"]),
            "HTTP 502"
        );
        let t = failure_text("", 0, "Host m not found t=tok s=salt", &["tok", "salt"]);
        assert!(!t.contains("tok") && !t.contains("salt"));
        assert_eq!(failure_text("", 0, "Connection refused", &[""]), "Connection refused");
    }

    #[test]
    fn rating_and_similar() {
        let mut rq = songs("a,b,a");
        assert_eq!(apply_rating(&mut rq, "a", 4), 2);
        assert!(rq[0]["userRating"] == 4 && rq[2]["userRating"] == 4 && rq[1].get("userRating").is_none());
        assert_eq!(apply_rating(&mut rq, "z", 3), 0);
        assert_eq!(ids(&similar_append(&songs("a,b"), &songs("c,a,d,c"))), "c,d");
        assert!(similar_append(&songs("a"), &songs("a")).is_empty());
        assert!(similar_append(&[], &[]).is_empty());
    }

    #[test]
    fn server_address_and_login_errors() {
        assert_eq!(normalize_server(" music.example.com/ "), "https://music.example.com");
        assert_eq!(normalize_server("http://10.0.0.2:4533//"), "http://10.0.0.2:4533");
        assert_eq!(normalize_server(""), "");
        assert_eq!(normalize_server("https://"), "");
        assert_eq!(normalize_server("ftp://x.com"), "");
        assert_eq!(login_error(NO_SERVER, ""), "Sunucuya ulaşılamıyor.");
        assert_eq!(login_error(NOT_SUBSONIC, ""), "Bu adres bir Navidrome/Subsonic sunucusu değil.");
        assert_eq!(login_error(40, "Wrong username or password"), "Kullanıcı adı ya da şifre yanlış.");
        assert_eq!(login_error(41, ""), "Kullanıcı adı ya da şifre yanlış.");
        assert_eq!(login_error(30, ""), "Sunucu sürümü uyumsuz.");
        assert_eq!(login_error(0, "boom"), "boom");
        assert_eq!(login_error(0, ""), "Bağlanılamadı.");
    }

    #[test]
    fn download_names() {
        assert_eq!(download_name(&json!({"path": "Supertramp/Breakfast/01 - Gone.flac"}), "song"), "01 - Gone.flac");
        assert_eq!(download_name(&json!({"title": "A/B", "suffix": "mp3"}), "song"), "A_B.mp3");
        assert_eq!(download_name(&json!({"artist": "AC/DC", "name": "Back: In Black?"}), "album"), "AC_DC - Back_ In Black_.zip");
        assert_eq!(download_name(&json!({"name": "Mix"}), "playlist"), "Mix.zip");
        assert_eq!(download_name(&json!({}), "song"), "re-la");
    }

    // reporting what a radio station plays (the track comes from SUB/WAVE, the position is the listener's)
    #[test]
    fn radio_plays() {
        assert_eq!(play_report("a", "playing", 5000.0, 100000.0).base, 95000.0); // playing: the clock time the track started
        assert_eq!(play_report("a", "paused", 5000.0, 100000.0).base, 5000.0); // otherwise: the position
        let none = |s: RadioStep| !s.now_playing && !s.count;
        let mut plays = RadioPlays::default();
        assert!(none(plays.update("a", 0.0, 200.0)) && none(plays.update("", 5000.0, 200.0)));
        assert!(plays.update("a", 3000.0, 200.0).now_playing); // the track begins: "now playing" once
        assert!(none(plays.update("a", 50000.0, 200.0)));
        assert!(plays.update("a", 100000.0, 200.0).count); // half of it: the play counts
        assert!(none(plays.update("a", 150000.0, 200.0))); // ... once
        assert!(plays.update("b", 4000.0, 200.0).now_playing && !plays.update("b", 4500.0, 200.0).count);
        let mut joined = RadioPlays::default(); // tuned in half way through: reported, never counted
        assert!(joined.update("c", 120000.0, 200.0).now_playing && none(joined.update("c", 150000.0, 200.0)));
        let mut long_track = RadioPlays::default();
        long_track.update("d", 1000.0, 600.0);
        assert!(!long_track.update("d", 239000.0, 600.0).count && long_track.update("d", 240000.0, 600.0).count);
        let mut unknown = RadioPlays::default();
        unknown.update("e", 1000.0, 0.0);
        assert!(none(unknown.update("e", 100000.0, 0.0))); // no duration: nothing to count against
        plays.reset();
        assert!(plays.update("b", 4000.0, 200.0).now_playing); // after reset the same track is new again
    }

    // what a stream plays, from mpv's own properties; unknown parts are left out
    #[test]
    fn stream_quality() {
        let p = |kv: &[(&str, &str)]| kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        let c = "audio-codec-name";
        let sr = "audio-params/samplerate";
        let ch = "audio-params/channel-count";
        assert_eq!(stream_quality_text(&p(&[(c, "flac"), (sr, "44100"), (ch, "2")]), 890000.0), "FLAC · 44,1 kHz · stereo · 890 kbps");
        assert_eq!(stream_quality_text(&p(&[(c, "mp3"), (sr, "48000"), (ch, "2")]), 0.0), "MP3 · 48 kHz · stereo");
        assert_eq!(stream_quality_text(&p(&[(c, "opus"), (sr, "22050"), (ch, "1")]), 64000.0), "Opus · 22,05 kHz · mono · 64 kbps");
        assert_eq!(stream_quality_text(&p(&[(c, "weird"), (ch, "6")]), 0.0), "WEIRD · 6 kanal");
        assert_eq!(stream_quality_text(&p(&[(sr, "96000")]), 0.0), "96 kHz");
        assert!(stream_quality_text(&p(&[]), 0.0).is_empty() && stream_quality_text(&p(&[]), 128000.0) == "128 kbps");
        assert_eq!(stream_quality_text(&p(&[(c, "flac"), (sr, "junk"), (ch, "0")]), 0.0), "FLAC");
    }

    // the deck against the timeline: followed or following, the music moves to it; alone, it moves to the music
    #[test]
    fn drift() {
        assert!(drift_action(-3600.0, true) == Drift::Seek && drift_action(2.5, true) == Drift::Seek);
        assert!(drift_action(0.5, true) == Drift::Slow && drift_action(-0.5, true) == Drift::Fast);
        assert!(drift_action(0.01, true) == Drift::Steady && drift_action(0.1, true) == Drift::Keep);
        assert!(drift_action(-3600.0, false) == Drift::Reanchor && drift_action(0.5, false) == Drift::Reanchor);
        assert!(drift_action(0.1, false) == Drift::Steady && drift_action(0.0, false) == Drift::Steady);
    }
}
