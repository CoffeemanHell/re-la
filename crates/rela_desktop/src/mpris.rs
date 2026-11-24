//! Media keys and desktop media controls: MPRIS 2 on the session bus (Linux), SMTC on Windows (`smtc.rs`). What they
//! show is worked out here for both. Commands go through the player like a button press, and only while
//! Sync.canControl. The art goes out as a local file: the cover URL carries the token.
use crate::hub::{PLAYER_QT, player, to};
use rela_core::player::PlayerSnapshot;
use rela_core::subwave::SubwaveSnapshot;
use serde_json::Value;
#[cfg(target_os = "linux")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
#[cfg(target_os = "linux")]
use zbus::object_server::SignalEmitter;
#[cfg(target_os = "linux")]
use zbus::{ObjectPath, OwnedVariant, Variant as Var};

#[cfg(target_os = "linux")]
const PATH: &str = "/org/mpris/MediaPlayer2";
#[cfg(target_os = "linux")]
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub track: String, // object path
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub length_us: Option<i64>,
    pub art: Option<String>, // file:// URL
}

/// The Player interface's properties that go out in PropertiesChanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub playback: &'static str,
    pub volume: f64, // 0..1
    pub meta: Meta,
    pub can_play: bool, // CanPlay, CanPause
    pub can_skip: bool, // CanGoNext, CanGoPrevious, CanSeek
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

pub fn status(p: &PlayerSnapshot, can_control: bool, sw: &SubwaveSnapshot, cover_file: &str) -> Status {
    let station = !p.station.is_empty();
    let song = p.current.as_object().is_some_and(|o| !o.is_empty()) && !station;
    let art = url::Url::from_file_path(cover_file).ok().map(String::from);
    let mut meta = Meta {
        track: if song || station { format!("/org/rela/track/{}", p.index.max(0)) } else { "/org/mpris/MediaPlayer2/TrackList/NoTrack".into() },
        ..Default::default()
    };
    let live = if sw.active { &sw.now } else { &Value::Null }; // SUB/WAVE: the real track
    if station && !text(live, "title").is_empty() {
        meta.title = text(live, "title");
        meta.artist = Some(text(live, "artist"));
        meta.album = Some(text(live, "album"));
        meta.art = art;
    } else if station {
        meta.title = p.station.get("name").and_then(Value::as_str).unwrap_or_default().into();
    } else if song {
        meta.title = text(&p.current, "title");
        meta.artist = Some(text(&p.current, "artist"));
        meta.album = Some(text(&p.current, "album"));
        meta.length_us = Some((p.current.get("duration").and_then(Value::as_f64).unwrap_or(0.0) * 1e6) as i64);
        meta.art = art;
    }
    Status {
        playback: if !song && !station { "Stopped" } else if p.playing { "Playing" } else { "Paused" },
        volume: p.volume / 100.0,
        meta,
        can_play: can_control && (song || station),
        can_skip: can_control && song,
    }
}

/// The property names that differ, in PropertiesChanged's terms.
pub fn changed(old: &Status, new: &Status) -> Vec<&'static str> {
    let mut out = vec![];
    if old.playback != new.playback {
        out.push("PlaybackStatus");
    }
    if old.volume != new.volume {
        out.push("Volume");
    }
    if old.meta != new.meta {
        out.push("Metadata");
    }
    if old.can_play != new.can_play {
        out.extend(["CanPlay", "CanPause"]);
    }
    if old.can_skip != new.can_skip {
        out.extend(["CanGoNext", "CanGoPrevious", "CanSeek"]);
    }
    out
}

/// Seeked when the position jumped: desktop widgets keep the time themselves otherwise. positionChanged comes ~20 times
/// a second while playing and only on a change while paused. A seek is a move more than 0.5 s off where the clock
/// says it should be; the clock runs only between two playing samples, and a playing gap over 1 s (a stall) starts it
/// again. A new song is Metadata, not a seek.
#[derive(Default)]
pub struct SeekWatch {
    last_pos: f64,
    last_song: String,
    last_playing: bool,
    last_at: Option<f64>,
}

impl SeekWatch {
    /// now: seconds on a monotonic clock. true = send Seeked.
    pub fn sample(&mut self, pos: f64, playing: bool, song: &str, now: f64) -> bool {
        let gap = self.last_at.map_or(1e9, |t| now - t);
        let mut seek = false;
        if !song.is_empty() && song == self.last_song && (gap < 1.0 || !playing) {
            let expected = self.last_pos + if playing && self.last_playing { gap } else { 0.0 };
            seek = (pos - expected).abs() > 0.5;
        }
        self.last_at = Some(now);
        self.last_pos = pos;
        self.last_playing = playing;
        self.last_song = song.into();
        seek
    }
}

// ------------------------------------------------------------------ the service

/// What the front end's listener hands over (on the player's, relay's or SUB/WAVE's thread).
pub enum Event {
    Player { position: bool, snapshot: Box<PlayerSnapshot> },
    CanControl(bool),
    Subwave(Box<SubwaveSnapshot>),
}

pub(crate) enum Msg {
    Event(Event),
    Cover(String, Option<Vec<u8>>),
    Quit(std::sync::mpsc::Sender<()>),
}

static TX: OnceLock<mpsc::UnboundedSender<Msg>> = OnceLock::new();

/// What the bus reads (property getters, commands).
pub(crate) struct Shared {
    player: PlayerSnapshot,
    can_control: bool,
    sw: SubwaveSnapshot,
    cover_file: String,
}

impl Shared {
    pub(crate) fn status(&self) -> Status {
        status(&self.player, self.can_control, &self.sw, &self.cover_file)
    }
}

/// Starts MPRIS on its own thread; the app runs on without it when there is no session bus or the name is taken.
pub fn start(cache_dir: PathBuf) {
    let (tx, rx) = mpsc::unbounded_channel();
    if TX.set(tx).is_err() {
        return;
    }
    let _ = std::thread::Builder::new().name("re-la-mpris".into()).spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("mpris runtime");
        #[cfg(target_os = "linux")]
        let r = rt.block_on(run(rx, cache_dir.join("mpris")));
        #[cfg(windows)]
        let r = rt.block_on(crate::smtc::run(rx, cache_dir.join("mpris")));
        if let Err(e) = r {
            eprintln!("media controls: {e}"); // no media keys, nothing else lost
        }
    });
}

pub fn send(e: Event) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Event(e));
    }
}

/// On quit: the cover file goes (waits up to 1 s).
pub fn quit() {
    let (done, wait) = std::sync::mpsc::channel();
    if TX.get().is_some_and(|tx| tx.send(Msg::Quit(done)).is_ok()) {
        let _ = wait.recv_timeout(Duration::from_secs(1));
    }
}

#[cfg(target_os = "linux")]
fn value<'a>(v: impl Into<Var<'a>>) -> OwnedVariant {
    OwnedVariant::try_from(v.into()).expect("no file descriptors here")
}

#[cfg(target_os = "linux")]
fn metadata(m: &Meta) -> HashMap<String, OwnedVariant> {
    let mut d = HashMap::new();
    let track = ObjectPath::try_from(m.track.as_str()).unwrap_or_else(|_| ObjectPath::from_static_str_unchecked("/"));
    d.insert("mpris:trackid".into(), value(track));
    d.insert("xesam:title".into(), value(m.title.as_str()));
    if let Some(a) = &m.artist {
        d.insert("xesam:artist".into(), value(vec![a.as_str()]));
    }
    if let Some(a) = &m.album {
        d.insert("xesam:album".into(), value(a.as_str()));
    }
    if let Some(l) = m.length_us {
        d.insert("mpris:length".into(), value(l));
    }
    if let Some(a) = &m.art {
        d.insert("mpris:artUrl".into(), value(a.as_str()));
    }
    d
}

#[cfg(target_os = "linux")]
fn property(s: &Status, name: &str) -> OwnedVariant {
    match name {
        "PlaybackStatus" => value(s.playback),
        "Volume" => value(s.volume),
        "Metadata" => value(metadata(&s.meta)),
        "CanPlay" | "CanPause" => value(s.can_play),
        _ => value(s.can_skip),
    }
}

/// The cover that goes with what plays: the song's, or SUB/WAVE's for a station ("sw:" ids).
fn cover_id(p: &PlayerSnapshot, sw: &SubwaveSnapshot) -> String {
    if p.station.is_empty() {
        return p.current.get("coverArt").and_then(Value::as_str).unwrap_or_default().into();
    }
    match sw.active.then(|| sw.now.get("id").and_then(Value::as_str)).flatten() {
        Some(id) if !id.is_empty() => format!("sw:{id}"),
        _ => String::new(),
    }
}

/// What the platform's side sends out.
pub(crate) enum Out {
    /// The position jumped (µs).
    Seeked(i64),
    /// These properties (MPRIS names) differ from what went out last.
    Changed(Vec<&'static str>, Status),
}

pub(crate) fn new_shared() -> Arc<Mutex<Shared>> {
    Arc::new(Mutex::new(Shared {
        player: PlayerSnapshot::default(),
        can_control: true,
        sw: SubwaveSnapshot::default(),
        cover_file: String::new(),
    }))
}

/// Keeps `shared` and the cover file current until Quit, and tells `out` what changed.
pub(crate) async fn serve(mut rx: mpsc::UnboundedReceiver<Msg>, dir: PathBuf, shared: Arc<Mutex<Shared>>, mut out: impl AsyncFnMut(Out)) {
    let mut sent = shared.lock().unwrap().status();
    let (mut seek, clock) = (SeekWatch::default(), Instant::now());
    let (mut cover, mut n) = (String::new(), 0u64);
    while let Some(m) = rx.recv().await {
        match m {
            Msg::Event(Event::Player { position, snapshot }) => {
                let s = *snapshot;
                let song = s.current.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
                if position && seek.sample(s.position, s.playing, &song, clock.elapsed().as_secs_f64()) {
                    out(Out::Seeked((s.position * 1e6) as i64)).await;
                }
                shared.lock().unwrap().player = s;
            }
            Msg::Event(Event::CanControl(c)) => shared.lock().unwrap().can_control = c,
            Msg::Event(Event::Subwave(s)) => shared.lock().unwrap().sw = *s,
            Msg::Cover(id, bytes) => {
                if id != cover {
                    continue; // an answer for a song that is gone
                }
                if let Some(b) = bytes.filter(|b| !b.is_empty()) {
                    n += 1; // a new name per song: desktops cache the art by its URL
                    let ext = if b.starts_with(b"\x89PNG") { "png" } else if b.get(8..12) == Some(b"WEBP") { "webp" } else { "jpg" };
                    let file = dir.join(format!("cover-{n}.{ext}"));
                    if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&file, &b).is_ok() {
                        shared.lock().unwrap().cover_file = file.to_string_lossy().into_owned();
                    }
                }
            }
            Msg::Quit(done) => {
                let _ = std::fs::remove_file(&shared.lock().unwrap().cover_file);
                let _ = done.send(());
                return;
            }
        }
        // the cover first: a change of song drops the old file before Metadata goes out
        let want = {
            let sh = shared.lock().unwrap();
            cover_id(&sh.player, &sh.sw)
        };
        if want != cover {
            cover = want.clone();
            let old = std::mem::take(&mut shared.lock().unwrap().cover_file);
            let _ = std::fs::remove_file(old);
            if !want.is_empty() {
                player().cover(want.clone(), 512, move |b| {
                    if let Some(tx) = TX.get() {
                        let _ = tx.send(Msg::Cover(want, b));
                    }
                });
            }
        }
        let now = shared.lock().unwrap().status();
        let names = changed(&sent, &now);
        if !names.is_empty() {
            out(Out::Changed(names, now.clone())).await;
        }
        sent = now;
    }
}

#[cfg(target_os = "linux")]
async fn run(rx: mpsc::UnboundedReceiver<Msg>, dir: PathBuf) -> zbus::Result<()> {
    let shared = new_shared();
    let conn = zbus::connection::Builder::session()
        .name("org.mpris.MediaPlayer2.rela")
        .serve_at(PATH, Root)
        .serve_at(PATH, PlayerIface(shared.clone()))
        .build()
        .await?;
    serve(rx, dir, shared, async |o| match o {
        Out::Seeked(at) => {
            let _ = conn.emit_signal(None::<zbus::names::BusName>, PATH, PLAYER_IFACE, "Seeked", &(at,)).await;
        }
        Out::Changed(names, now) => {
            let props: HashMap<&str, OwnedVariant> = names.iter().map(|k| (*k, property(&now, k))).collect();
            let body = (PLAYER_IFACE, props, Vec::<&str>::new());
            let _ = conn.emit_signal(None::<zbus::names::BusName>, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged", &body).await;
        }
    })
    .await;
    Ok(())
}


#[cfg(target_os = "linux")]
struct Root;

#[cfg(target_os = "linux")]
#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        to(&PLAYER_QT, |o| o.raise_requested());
    }
    fn quit(&self) {}
    #[zbus(property)]
    fn identity(&self) -> &str {
        "re-la"
    }
    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "re-la"
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec![]
    }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(target_os = "linux")]
struct PlayerIface(Arc<Mutex<Shared>>);

#[cfg(target_os = "linux")]
impl PlayerIface {
    fn now(&self) -> (Status, f64, f64) {
        let s = self.0.lock().unwrap();
        (s.status(), s.player.position, s.player.duration)
    }
}

#[cfg(target_os = "linux")]
#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerIface {
    fn play_pause(&self) {
        if self.now().0.can_play {
            player().call(|c| c.toggle());
        }
    }
    fn play(&self) {
        let s = self.now().0;
        if s.can_play && s.playback != "Playing" {
            player().call(|c| c.toggle());
        }
    }
    fn pause(&self) {
        let s = self.now().0;
        if s.can_play && s.playback == "Playing" {
            player().call(|c| c.toggle());
        }
    }
    fn stop(&self) {
        self.pause();
    }
    fn next(&self) {
        if self.now().0.can_skip {
            player().call(|c| c.next());
        }
    }
    fn previous(&self) {
        if self.now().0.can_skip {
            player().call(|c| c.prev());
        }
    }
    fn seek(&self, offset: i64) {
        let (s, pos, dur) = self.now();
        if s.can_skip {
            let to = (pos + offset as f64 / 1e6).clamp(0.0, dur.max(0.0));
            player().call(move |c| c.seek(to));
        }
    }
    fn set_position(&self, track: ObjectPath<'_>, position: i64) {
        let s = self.now().0;
        if s.can_skip && track.as_str() == s.meta.track {
            player().call(move |c| c.seek(position as f64 / 1e6));
        }
    }
    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> &str {
        self.now().0.playback
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedVariant> {
        metadata(&self.now().0.meta)
    }
    #[zbus(property)]
    fn position(&self) -> i64 {
        (self.now().1 * 1e6) as i64
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.now().0.volume
    }
    /// Desktop media controls set the app's volume, not the system's; through the QML setter, as a slider would.
    #[zbus(property)]
    fn set_volume(&mut self, v: f64) {
        let v = v.clamp(0.0, 1.0) * 100.0;
        to(&PLAYER_QT, move |o| o.set_volume(v));
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.now().0.can_play
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.now().0.can_play
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.now().0.can_skip
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.now().0.can_skip
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.now().0.can_skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    fn song(playing: bool) -> PlayerSnapshot {
        PlayerSnapshot {
            queue: Arc::new(vec![json!({}), json!({})]),
            index: 1,
            current: json!({"id": "s2", "title": "T", "artist": "A", "album": "B", "duration": 200.5, "coverArt": "c2"}),
            playing,
            volume: 40.0,
            ..Default::default()
        }
    }

    fn station() -> PlayerSnapshot {
        let mut p = song(true);
        p.station = json!({"id": "r1", "name": "Radio One"}).as_object().unwrap().clone();
        p
    }

    /// An absolute path on this platform, and its file:// URL.
    fn file(name: &str) -> (String, String) {
        let p = std::env::temp_dir().join(name);
        (p.to_string_lossy().into_owned(), url::Url::from_file_path(&p).unwrap().to_string())
    }

    #[test]
    fn status_of_a_song() {
        let (cover, url) = file("cover 1.jpg");
        let s = status(&song(false), true, &SubwaveSnapshot::default(), &cover);
        assert_eq!((s.playback, s.volume, s.can_play, s.can_skip), ("Paused", 0.4, true, true));
        assert_eq!(
            s.meta,
            Meta {
                track: "/org/rela/track/1".into(),
                title: "T".into(),
                artist: Some("A".into()),
                album: Some("B".into()),
                length_us: Some(200_500_000),
                art: Some(url),
            }
        );
    }

    #[test]
    fn status_when_nothing_plays_or_no_control() {
        let s = status(&PlayerSnapshot::default(), true, &SubwaveSnapshot::default(), "");
        assert_eq!((s.playback, s.can_play, s.meta.track.as_str()), ("Stopped", false, "/org/mpris/MediaPlayer2/TrackList/NoTrack"));
        let s = status(&song(true), false, &SubwaveSnapshot::default(), "");
        assert_eq!((s.playback, s.can_play, s.can_skip, s.meta.art), ("Playing", false, false, None));
    }

    #[test]
    fn status_of_a_station() {
        let (x, url) = file("x.jpg");
        let s = status(&station(), true, &SubwaveSnapshot::default(), &x);
        assert_eq!((s.meta.title.as_str(), s.meta.artist, s.meta.art, s.can_play, s.can_skip), ("Radio One", None, None, true, false));
        let sw = SubwaveSnapshot { active: true, now: json!({"id": "s9", "title": "Live", "artist": "L", "album": "M"}), ..Default::default() };
        let s = status(&station(), true, &sw, &x);
        assert_eq!((s.meta.title.as_str(), s.meta.artist.as_deref(), s.meta.length_us), ("Live", Some("L"), None));
        assert_eq!(s.meta.art, Some(url));
    }

    #[test]
    fn changed_names_each_property() {
        let a = status(&song(false), true, &SubwaveSnapshot::default(), "");
        assert!(changed(&a, &a).is_empty());
        let b = status(&song(true), true, &SubwaveSnapshot::default(), "");
        assert_eq!(changed(&a, &b), vec!["PlaybackStatus"]);
        let c = status(&song(true), false, &SubwaveSnapshot::default(), &file("x.jpg").0);
        assert_eq!(changed(&b, &c), vec!["Metadata", "CanPlay", "CanPause", "CanGoNext", "CanGoPrevious", "CanSeek"]);
    }

    #[test]
    fn seek_watch() {
        let mut w = SeekWatch::default();
        assert!(!w.sample(10.0, true, "s1", 0.0), "the first sample");
        assert!(!w.sample(10.05, true, "s1", 0.05), "playing on");
        assert!(w.sample(60.0, true, "s1", 0.1), "a jump");
        assert!(!w.sample(0.0, true, "s2", 0.15), "a new song is not a seek");
        assert!(!w.sample(30.0, true, "s2", 2.0), "a stall over 1 s starts the clock again");
        assert!(!w.sample(30.0, false, "s2", 2.05), "paused where it was");
        assert!(w.sample(5.0, false, "s2", 9.0), "a seek while paused");
        assert!(!w.sample(5.0, false, "s2", 20.0), "still paused");
    }
}
