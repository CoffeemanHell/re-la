//! PlayerCore: the player's state and its queue actions. It never blocks: Subsonic calls and timers go out as
//! `Job`s whose callback runs on the core again (the actor or a test runs them).
//! anchor = relay-clock ms at which the current track was at 0:00.
use super::deck::Deck;
use crate::queue::{
    find_station, insert_songs, next_index, queue_window, shuffle_upcoming, stream_quality_text, text,
    unshuffle,
};
use crate::queue::{RadioPlays, ReportGate};
use crate::dj::Brain;
use crate::dj::listen::{Heard, classify};
use crate::settings::{Settings, migrate_settings};
use crate::songs::SongCache;
use crate::subsonic::{Failure, Login, Subsonic};
use md5::{Digest, Md5};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// What the core needs from listening together (the SyncClient, or a fake).
pub trait SyncPort: Send {
    fn following(&self) -> String;
    fn has_listeners(&self) -> bool;
    /// Relay clock, ms.
    fn now(&self) -> f64;
    fn publish(&self, state: Map<String, Value>);
    fn send(&self, op: &str, args: Value);
    fn start(&self, url: String, user: String, token: String, salt: String);
    /// Leaves whom we follow; SyncEvent::Ended tells when it is done.
    fn unfollow(&self);
    fn stop(&self, bye: bool);
    fn demo(&self);
}

/// SUB/WAVE for the station that plays: the stream URL may hold a password, so it goes here and nowhere else.
pub trait StationPort: Send {
    fn open(&self, home_page: String, stream_url: String);
    fn close(&self);
}

pub type ApiThen = Box<dyn FnOnce(&mut PlayerCore, Result<Value, Failure>) + Send>;

pub enum Job {
    Api { client: Arc<Subsonic>, method: String, query: Vec<(String, String)>, then: ApiThen },
    Timer { ms: u64, then: Box<dyn FnOnce(&mut PlayerCore) + Send> },
    /// The original file (song) or a zip (album, playlist) to path; the actor reports through download_progress/_done.
    Download { client: Arc<Subsonic>, id: String, path: PathBuf },
    /// A song into the song cache; the actor reports through cache_done.
    CacheSong { client: Arc<Subsonic>, query: Vec<(String, String)>, path: PathBuf },
    /// Logout: the URLs (also CacheSong's) carry this login's token.
    CancelDownload,
    /// Slow work off the player thread; what it returns runs on the player thread when done.
    Work { run: Box<dyn FnOnce() -> Then + Send> },
}

/// What a `Job::Work` hands back to run on the player thread.
pub type Then = Box<dyn FnOnce(&mut PlayerCore) + Send>;

/// Which signal groups changed since the last take_changes.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Changes {
    pub state: bool,
    pub queue: bool,
    pub volume: bool,
    pub position: bool,
    pub status: bool,
    pub options: bool,
    pub theme: bool,
    pub stream_info: bool,
}

impl Changes {
    pub fn any(&self) -> bool {
        self.state || self.queue || self.volume || self.position || self.status || self.options || self.theme || self.stream_info
    }
    pub fn all() -> Changes {
        Changes { state: true, queue: true, volume: true, position: true, status: true, options: true, theme: true, stream_info: true }
    }
}

/// Everything the front end shows.
#[derive(Debug, Clone, Default)]
pub struct PlayerSnapshot {
    pub online: bool,
    pub offline: bool, // no server, but songs of the queue are on disk
    pub cache_bytes: u64, // what the song cache holds
    pub audio_devices: Vec<(String, String)>, // (name, description), once asked for
    pub signed_in: bool,
    pub login_failed: bool, // status says why the last connect failed
    pub status: String,
    pub user: String,
    pub queue: Arc<Vec<Value>>,
    pub index: i64,
    pub current: Value,
    pub playing: bool,
    pub crossfade: i64,
    pub exclusive: bool,
    pub dj: bool,
    pub connecting: bool,
    pub volume: f64,
    pub position: f64,
    pub duration: f64,
    pub signal_path: String,
    pub bitrate: i64,
    pub stream_info: String,
    pub options: Map<String, Value>,
    pub theme: String,
    pub station: Map<String, Value>,
    pub shuffle: bool,
    pub repeat: i64,
    /// {id, title} of an early skip that will count against the song unless taken back; null otherwise
    pub skip_undo: Value,
}

/// An early skip that counts once `Knobs::search_ms` pass without a search: it may yet be the start of one, or be
/// taken back.
pub(super) struct Held {
    pub(super) id: String,
    pub(super) day: u32,
    pub(super) at_ms: f64,
    pub(super) before: crate::dj::session::Session, // the sitting before the skip, for an undo
}

/// How long the "undo skip" notice shows.
pub(super) const UNDO_MS: f64 = 8_000.0;

/// One deck as the core sees it.
pub(super) struct DeckState {
    pub(super) out: Box<dyn Deck>,
    pub(super) id: String, // song loaded on this deck
    pub(super) pos: f64,
    pub(super) dur: f64,
    pub(super) speed: f64,
    pub(super) pending_seek: f64,
    pub(super) bitrate: f64,
    pub(super) loading: bool,
    pub(super) seeking: bool, // a seek is out: time-pos is stale until it is near pos
    pub(super) seek_at: f64,  // clock ms of that seek
    pub(super) base: f64,     // a live transcode: the second of the song its stream starts at
    pub(super) live_transcode: bool, // no seeking inside the stream (see Source)
    pub(super) failed: bool, // could not be opened while the other deck played
    pub(super) appended: bool, // gapless: next song already queued in this deck
    pub(super) appended_from_dur: f64,
    pub(super) appended_live: bool, // the appended song is a live transcode
    pub(super) appended_idx: i64, // queue index of the appended song
    pub(super) props: HashMap<String, String>, // observed audio-params/* etc.
}

impl DeckState {
    fn new(out: Box<dyn Deck>) -> DeckState {
        DeckState {
            out,
            id: String::new(),
            pos: -1.0,
            dur: 0.0,
            speed: 1.0,
            pending_seek: 0.0,
            bitrate: 0.0,
            loading: false,
            seeking: false,
            seek_at: 0.0,
            base: 0.0,
            live_transcode: false,
            failed: false,
            appended: false,
            appended_from_dur: 0.0,
            appended_live: false,
            appended_idx: -1,
            props: HashMap::new(),
        }
    }
}

/// Only what peers need to show and play a song (no server paths).
const SONG_KEYS: [&str; 14] =
    ["id", "title", "artist", "album", "albumId", "artistId", "duration", "coverArt", "suffix", "bitDepth", "samplingRate", "genre", "starred", "userRating"];

pub(super) fn song_json(song: &Value) -> Value {
    let mut o = Map::new();
    for k in SONG_KEYS {
        if let Some(v) = song.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    Value::Object(o)
}

pub(super) fn songs_json(songs: &[Value]) -> Value {
    Value::Array(songs.iter().map(song_json).collect())
}

pub(super) fn queue_hash(q: &[Value]) -> String {
    let mut ids = String::new();
    for s in q {
        ids += &text(&s["id"]);
        ids.push(',');
    }
    Md5::digest(ids.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..10].to_string()
}

pub(super) fn queue_key(q: &[Value], i: i64) -> String {
    format!("{}|{i}", queue_hash(q))
}

pub(crate) fn default_options() -> Map<String, Value> {
    let mut o = json!({
        "relayUrl": "",       // empty: <server>/re-with
        "radioLanguage": 80,  // % of radio picks in the seed's language
        "radioGenre": 50,     // seed genre preference
        "radioBatch": 10,
        "radioExplore": 20,   // % of each DJ batch kept for new or rarely played songs
        "closeToTray": false, // the tray: its icon, and closing the window keeps playing there
        "audioDevice": "auto", // the desktop's output device
        "offlineGb": 0,       // song cache limit; 0: no new songs
        "offlineAhead": 3,    // songs of the queue to have on disk ahead
        // keyboard shortcuts (themes/shared/Shortcuts.qml has the standard ones); empty: the standard one
        "keyToggle": "",
        "keyNext": "",
        "keyPrev": "",
        "keySearch": "",
        "keyBack": "",
        "keyForward": "",
        "gridSize": 172,      // album card width in px (grids)
        "columns": "artist,album,duration,year,fav,rating,bitrate", // track table columns (themes' Columns.qml keys)
        "autoContinue": true, // queue end: add songs (dj picks how)
        "scrobble": true,     // report plays
        "nowPlaying": true,   // report what is playing now
        // the Android app's look (only these keys are read back at start; the desktop ignores them)
        "amoled": false,
        "gridColumns": 2,
        "coverShape": "soft",
        "artistShape": "circle",
        "coverColors": false,
        "alphaScroll": true,
        "albumView": "grid",
        "albumOrder": "newest",
        "coverTap": "lyrics",     // Now Playing cover tap: lyrics | queue | none
        "tabs": "library,albums,playlists,artists,radio,together", // bottom bar tabs shown
        "transcodeWifi": "",      // "opus-128"; empty: the original
        "transcodeMobile": "",
        "coverQuality": "high",   // low | medium | high: cover pixels ×0.5 / ×0.75 / ×1
    })
    .as_object()
    .cloned()
    .unwrap();
    // the DJ's rules: dj::knobs::RANGES says what each one is
    o.extend(crate::dj::knobs::RANGES.iter().map(|(k, def, ..)| (k.to_string(), Value::from(*def))));
    o
}

pub struct PlayerCore {
    /// Wall clock in ms, set by whoever drives the core before each call.
    pub clock_ms: f64,
    pub(super) settings: Arc<dyn Settings>,
    pub(super) cache_dir: PathBuf,
    pub(super) decks: [DeckState; 2],
    pub(super) songs: SongCache,
    pub(super) cache_bytes: u64,
    pub(super) cache_next: f64, // clock ms; no new download before it
    pub(super) caching: bool,
    pub(super) cache_keep: HashSet<PathBuf>, // what the running download must not push out
    pub(super) held: HashSet<String>, // the cached songs, while offline
    pub(super) offline: bool,
    pub(super) mobile: bool, // on mobile data (Android)
    pub(super) audio_devices: Vec<(String, String)>,
    pub(super) cur: usize,
    pub(super) sync: Box<dyn SyncPort>,
    pub(super) station_port: Box<dyn StationPort>,
    pub(super) login: Login,
    pub(super) client: Option<Arc<Subsonic>>,
    pub(super) status: String,
    pub(super) theme: String,
    pub(super) qhash: String, // queue hash at the last sync state
    pub(super) queue: Arc<Vec<Value>>,
    pub(super) queue_sent: Arc<Vec<Value>>, // queue at the last queue change
    pub(super) pos_sent: (f64, f64, i64, String), // position, duration, bitrate, signalPath at the last position change
    pub(super) options: Map<String, Value>,
    pub(super) station: Map<String, Value>, // {id, name} while a stream plays
    pub(super) stream_info_sent: String,
    pub(super) radio_id: String,
    pub(super) radio_pos_ms: f64,
    pub(super) radio_dur: f64,
    pub(super) radio_plays: RadioPlays,
    pub(super) radio_now: Map<String, Value>, // {title, artist} the station plays now
    pub(super) want_station: String,          // station being resolved; a song command cancels it
    pub(super) resume_pos: f64,               // queue position when the stream started
    pub(super) local_key: String, // queue_key of the queue loaded from disk at start, until the server's arrives
    pub(super) local_pos: f64,
    pub(super) idx: i64,
    pub(super) xfade: i64,
    pub(super) qrev: i64,  // bumped whenever the queue changed since the last state
    pub(super) generation: u64,   // bumped by each connect/logout; stale callbacks compare and bail
    pub(super) last_error: i64,
    pub(super) last_msg: String,
    pub(super) downloading: bool,
    pub(super) volume: f64,
    pub(super) anchor: f64,
    pub(super) online: bool,
    pub(super) playing: bool,
    pub(super) leaving: bool, // pause_here while following: until Ended, the root's state may still start it again
    pub(super) fading: bool,
    pub(super) fade_start: f64,
    pub(super) fade_s: f64, // how long this fade takes
    pub(super) dj: bool,
    pub(super) exclusive: bool,
    pub(super) shuffle: bool,
    pub(super) connecting: bool,
    pub(super) fresh: bool, // login from the form
    pub(super) login_failed: bool,
    pub(super) repeat: i64,
    pub(super) failed: i32, // songs in a row that failed to open
    pub(super) extending: bool,
    pub(super) extend_seq: u64,
    pub(super) extend_retry: f64, // clock ms; extend waits until then after a call that added nothing
    pub(super) play: u64,         // play serial: bumped whenever a song starts
    pub(super) now_sent: Option<u64>,
    pub(super) scrobbled: Option<u64>,
    pub(super) playback_report: bool, // server has the OpenSubsonic playbackReport extension
    pub(super) report: ReportGate,
    pub(super) before: Vec<Value>, // upcoming songs in their order before shuffle was turned on
    pub(super) persist_seq: u64,
    pub(super) jobs: Vec<Job>,
    pub(super) changes: Changes,
    pub(super) replies: Vec<(u64, Option<Value>, bool)>,
    pub(super) rng: fastrand::Rng,
    pub(super) data_dir: PathBuf,
    pub(super) brain: Option<(String, Brain)>, // the DJ's, for the account it was opened for
    pub(super) last_full: Vec<String>,         // genres of the song before, when it was heard in full
    pub(super) memory_saved: f64,              // clock ms
    pub(super) held_skip: Option<Held>,
    pub(super) skip_undo: Value, // what the "undo skip" notice offers, while it shows
    pub(super) by_listener: bool,              // a command from someone in the room is running
    pub(super) index_busy: bool,               // the DJ's library index is being walked
    pub(super) index_next: f64,                // clock ms: no walk before it
    pub(super) genres_seq: u64,                // the latest genre rebuild asked for
}

impl PlayerCore {
    pub fn new(
        settings: Arc<dyn Settings>,
        cache_dir: PathBuf,
        data_dir: PathBuf,
        decks: [Box<dyn Deck>; 2],
        sync: Box<dyn SyncPort>,
        station_port: Box<dyn StationPort>,
    ) -> PlayerCore {
        migrate_settings(&*settings);
        let s = &settings;
        let login = Login { server: s.get_string("url"), user: s.get_string("user"), salt: s.get_string("salt"), token: s.get_string("token") };
        let exclusive = s.get_bool("exclusive", false);
        let mut options = default_options();
        for (k, def) in options.iter_mut() {
            let key = format!("options/{k}");
            if !s.contains(&key) {
                continue;
            }
            // the INI file gives text back; QML compares types
            *def = match def {
                Value::Bool(_) => s.get_bool(&key, false).into(),
                Value::Number(_) => s.get_i64(&key, 0).into(),
                _ => s.get_string(&key).into(),
            };
        }
        let [a, b] = decks;
        let songs = SongCache::open(cache_dir.join("songs"));
        let mut c = PlayerCore {
            clock_ms: 0.0,
            cache_bytes: songs.trim(u64::MAX, &HashSet::new()),
            songs,
            cache_next: 0.0,
            caching: false,
            cache_keep: HashSet::new(),
            held: HashSet::new(),
            offline: false,
            mobile: false,
            audio_devices: vec![],
            cache_dir,
            decks: [DeckState::new(a), DeckState::new(b)],
            cur: 0,
            sync,
            station_port,
            client: None,
            status: String::new(),
            theme: if s.contains("theme") { s.get_string("theme") } else if cfg!(windows) { "default".into() } else { "nothing".into() },
            qhash: String::new(),
            queue: Arc::default(),
            queue_sent: Arc::default(),
            pos_sent: (0.0, 0.0, 0, String::new()),
            options,
            station: Map::new(),
            stream_info_sent: String::new(),
            radio_id: String::new(),
            radio_pos_ms: 0.0,
            radio_dur: 0.0,
            radio_plays: RadioPlays::default(),
            radio_now: Map::new(),
            want_station: String::new(),
            resume_pos: 0.0,
            local_key: String::new(),
            local_pos: 0.0,
            idx: -1,
            xfade: if exclusive { 0 } else { s.get_i64("crossfade", 6) },
            qrev: 0,
            generation: 0,
            last_error: 0,
            last_msg: String::new(),
            downloading: false,
            volume: s.get_f64("volume", 100.0),
            anchor: 0.0,
            online: false,
            playing: false,
            leaving: false,
            fading: false,
            fade_start: 0.0,
            fade_s: 0.0,
            dj: s.get_bool("dj", false),
            exclusive,
            shuffle: s.get_bool("shuffle", false),
            connecting: false,
            fresh: false,
            login_failed: false,
            repeat: s.get_i64("repeat", 0),
            failed: 0,
            extending: false,
            extend_seq: 0,
            extend_retry: 0.0,
            play: 0,
            now_sent: None,
            scrobbled: None,
            playback_report: false,
            report: ReportGate::default(),
            before: vec![],
            persist_seq: 0,
            jobs: vec![],
            changes: Changes::default(),
            replies: vec![],
            rng: fastrand::Rng::new(),
            data_dir,
            brain: None,
            last_full: vec![],
            memory_saved: 0.0,
            held_skip: None,
            skip_undo: Value::Null,
            by_listener: false,
            index_busy: false,
            index_next: 0.0,
            genres_seq: 0,
            settings,
            login,
        };
        c.set_login(c.login.clone());
        let v = c.volume;
        let device = c.options.get("audioDevice").map(text).unwrap_or_default();
        for d in &mut c.decks {
            d.out.set_volume(v);
            if device != "auto" {
                d.out.set_audio_device(&device);
            }
            if exclusive {
                d.out.set_exclusive(true);
            }
        }
        c
    }

    pub fn take_jobs(&mut self) -> Vec<Job> {
        std::mem::take(&mut self.jobs)
    }

    /// Every queue edit ends in a state change; the queue counts as changed only when its songs differ.
    pub fn take_changes(&mut self) -> Changes {
        let mut ch = std::mem::take(&mut self.changes);
        if ch.state && !Arc::ptr_eq(&self.queue, &self.queue_sent) && *self.queue != *self.queue_sent {
            ch.queue = true;
        }
        if ch.state {
            self.queue_sent = self.queue.clone();
        }
        ch
    }

    /// (request id, reply for the theme or None for null, whether to call done at all).
    pub fn take_replies(&mut self) -> Vec<(u64, Option<Value>, bool)> {
        std::mem::take(&mut self.replies)
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            online: self.online,
            offline: self.offline,
            cache_bytes: self.cache_bytes,
            audio_devices: self.audio_devices.clone(),
            signed_in: !self.login.token.is_empty(),
            login_failed: self.login_failed,
            status: self.status.clone(),
            user: self.login.user.clone(),
            queue: self.queue.clone(),
            index: self.idx,
            current: if self.valid(self.idx) { self.queue[self.idx as usize].clone() } else { json!({}) },
            playing: self.playing,
            crossfade: self.xfade,
            exclusive: self.exclusive,
            dj: self.dj,
            connecting: self.connecting,
            volume: self.volume,
            position: self.position(),
            duration: self.duration(),
            signal_path: self.signal_path(),
            bitrate: self.bitrate(),
            stream_info: self.stream_info(),
            options: self.options.clone(),
            theme: self.theme.clone(),
            station: self.station.clone(),
            shuffle: self.shuffle,
            repeat: self.repeat,
            skip_undo: self.skip_undo.clone(),
        }
    }

    // ------------------------------------------------------------ helpers

    pub(super) fn valid(&self, i: i64) -> bool {
        i >= 0 && (i as usize) < self.queue.len()
    }
    pub(super) fn song_id(&self, i: i64) -> String {
        text(&self.queue[i as usize]["id"])
    }
    pub(super) fn song_dur(&self, i: i64) -> f64 {
        self.queue[i as usize]["duration"].as_f64().unwrap_or(0.0)
    }
    pub(super) fn following(&self) -> bool {
        !self.sync.following().is_empty()
    }
    pub(super) fn server_now(&self) -> f64 {
        self.sync.now()
    }
    pub(super) fn state_changed(&mut self) {
        self.changes.state = true;
    }

    pub(super) fn set_login(&mut self, login: Login) {
        self.client = (!login.token.is_empty() && crate::subsonic::subsonic_url(&login, "ping", &[]).is_some()).then(|| Arc::new(Subsonic::new(login.clone())));
        self.login = login;
    }

    /// raw = original file: bit-exact and seekable over HTTP ranges.
    pub fn stream_url(&self, id: &str) -> String {
        self.call_url("stream", &[("id".into(), id.into()), ("format".into(), "raw".into())])
    }

    pub(super) fn call_url(&self, method: &str, query: &[(String, String)]) -> String {
        self.client.as_ref().map_or(String::new(), |c| c.url(method, query).to_string())
    }

    /// getCoverArt for the cover cache; built without a token too (the cache keys on the URL without the login).
    pub fn cover_url(&self, id: &str, size: u32) -> Option<url::Url> {
        crate::subsonic::subsonic_url(&self.login, "getCoverArt", &[("id".into(), id.into()), ("size".into(), size.to_string())])
    }

    pub fn is_online(&self) -> bool {
        self.online
    }

    pub fn set_status(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.changes.status = true;
    }

    /// A Subsonic call; then gets the reply or None. Not quiet: a failure sets the status line and last_error.
    pub(super) fn api(
        &mut self,
        method: &str,
        query: Vec<(String, String)>,
        quiet: bool,
        then: impl FnOnce(&mut PlayerCore, Option<Value>) + Send + 'static,
    ) {
        let Some(client) = self.client.clone() else {
            // no login (demo): fails like an unreachable server would, later
            self.jobs.push(Job::Timer { ms: 0, then: Box::new(move |c| then(c, None)) });
            return;
        };
        let m = method.to_string();
        let wrapped: ApiThen = Box::new(move |c, r| match r {
            Ok(v) => then(c, Some(v)),
            Err(f) => {
                if !quiet {
                    c.last_error = f.code;
                    c.last_msg = f.message.clone();
                    let st = f.status_text(&m, &[&c.login.token, &c.login.salt]);
                    c.set_status(st);
                }
                then(c, None)
            }
        });
        self.jobs.push(Job::Api { client, method: method.into(), query, then: wrapped });
    }

    pub(super) fn timer(&mut self, ms: u64, then: impl FnOnce(&mut PlayerCore) + Send + 'static) {
        self.jobs.push(Job::Timer { ms, then: Box::new(then) });
    }

    pub(super) fn position(&self) -> f64 {
        if !self.station.is_empty() {
            return self.resume_pos; // where the queue waits
        }
        let d = &self.decks[self.cur];
        if d.loading { d.pending_seek } else { d.pos.max(0.0) }
    }

    pub(super) fn duration(&self) -> f64 {
        if !self.station.is_empty() {
            return 0.0;
        }
        let d = &self.decks[self.cur];
        if d.dur > 0.0 {
            d.dur
        } else if self.valid(self.idx) {
            self.song_dur(self.idx)
        } else {
            0.0
        }
    }

    pub(super) fn bitrate(&self) -> i64 {
        (self.decks[self.cur].bitrate / 1000.0).round() as i64
    }

    pub(super) fn stream_info(&self) -> String {
        if self.station.is_empty() { String::new() } else { stream_quality_text(&self.decks[self.cur].props, self.decks[self.cur].bitrate) }
    }

    /// e.g. "s32 96000 Hz → s32 96000 Hz · pipewire": what actually reaches the output.
    pub(super) fn signal_path(&self) -> String {
        let d = &self.decks[self.cur];
        let p = |k: &str| d.props.get(k).cloned().unwrap_or_default();
        if !d.props.contains_key("audio-params/format") {
            return String::new();
        }
        let mut s = format!(
            "{} {} Hz → {} {} Hz · {}",
            p("audio-params/format"),
            p("audio-params/samplerate"),
            p("audio-out-params/format"),
            p("audio-out-params/samplerate"),
            p("current-ao")
        );
        if self.exclusive {
            s += " exclusive";
        }
        if d.speed != 1.0 {
            s += " · resync";
        }
        if self.volume < 100.0 {
            s += " · soft volume";
        }
        s
    }

    // ------------------------------------------------------------ sync

    /// Our state as the relay keeps it.
    pub(super) fn sync_state(&mut self) -> Map<String, Value> {
        let h = queue_hash(&self.queue);
        if h != self.qhash {
            self.qhash = h;
            self.qrev += 1;
        }
        let pos = self.position();
        self.anchor = self.server_now() - pos * 1000.0;
        let mut s = json!({
            "qrev": self.qrev, "queue": songs_json(&self.queue), "index": self.idx, "playing": self.playing,
            "anchor": self.anchor, "position": pos, "xfade": self.xfade,
            "station": text(self.station.get("id").unwrap_or(&Value::Null)), "shuffle": self.shuffle, "repeat": self.repeat,
        });
        if !self.station.is_empty() {
            // for the online list, bounded (subwave::sw_radio_json)
            let name = text(&self.station["name"]);
            let now = Value::Object(self.radio_now.clone());
            s["radio"] = crate::subwave::sw_radio_json(&name, &now);
        }
        s.as_object().cloned().unwrap()
    }

    /// Our state to the relay; while following, the root's state is the truth and ours isn't sent.
    pub(super) fn publish(&mut self) {
        if self.following() {
            return;
        }
        let s = self.sync_state();
        self.sync.publish(s);
        self.persist_queue();
    }

    /// true: sent to the root because we follow.
    pub(super) fn remote(&mut self, op: &str, args: Value) -> bool {
        if !self.following() {
            return false;
        }
        self.sync.send(op, args);
        true
    }

    // ------------------------------------------------------------ persistence

    pub(super) fn play_queue_query(&self) -> Vec<(String, String)> {
        let mut q: Vec<(String, String)> = self.queue.iter().map(|s| ("id".into(), text(&s["id"]))).collect();
        if self.valid(self.idx) {
            q.push(("current".into(), self.song_id(self.idx)));
        }
        q.push(("position".into(), ((self.position() * 1000.0).round() as i64).to_string())); // ms
        q
    }

    /// savePlayQueue, debounced 5 s; skipped while following.
    pub(super) fn persist_queue(&mut self) {
        if !(self.online && !self.login.token.is_empty() && !self.following()) {
            return;
        }
        self.persist_seq += 1;
        let seq = self.persist_seq;
        self.timer(5000, move |c| {
            // checked again here: we may have started following during the 5 s (the queue is the root's now). An empty
            // queue may be one that could not be fetched: never over the server's
            if seq != c.persist_seq || !c.online || c.following() || c.queue.is_empty() {
                return;
            }
            let q = c.play_queue_query();
            c.api("savePlayQueue", q, true, |_, _| {}); // the next change saves again
            c.save_local_queue();
        });
    }

    pub(super) fn local_queue_file(&self) -> PathBuf {
        self.cache_dir.join("queue.json")
    }

    /// The last savePlayQueue, also on disk for the next start.
    pub(super) fn save_local_queue(&self) {
        let body = json!({"server": self.login.server, "user": self.login.user, "queue": *self.queue, "index": self.idx, "position": self.position()});
        let path = self.local_queue_file();
        let _ = std::fs::create_dir_all(&self.cache_dir);
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, body.to_string()).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }

    /// Playback is over (quit, logout, reports switched off): the server's now playing clears at once instead of running
    /// on. The call for the caller to send (or wait on); None when the server has nothing to clear.
    pub(super) fn report_stopped(&mut self) -> Option<Job> {
        if !self.online || self.login.token.is_empty() || self.report.sent.id.is_empty() || self.report.sent.state == "stopped" {
            return None;
        }
        self.report.sent.state = "stopped".into();
        self.report.pending_since = -1.0;
        let q = vec![
            ("mediaId".into(), self.report.sent.id.clone()),
            ("mediaType".into(), "song".into()),
            ("positionMs".into(), ((self.position() * 1000.0).round() as i64).to_string()),
            ("state".into(), "stopped".into()),
            ("ignoreScrobble".into(), "true".into()),
        ];
        Some(Job::Api { client: self.client.clone()?, method: "reportPlayback".into(), query: q, then: Box::new(|_, _| {}) })
    }

    // ------------------------------------------------------------ decks

    pub(super) fn load_deck(&mut self, k: usize, id: &str, pos: f64, paused: bool) {
        let src = self.source(id);
        let d = &mut self.decks[k];
        d.live_transcode = src.live_transcode;
        d.base = if src.live_transcode { pos.floor().max(0.0) } else { 0.0 }; // timeOffset takes whole seconds
        let url = if d.base > 0.0 { format!("{}&timeOffset={}", src.url, d.base) } else { src.url };
        d.id = id.into();
        d.pos = -1.0;
        d.dur = 0.0;
        d.pending_seek = pos;
        d.loading = true;
        d.seeking = false;
        d.appended = false;
        d.failed = false;
        d.out.load(&url, paused);
    }

    pub(super) fn load(&mut self, i: i64, pos: f64) {
        let (cur, other) = (self.cur, 1 - self.cur);
        self.fading = false;
        self.decks[other].out.stop();
        self.decks[other].id.clear();
        self.idx = i;
        let v = self.volume;
        self.decks[cur].out.set_volume(v);
        self.set_speed(cur, 1.0);
        if !self.valid(i) {
            self.decks[cur].out.stop();
            self.decks[cur].id.clear();
            self.playing = false;
        } else {
            self.play += 1;
            let id = self.song_id(i);
            let paused = !self.playing;
            self.load_deck(cur, &id, pos, paused);
            self.anchor = self.server_now() - pos * 1000.0;
        }
        self.state_changed();
    }

    pub(super) fn set_playing(&mut self, p: bool) {
        self.playing = p;
        let (cur, other, v) = (self.cur, 1 - self.cur, self.volume);
        self.decks[cur].out.set_paused(!p);
        if p && !self.fading {
            self.decks[cur].out.set_volume(v); // shortcut: unexplained "time runs, no sound" reports; a deck never stays silent after play
        }
        if p && std::env::var("RELA_AUDIO_LOG").is_ok_and(|v| v != "0") {
            // tells a silent mpv from a muted / missing PipeWire stream
            eprintln!("audio deck={cur} {} fading={}", self.decks[cur].out.audio_log(), self.fading as i32);
        }
        if !p && self.fading {
            self.fading = false;
            self.decks[other].out.stop();
            self.decks[other].id.clear();
            self.decks[cur].out.set_volume(v);
        }
        self.state_changed();
    }

    /// Next song changed: forget what was queued for gapless. (The crossfade preload is re-checked by id in tick.)
    pub(super) fn drop_prefetch(&mut self) {
        let d = &mut self.decks[self.cur];
        if d.appended {
            d.out.clear_appended();
        }
        d.appended = false;
    }

    /// The next song in another quality: what either way of preloading it holds is the old one.
    pub(super) fn drop_preload(&mut self) {
        self.drop_prefetch();
        if !self.fading {
            self.decks[1 - self.cur].id.clear(); // tick loads it again
        }
    }

    /// Every seek goes here: until the deck's position is near the target (at most 3 s), the old place or one in
    /// between is not shown.
    pub(super) fn seek_deck(&mut self, k: usize, pos: f64) {
        if self.decks[k].live_transcode {
            let (id, paused) = (self.decks[k].id.clone(), !self.playing);
            return self.load_deck(k, &id, pos, paused);
        }
        let now = self.clock_ms;
        let d = &mut self.decks[k];
        if d.loading {
            d.pending_seek = pos; // FILE_LOADED seeks there
            return;
        }
        d.out.seek(pos);
        d.pos = pos;
        d.seeking = true;
        d.seek_at = now;
    }

    pub(super) fn set_speed(&mut self, k: usize, s: f64) {
        let d = &mut self.decks[k];
        if d.speed == s {
            return;
        }
        d.speed = s;
        d.out.set_speed(s);
    }

    // ------------------------------------------------------------ UI actions

    pub(super) fn queue_edited(&mut self) {
        self.drop_prefetch();
        self.publish();
        self.state_changed();
    }

    /// Replace the queue, play from start.
    pub fn play(&mut self, songs: Vec<Value>, start: i64) {
        let mut q = vec![];
        let mut start = start.max(0) as usize;
        let win = queue_window(&songs, &mut start);
        if insert_songs(&mut q, -1, &win, 2) < 0 {
            return;
        }
        if self.remote("play", json!({"songs": songs_json(&q), "start": start})) {
            return;
        }
        let mut start = start.min(q.len() - 1);
        if self.refused_offline(&text(&q[start]["id"])) {
            return;
        }
        if self.shuffle {
            // the picked song first, the rest shuffled; off restores the list order
            let s = q.remove(start);
            q.insert(0, s);
            self.before = q[1..].to_vec();
            shuffle_upcoming(&mut q, 0, &mut self.rng);
            start = 0;
        }
        self.queue = Arc::new(q);
        self.extend_seq += 1; // picks for the old queue
        self.extending = false;
        self.extend_retry = 0.0;
        self.jump(start as i64);
    }

    /// where: 0 play now, 1 next, 2 end.
    pub fn add(&mut self, songs: Vec<Value>, where_: i32) {
        if songs.is_empty() || self.remote("add", json!({"songs": songs_json(&songs), "where": where_})) {
            return;
        }
        if (where_ == 0 || !self.valid(self.idx)) && self.refused_offline(&text(&songs[0]["id"])) {
            return;
        }
        let idx = self.idx;
        let first = insert_songs(Arc::make_mut(&mut self.queue), idx, &songs, where_);
        if first < 0 {
            return;
        }
        if where_ == 0 || !self.valid(self.idx) {
            return self.jump(first);
        }
        self.queue_edited();
    }

    /// Before anything changes: offline only the songs on disk play.
    fn refused_offline(&mut self, id: &str) -> bool {
        let no = self.offline && !SongCache::holds(&self.held, &self.login.server, id);
        if no {
            self.set_status(crate::i18n::tr("Çevrimdışı: bu şarkı inmemiş", "Offline: this song is not saved"));
        }
        no
    }

    pub fn jump(&mut self, i: i64) {
        if !self.valid(i) || self.remote("jump", json!({"i": i})) {
            return;
        }
        if self.refused_offline(&self.song_id(i)) {
            return;
        }
        self.end_station();
        self.drop_prefetch();
        self.playing = true;
        self.load(i, 0.0);
        self.set_playing(true);
        self.publish();
    }

    pub fn remove(&mut self, i: i64) {
        if !self.valid(i) || self.remote("remove", json!({"i": i})) {
            return;
        }
        Arc::make_mut(&mut self.queue).remove(i as usize);
        if i < self.idx {
            self.idx -= 1;
        } else if i == self.idx && i == self.queue.len() as i64 {
            self.playing &= !self.station.is_empty(); // the last one: the one before it waits, paused
            self.idx = i - 1;
            if self.station.is_empty() {
                self.load(self.idx, 0.0);
            }
        } else if i == self.idx && self.station.is_empty() {
            self.load(self.idx, 0.0);
        }
        self.queue_edited();
    }

    pub fn move_song(&mut self, from: i64, to: i64) {
        if !self.valid(from) || !self.valid(to) || from == to || self.remote("move", json!({"from": from, "to": to})) {
            return;
        }
        let q = Arc::make_mut(&mut self.queue);
        let s = q.remove(from as usize);
        q.insert(to as usize, s);
        if self.idx == from {
            self.idx = to;
        } else if from < self.idx && to >= self.idx {
            self.idx -= 1;
        } else if from > self.idx && to <= self.idx {
            self.idx += 1;
        }
        self.queue_edited();
    }

    pub fn clear_upcoming(&mut self) {
        if !self.valid(self.idx + 1) || self.remote("clearUpcoming", json!({})) {
            return;
        }
        let keep = (self.idx + 1) as usize;
        Arc::make_mut(&mut self.queue).truncate(keep);
        self.queue_edited();
    }

    pub fn toggle(&mut self) {
        if (!self.valid(self.idx) && self.station.is_empty()) || self.remote("toggle", json!({})) {
            return;
        }
        if !self.station.is_empty() {
            return self.stop_station(); // pausing a stream ends it; the queue waits where it was
        }
        if !self.playing && self.offline && !self.holds(self.idx) {
            let n = self.playable(self.idx, 1);
            return self.jump(if n < 0 { self.idx } else { n });
        }
        if !self.playing && self.decks[self.cur].id.is_empty() {
            return self.jump(self.idx); // the song failed to open: try it again
        }
        self.set_playing(!self.playing);
        self.publish();
    }

    /// The system takes the sound away (headphones out, another app plays, the app is swiped away): it stops here and
    /// nowhere else. A follower leaves; the root plays on.
    pub fn pause_here(&mut self) {
        if self.following() {
            self.leaving = true;
            self.sync.unfollow();
        }
        self.stop_here();
    }

    pub(super) fn stop_here(&mut self) {
        self.stop_station();
        if self.playing {
            self.set_playing(false);
            self.publish();
        }
    }

    pub fn next(&mut self) {
        let n = next_index(self.idx, self.queue.len() as i64, if self.repeat == 0 { 0 } else { 1 }); // the button always moves on
        let n = self.playable(n, 1);
        if n < 0 || self.remote("next", json!({})) {
            return;
        }
        let turn = self.heard(false);
        self.end_station();
        self.load(n, 0.0);
        if turn {
            self.drop_dj_picks();
        }
        self.publish();
    }

    pub fn prev(&mut self) {
        if !self.valid(self.idx) || self.remote("prev", json!({})) {
            return;
        }
        let restart = self.station.is_empty() && self.position() > 3.0;
        self.end_station();
        let before = self.playable((self.idx - 1).max(0), -1);
        self.load(if restart || before < 0 { self.idx } else { before }, 0.0);
        self.publish();
    }

    pub fn seek(&mut self, s: f64) {
        if self.decks[self.cur].id.is_empty() || !self.station.is_empty() || self.remote("seek", json!({"s": s})) {
            return;
        }
        self.seek_deck(self.cur, s.max(0.0)); // mpv takes a negative one from the end
        self.publish();
    }

    pub fn set_shuffle(&mut self, on: bool) {
        if on == self.shuffle || self.remote("shuffle", json!({"on": on})) {
            return;
        }
        self.shuffle = on;
        self.settings.set("shuffle", on.into());
        let at = (self.idx + 1).max(0) as usize;
        if on {
            self.before = self.queue[at..].to_vec();
            let idx = self.idx;
            shuffle_upcoming(Arc::make_mut(&mut self.queue).as_mut_slice(), idx, &mut self.rng);
        } else {
            let mut q = self.queue[..at].to_vec();
            q.extend(unshuffle(&self.queue[at..], &self.before));
            self.queue = Arc::new(q);
            self.before.clear();
        }
        self.queue_edited();
    }

    /// 0 off, 1 all, 2 one.
    pub fn set_repeat(&mut self, mode: i64) {
        let mode = mode.clamp(0, 2);
        if mode == self.repeat || self.remote("repeat", json!({"mode": mode})) {
            return;
        }
        self.repeat = mode;
        self.settings.set("repeat", mode.into());
        self.drop_prefetch(); // the song after this one may have changed
        self.publish();
        self.state_changed();
    }

    pub fn set_crossfade(&mut self, seconds: i64) {
        if self.following() {
            return; // the root's crossfade applies while following
        }
        let s = if self.exclusive { 0 } else { seconds.clamp(0, 12) };
        if s == self.xfade {
            return;
        }
        self.xfade = s;
        self.drop_prefetch();
        self.settings.set("crossfade", s.into());
        self.publish();
        self.state_changed();
    }

    /// Exclusive device access = bit-perfect, but only one deck can hold the device: crossfade off.
    pub fn set_exclusive(&mut self, on: bool) {
        if on == self.exclusive {
            return;
        }
        self.exclusive = on;
        self.settings.set("exclusive", on.into());
        if on {
            self.set_crossfade(0);
            let other = 1 - self.cur;
            self.decks[other].out.stop();
            self.decks[other].id.clear();
        }
        for d in &mut self.decks {
            d.out.set_exclusive(on);
        }
        self.state_changed();
    }

    /// The DJ's numbers as the options set them now.
    pub(super) fn knobs(&self) -> crate::dj::Knobs {
        crate::dj::Knobs::from_options(&self.options)
    }

    fn brain_key(&self) -> String {
        crate::disk::hashed(&format!("{}\n{}", self.login.server, self.login.user))
    }

    /// The signed-in account's brain if it is open (nothing is opened for a look).
    pub(super) fn dj_open(&self) -> Option<&Brain> {
        self.brain.as_ref().filter(|(k, _)| *k == self.brain_key()).map(|(_, b)| b)
    }

    /// The DJ's brain for the signed-in account; another account's is saved and closed first.
    pub(super) fn dj_brain(&mut self) -> &mut Brain {
        let key = self.brain_key();
        if self.brain.as_ref().is_none_or(|(k, _)| *k != key) {
            self.dj_flush();
            self.brain = None; // saved on drop
            let b = Brain::open_lazy(&self.cache_dir.join("dj").join(&key), &self.data_dir.join("dj").join(&key));
            self.brain = Some((key, b));
            self.last_full.clear(); // the other account's listening
            self.index_next = 0.0; // the other account's refresh deadline
            self.dj_rebuild_genres();
        }
        &mut self.brain.as_mut().unwrap().1
    }

    /// Genre closeness from the index and memory as they are now, built off the player thread (a big or oddly
    /// tagged library takes a while). It lands only if this account's brain is still open and nothing newer was asked.
    pub(super) fn dj_rebuild_genres(&mut self) {
        let (day, k) = (crate::dj::memory::day(self.clock_ms), self.knobs());
        let Some((key, b)) = self.brain.as_ref() else { return };
        let build = b.genres_later(day, &k);
        self.genres_seq += 1;
        let (key, seq) = (key.clone(), self.genres_seq);
        self.jobs.push(Job::Work {
            run: Box::new(move || {
                let g = build();
                Box::new(move |c: &mut PlayerCore| {
                    if let Some((k, b)) = c.brain.as_mut()
                        && *k == key
                        && c.genres_seq == seq
                    {
                        b.genres = g;
                    }
                })
            }),
        });
    }

    /// The current song ends (`natural`: played to its end). The DJ learns from it, unless following someone or on a
    /// station. True when its upcoming picks should make way: the listener just started looking for something else,
    /// or a song caught them while they were.
    pub(super) fn heard(&mut self, natural: bool) -> bool {
        if !self.dj || self.following() || self.by_listener || !self.station.is_empty() || !self.valid(self.idx) {
            return false;
        }
        if self.login.token.is_empty() {
            return false; // no one signed in (demo): nobody's listening to learn
        }
        let song = self.queue[self.idx as usize].clone();
        let id = text(&song["id"]);
        let (pos, dur) = (self.position(), self.duration());
        let wave = crate::waveform::cached(&self.cache_dir.join("waveforms"), &self.login.server, &id);
        let k = self.knobs();
        let h = classify(pos, dur, natural, wave.as_ref().map(|w| w.rms.as_slice()), &k);
        let ratio = if dur > 0.0 { pos / dur } else { 0.0 };
        let (now, day) = (self.clock_ms, crate::dj::memory::day(self.clock_ms));
        if self.held_skip.as_ref().is_some_and(|h| now - h.at_ms > k.search_ms) {
            self.dj_flush_held(); // no search came of it
        }
        let b = self.dj_brain();
        if b.index.get(&id).is_none() {
            b.index.upsert(&song, day);
        }
        // "less like this" already said it: leaving the song says nothing more, and it never holds a searching listener
        let less = b.memory.feedback(&id) < 0;
        let (h, ratio) = if less && h != Heard::Full { (Heard::Neutral, 0.0) } else { (h, ratio) };
        let Some(s) = b.index.get(&id).cloned() else { return false };
        let was = b.session.searching(now, &k);
        let before = b.session.clone();
        let caught = b.session.hear(&s, h, now, ratio, &b.genres, &b.index, &k);
        let searching = b.session.searching(now, &k);
        if searching {
            self.held_skip = None; // it was the start of a search: skips while looking for something else do not count
            self.set_skip_undo(Value::Null);
        } else if h == Heard::EarlySkip {
            if self.held_skip.is_some() {
                self.dj_flush_held();
            }
            self.held_skip = Some(Held { id: id.clone(), day, at_ms: now, before });
            self.set_skip_undo(json!({"id": id, "title": text(&song["title"])}));
        } else {
            b.memory.record(&id, h, day, &k);
        }
        let last_full = std::mem::take(&mut self.last_full); // after dj_brain: another account's is gone
        let b = self.dj_brain();
        let genres: Vec<String> = s.genres.iter().map(|&g| b.index.word(g).to_string()).collect();
        if h == Heard::Full {
            for a in &last_full {
                for g in genres.iter().filter(|g| *g != a) {
                    b.memory.record_pair(a, g, day, &k);
                }
            }
        }
        if now - self.memory_saved > 30_000.0 {
            let _ = self.dj_brain().memory.save();
            self.memory_saved = now;
        }
        if h == Heard::Full {
            self.last_full = genres;
        }
        caught || (searching && !was)
    }

    pub(super) fn dj_flush_held(&mut self) {
        let k = self.knobs();
        if let (Some(h), Some((_, b))) = (self.held_skip.take(), self.brain.as_mut()) {
            b.memory.record(&h.id, Heard::EarlySkip, h.day, &k);
        }
        self.set_skip_undo(Value::Null);
    }

    fn set_skip_undo(&mut self, v: Value) {
        if self.skip_undo != v {
            self.skip_undo = v;
            self.state_changed();
        }
    }

    /// The notice's time is up (tick).
    pub(super) fn expire_skip_undo(&mut self) {
        if !self.skip_undo.is_null() && self.held_skip.as_ref().is_none_or(|h| self.clock_ms - h.at_ms > UNDO_MS) {
            self.set_skip_undo(Value::Null);
        }
    }

    /// Takes back the early skip the notice offers: it will not count, and the sitting is as before it.
    pub fn undo_skip(&mut self) {
        if self.skip_undo.is_null() {
            return;
        }
        let key = self.brain_key();
        if let (Some(h), Some((k, b))) = (self.held_skip.take(), self.brain.as_mut())
            && *k == key
        {
            b.session = h.before;
        }
        self.set_skip_undo(Value::Null);
    }

    /// A held skip counts now and the memory goes to disk (quit, logout, another account).
    pub(super) fn dj_flush(&mut self) {
        self.dj_flush_held();
        if let Some((_, b)) = self.brain.as_mut() {
            let _ = b.memory.save();
        }
    }

    /// The DJ's upcoming picks go (the listener's own songs stay); tick refills the queue for the new direction.
    pub(super) fn drop_dj_picks(&mut self) {
        let at = ((self.idx + 1).max(0) as usize).min(self.queue.len());
        let mut kept = self.queue[..at].to_vec();
        kept.extend(self.queue[at..].iter().filter(|s| !s["dj"].as_bool().unwrap_or(false)).cloned());
        self.extend_seq += 1; // a request still out was picked for the old direction
        self.extending = false;
        if kept.len() != self.queue.len() {
            self.queue = Arc::new(kept);
            self.drop_prefetch();
            self.state_changed();
        }
    }

    /// "More like this" (1), "less like this" (-1: on to the next song if it is the one playing, and leaving it says
    /// nothing more) or neither (0). Kept in the DJ memory until taken back or the memory is reset.
    pub fn dj_feedback(&mut self, song: Value, dir: i8) {
        let id = text(&song["id"]);
        if id.is_empty() {
            return;
        }
        let (day, dir) = (crate::dj::memory::day(self.clock_ms), dir.clamp(-1, 1));
        let b = self.dj_brain();
        if b.index.get(&id).is_none() {
            b.index.upsert(&song, day);
        }
        b.memory.set_feedback(&id, dir, day, &text(&song["title"]), &text(&song["artist"]));
        if dir != 0
            && let Some(s) = b.index.get(&id).cloned()
        {
            b.session.nudge(&s, 3.0 * f32::from(dir), &b.genres, &b.index);
        }
        let _ = b.memory.save();
        if dir < 0 && self.valid(self.idx) && text(&self.queue[self.idx as usize]["id"]) == id {
            self.next();
        }
    }

    /// Forgets what the DJ learned from skips, listens and "more / less like this"; the library index stays.
    pub fn reset_dj_memory(&mut self) {
        let b = self.dj_brain();
        b.memory.clear();
        b.session = Default::default();
        self.held_skip = None;
        self.set_skip_undo(Value::Null);
        let _ = self.dj_brain().memory.save();
        self.dj_rebuild_genres();
        self.last_full.clear();
    }

    /// The DJ's settings back to how they came.
    pub fn reset_dj_settings(&mut self) {
        let defaults = default_options();
        for k in crate::dj::knobs::KEYS {
            self.set_option(k, defaults[*k].clone());
        }
    }

    /// Queue end: on = client-side picks, off = Navidrome's similar songs.
    pub fn set_dj(&mut self, on: bool) {
        self.dj = on;
        self.settings.set("dj", on.into());
        self.state_changed();
    }

    pub fn set_volume(&mut self, v: f64) {
        self.volume = v.clamp(0.0, 100.0);
        if !self.fading {
            let v = self.volume;
            self.decks[self.cur].out.set_volume(v);
        }
        self.settings.set("volume", self.volume.into());
        self.changes.volume = true;
    }

    pub fn set_theme(&mut self, name: &str) {
        if name == self.theme {
            return;
        }
        self.theme = name.into();
        self.settings.set("theme", name.into());
        self.changes.theme = true;
    }

    pub fn set_option(&mut self, key: &str, value: Value) {
        if self.options.get(key) == Some(&value) {
            return;
        }
        if key == "nowPlaying" && !value.as_bool().unwrap_or(true)
            && let Some(j) = self.report_stopped() {
                self.jobs.push(j);
            }
        self.settings.set(&format!("options/{key}"), value.clone());
        self.options.insert(key.into(), value.clone());
        self.changes.options = true;
        match key {
            "audioDevice" => self.decks.iter_mut().for_each(|d| d.out.set_audio_device(&text(&value))),
            "transcodeWifi" | "transcodeMobile" => self.drop_preload(),
            "offlineGb" => {
                self.cache_next = 0.0;
                if self.cache_cap() > 0 {
                    self.trim_songs(self.cache_cap(), HashSet::new());
                }
            }
            _ => {}
        }
    }

    /// Asks the output for its devices; the snapshot has them then.
    pub fn refresh_audio_devices(&mut self) {
        self.audio_devices = self.decks[0].out.audio_devices();
        self.changes.options = true;
    }

    // ------------------------------------------------------------ internet radio

    /// Internet radio; empty id ends it.
    pub fn play_station(&mut self, id: &str) {
        if self.remote("station", json!({"id": id})) {
            return;
        }
        if id.is_empty() {
            return self.stop_station();
        }
        self.open_station(id);
    }

    /// The stream URL is resolved on each device: it may carry a password, so it never reaches QML or peers.
    pub(super) fn open_station(&mut self, id: &str) {
        self.want_station = id.into();
        let (generation, id) = (self.generation, id.to_string());
        self.api("getInternetRadioStations", vec![], false, move |c, r| {
            let Some(r) = r else {
                if c.want_station == id {
                    c.want_station.clear();
                }
                return;
            };
            if generation != c.generation || c.want_station != id {
                return; // a song or another station came first
            }
            let list = r["internetRadioStations"]["internetRadioStation"].as_array().cloned().unwrap_or_default();
            let st = find_station(&list, &id);
            if st.as_object().is_none_or(Map::is_empty) {
                c.want_station.clear();
                return c.set_status(crate::i18n::tr("Yayın açılamadı", "Could not open the stream"));
            }
            c.start_station(&st);
        });
    }

    pub(super) fn start_station(&mut self, st: &Value) {
        if self.station.is_empty() && !self.following() {
            self.resume_pos = self.position(); // following: the root's, from on_state
        }
        let (cur, other, v) = (self.cur, 1 - self.cur, self.volume);
        self.fading = false;
        self.decks[other].out.stop();
        self.decks[other].id.clear();
        self.drop_prefetch();
        let d = &mut self.decks[cur];
        d.id = format!("station:{}", text(&st["id"]));
        d.pos = -1.0;
        d.dur = 0.0;
        d.pending_seek = 0.0;
        d.base = 0.0;
        d.live_transcode = false;
        d.loading = true;
        d.seeking = false;
        self.set_speed(cur, 1.0);
        let d = &mut self.decks[cur];
        d.out.set_volume(v);
        d.out.load(&text(&st["url"]), false);
        self.clear_radio(); // never the previous station's track
        self.station = json!({"id": st["id"], "name": st["name"]}).as_object().cloned().unwrap();
        self.want_station.clear();
        self.playing = true;
        self.publish();
        self.state_changed();
        self.station_port.open(text(&st["homePage"]), text(&st["url"]));
    }

    /// Back to the queue, paused where it was.
    pub(super) fn stop_station(&mut self) {
        self.want_station.clear();
        if self.station.is_empty() {
            return;
        }
        self.station.clear();
        self.clear_radio();
        self.station_port.close();
        self.playing = false;
        self.load(self.idx, self.resume_pos);
        self.publish();
    }

    /// Forget the station; the caller loads a song.
    pub(super) fn end_station(&mut self) {
        let was = !self.station.is_empty();
        self.station.clear();
        self.clear_radio();
        self.want_station.clear();
        if was {
            self.station_port.close();
        }
    }

    pub(super) fn clear_radio(&mut self) {
        self.radio_now.clear();
        self.radio_id.clear();
        self.radio_pos_ms = 0.0;
        self.radio_dur = 0.0;
        self.radio_plays.reset();
    }
}
