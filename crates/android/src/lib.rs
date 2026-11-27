//! rela-core for the Android app (UniFFI → Kotlin): the same Player the desktop drives through cxx-qt
//! (crates/rela_desktop/src/player.rs), with the same calls. Songs, replies and options cross as JSON text, as
//! QML sees them; the token, the salt and station passwords never leave Rust (stream URLs reach Kotlin only as
//! the deck's `load`/`append` argument).
#![forbid(unsafe_code)]
use rela_core::player::{Changes, Deck, DeckEvent, DeckSink, EndReason, Player, PlayerListener, PlayerSnapshot};
use rela_core::settings::{JsonSettings, Settings};
use rela_core::subwave::{SubwaveEvent, SubwaveSnapshot, local_now};
use rela_core::sync::{SyncEvent, SyncSnapshot};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

uniffi::setup_scaffolding!();

/// Rela.initTls: certificates are Android's to judge, which needs the app's context. Before the first connection.
#[cfg(target_os = "android")]
#[jni::jni_mangle("app.rela.core.Rela")]
pub fn init_tls<'a>(mut env: jni::EnvUnowned<'a>, _this: jni::objects::JObject<'a>, context: jni::objects::JObject<'a>) {
    env.with_env(|env| rustls_platform_verifier::android::init_with_env(env, context)).resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

/// The language of the core's own text (status line, login errors): "tr" Turkish, any other English, as the app's
/// string resources fall back. Before Core::new.
#[uniffi::export]
pub fn set_language(lang: String) {
    rela_core::i18n::set_language(&lang);
}

/// One ExoPlayer: Kotlin runs each call on the player's looper. Calls come from the core's thread.
#[uniffi::export(with_foreign)]
pub trait DeckOut: Send + Sync {
    fn load(&self, url: String, paused: bool);
    fn append(&self, url: String);
    fn clear_appended(&self);
    fn drop_first(&self);
    fn set_paused(&self, paused: bool);
    fn seek(&self, seconds: f64);
    /// 0..100 on mpv's cubic scale.
    fn set_volume(&self, volume: f64);
    fn set_speed(&self, speed: f64);
    fn stop(&self);
}

#[uniffi::export(with_foreign)]
pub trait DeckFactory: Send + Sync {
    fn make_deck(&self, index: u32) -> Arc<dyn DeckOut>;
}

/// Called on the core's threads; Kotlin hops to its own.
#[uniffi::export(with_foreign)]
pub trait CoreListener: Send + Sync {
    /// queue_json: the whole queue, only when it changed.
    fn on_player(&self, changes: PlayerChanges, state: PlayerState, queue_json: Option<String>);
    /// A request's answer ("null" when it failed); call false: the login changed meanwhile, drop it.
    fn on_reply(&self, id: u64, json: String, call: bool);
    fn on_sync(&self, note: SyncNote, state: SyncState);
    fn on_subwave(&self, state: SubwaveState);
    /// Four times a second while a station is active: only the elapsed time moved.
    fn on_subwave_tick(&self, elapsed_ms: i64);
}

#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq)]
pub struct PlayerChanges {
    pub state: bool,
    pub queue: bool,
    pub volume: bool,
    pub position: bool,
    pub status: bool,
    pub options: bool,
    pub stream_info: bool,
}

/// What the app shows; `current`, `station`, `options` as JSON objects.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub online: bool,
    /// No server, but songs of the queue are on disk.
    pub offline: bool,
    /// What the song cache holds.
    pub cache_bytes: u64,
    pub signed_in: bool,
    pub login_failed: bool,
    pub status: String,
    pub user: String,
    pub connecting: bool,
    pub queue_length: u32,
    pub index: i64,
    pub current_json: String,
    pub playing: bool,
    pub crossfade: i64,
    pub dj: bool,
    pub station_json: String,
    pub shuffle: bool,
    pub repeat: i64,
    pub volume: f64,
    pub position: f64,
    pub duration: f64,
    pub signal_path: String,
    pub bitrate: i64,
    pub stream_info: String,
    pub options_json: String,
    /// Android: "system", "light" or "dark" (the desktop keeps its theme folder name under the same key).
    pub theme: String,
    /// {id, title} of a skip the DJ will count unless taken back, or null.
    pub skip_undo_json: String,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum SyncNote {
    Changed,
    UsersChanged,
    Connected,
    Ended { reason: String },
    ListenerJoined { name: String },
    ListenerLeft { name: String },
    Failed { code: String },
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct SyncState {
    pub available: bool,
    pub users_json: String,
    pub listeners: Vec<String>,
    pub following: String,
    pub can_control: bool,
    pub allow_control: bool,
    pub hidden: bool,
}

/// A song's waveform from the relay: peaks and RMS, 0..1, the same length.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Wave {
    pub peaks: Vec<f32>,
    pub rms: Vec<f32>,
}

/// What the station screen shows; `elapsed_ms` taken when the event left.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct SubwaveState {
    pub active: bool,
    pub now_json: String,
    pub cover_url: String,
    pub online: bool,
    pub lyrics_json: String,
    pub lyrics_state: String,
    pub like_enabled: bool,
    pub liked: bool,
    pub like_blocked_ms: i64,
    pub request_json: String,
    pub elapsed_ms: i64,
}

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq)]
pub enum DeckEnd {
    Eof,
    Stop,
    Error,
    Other,
}

/// What an ExoPlayer saw, in mpv's terms (see rela_core::player::DeckEvent).
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum DeckNote {
    Position { seconds: Option<f64> },
    Duration { seconds: f64 },
    Bitrate { bits_per_second: Option<f64> },
    Prop { name: String, value: Option<String> },
    StartFile,
    FileLoaded,
    EndFile { reason: DeckEnd, error: String },
}

fn player_changes(c: Changes) -> PlayerChanges {
    PlayerChanges {
        state: c.state,
        queue: c.queue,
        volume: c.volume,
        position: c.position,
        status: c.status,
        options: c.options,
        stream_info: c.stream_info,
    }
}

fn player_state(s: &PlayerSnapshot) -> PlayerState {
    PlayerState {
        online: s.online,
        offline: s.offline,
        cache_bytes: s.cache_bytes,
        signed_in: s.signed_in,
        login_failed: s.login_failed,
        status: s.status.clone(),
        user: s.user.clone(),
        connecting: s.connecting,
        queue_length: s.queue.len() as u32,
        index: s.index,
        current_json: s.current.to_string(),
        playing: s.playing,
        crossfade: s.crossfade,
        dj: s.dj,
        station_json: Value::Object(s.station.clone()).to_string(),
        shuffle: s.shuffle,
        repeat: s.repeat,
        volume: s.volume,
        position: s.position,
        duration: s.duration,
        signal_path: s.signal_path.clone(),
        bitrate: s.bitrate,
        stream_info: s.stream_info.clone(),
        options_json: Value::Object(s.options.clone()).to_string(),
        theme: s.theme.clone(),
        skip_undo_json: s.skip_undo.to_string(),
    }
}

fn sync_note(e: SyncEvent) -> SyncNote {
    match e {
        SyncEvent::UsersChanged => SyncNote::UsersChanged,
        SyncEvent::Connected => SyncNote::Connected,
        SyncEvent::Ended(reason) => SyncNote::Ended { reason },
        SyncEvent::ListenerJoined(name) => SyncNote::ListenerJoined { name },
        SyncEvent::ListenerLeft(name) => SyncNote::ListenerLeft { name },
        SyncEvent::Failed(code) => SyncNote::Failed { code },
        // the player's business; the snapshot may still have moved
        SyncEvent::Changed | SyncEvent::StateReceived(_) | SyncEvent::CommandReceived { .. } => SyncNote::Changed,
    }
}

fn sync_state(s: &SyncSnapshot) -> SyncState {
    SyncState {
        available: s.available,
        users_json: Value::Array(s.users.clone()).to_string(),
        listeners: s.listeners.clone(),
        following: s.following.clone(),
        can_control: s.can_control,
        allow_control: s.allow_control,
        hidden: s.hidden,
    }
}

fn elapsed_ms(s: &SubwaveSnapshot) -> i64 {
    if s.active { s.elapsed_ms(local_now()) } else { 0 }
}

fn subwave_state(s: &SubwaveSnapshot) -> SubwaveState {
    SubwaveState {
        active: s.active,
        now_json: s.now.to_string(),
        cover_url: s.cover_url.clone(),
        online: s.online,
        lyrics_json: s.lyrics.to_string(),
        lyrics_state: s.lyrics_state.clone(),
        like_enabled: s.like_enabled,
        liked: s.liked,
        like_blocked_ms: s.like_blocked_ms,
        request_json: s.request.to_string(),
        elapsed_ms: elapsed_ms(s),
    }
}

fn deck_event(n: DeckNote) -> DeckEvent {
    match n {
        DeckNote::Position { seconds } => DeckEvent::Position(seconds),
        DeckNote::Duration { seconds } => DeckEvent::Duration(seconds),
        DeckNote::Bitrate { bits_per_second } => DeckEvent::Bitrate(bits_per_second),
        DeckNote::Prop { name, value } => DeckEvent::Prop(name, value),
        DeckNote::StartFile => DeckEvent::StartFile,
        DeckNote::FileLoaded => DeckEvent::FileLoaded,
        DeckNote::EndFile { reason, error } => DeckEvent::EndFile {
            reason: match reason {
                DeckEnd::Eof => EndReason::Eof,
                DeckEnd::Stop => EndReason::Stop,
                DeckEnd::Error => EndReason::Error,
                DeckEnd::Other => EndReason::Other,
            },
            error,
        },
    }
}

struct Front(Arc<dyn CoreListener>);

impl PlayerListener for Front {
    fn on_player(&self, changes: Changes, snapshot: PlayerSnapshot) {
        let queue = changes.queue.then(|| Value::Array(snapshot.queue.to_vec()).to_string());
        self.0.on_player(player_changes(changes), player_state(&snapshot), queue);
    }
    fn on_reply(&self, id: u64, reply: Option<Value>, call: bool) {
        self.0.on_reply(id, reply.unwrap_or(Value::Null).to_string(), call);
    }
    fn on_sync(&self, event: SyncEvent, snapshot: SyncSnapshot) {
        self.0.on_sync(sync_note(event), sync_state(&snapshot));
    }
    fn on_subwave(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot) {
        match event {
            SubwaveEvent::Changed => self.0.on_subwave(subwave_state(&snapshot)),
            SubwaveEvent::Tick => self.0.on_subwave_tick(elapsed_ms(&snapshot)),
        }
    }
}

struct Out(Arc<dyn DeckOut>);

impl Deck for Out {
    fn load(&mut self, url: &str, paused: bool) {
        self.0.load(url.into(), paused);
    }
    fn append(&mut self, url: &str) {
        self.0.append(url.into());
    }
    fn clear_appended(&mut self) {
        self.0.clear_appended();
    }
    fn drop_first(&mut self) {
        self.0.drop_first();
    }
    fn set_paused(&mut self, paused: bool) {
        self.0.set_paused(paused);
    }
    fn seek(&mut self, s: f64) {
        self.0.seek(s);
    }
    fn set_volume(&mut self, v: f64) {
        self.0.set_volume(v);
    }
    fn set_speed(&mut self, s: f64) {
        self.0.set_speed(s);
    }
    fn stop(&mut self) {
        self.0.stop();
    }
    fn set_exclusive(&mut self, _: bool) {} // desktop only
}

fn json_list(text: &str) -> Vec<Value> {
    match serde_json::from_str(text) {
        Ok(Value::Array(a)) => a,
        _ => vec![],
    }
}

/// The core of one app process: made by the playback service, dropped with it.
#[derive(uniffi::Object)]
pub struct Core {
    player: Player,
    sink: DeckSink,
}

#[uniffi::export]
impl Core {
    /// Settings in `<files_dir>/settings.json`, the DJ's memory under files_dir; queue and covers in cache_dir. A saved
    /// login connects at once.
    #[uniffi::constructor]
    pub fn new(files_dir: String, cache_dir: String, decks: Arc<dyn DeckFactory>, listener: Arc<dyn CoreListener>) -> Arc<Core> {
        let files_dir = PathBuf::from(files_dir);
        let settings: Arc<dyn Settings> = Arc::new(JsonSettings::open(files_dir.join("settings.json")));
        let sink: Arc<Mutex<Option<DeckSink>>> = Arc::default();
        let keep = sink.clone();
        let player = Player::new(settings, PathBuf::from(cache_dir), files_dir, move |i, s| {
            *keep.lock().unwrap() = Some(s);
            Box::new(Out(decks.make_deck(i as u32)))
        }, Arc::new(Front(listener)));
        let sink = sink.lock().unwrap().take().expect("Player::new makes two decks");
        Arc::new(Core { player, sink })
    }

    /// From any thread.
    pub fn deck_event(&self, deck: u32, note: DeckNote) {
        (self.sink)(deck as usize, deck_event(note));
    }

    /// Every field again (a UI that just appeared).
    pub fn refresh(&self) {
        self.player.refresh();
    }

    /// Only the server address and user name are readable (to fill the login form).
    pub fn setting(&self, key: String) -> String {
        if key != "url" && key != "user" {
            return String::new();
        }
        self.player.settings().get_string(&key)
    }

    pub fn login(&self, server: String, user: String, password: String) {
        self.player.call(move |c| c.login(&server, &user, &password));
    }
    pub fn logout(&self) {
        self.player.call(|c| c.logout());
    }

    /// A Subsonic call through the core's guards; false when the method is not allowed (no reply then).
    pub fn request(&self, id: u64, method: String, params_json: String, write: bool) -> bool {
        if !rela_core::queue::askable(&method, write) {
            return false;
        }
        let params = match serde_json::from_str(&params_json) {
            Ok(Value::Object(o)) => o,
            _ => Map::new(),
        };
        self.player.call(move |c| {
            c.request(id, &method, params, write);
        });
        true
    }

    pub fn play_json(&self, songs_json: String, start: i64) {
        let songs = json_list(&songs_json);
        self.player.call(move |c| c.play(songs, start));
    }
    /// where_: as Player.add (0 play now, 1 next, 2 end).
    pub fn add_json(&self, songs_json: String, where_: i32) {
        let songs = json_list(&songs_json);
        self.player.call(move |c| c.add(songs, where_));
    }
    pub fn play_station(&self, id: String) {
        self.player.call(move |c| c.play_station(&id));
    }
    pub fn jump(&self, i: i64) {
        self.player.call(move |c| c.jump(i));
    }
    pub fn remove(&self, i: i64) {
        self.player.call(move |c| c.remove(i));
    }
    pub fn move_song(&self, from: i64, to: i64) {
        self.player.call(move |c| c.move_song(from, to));
    }
    pub fn clear_upcoming(&self) {
        self.player.call(|c| c.clear_upcoming());
    }
    pub fn toggle(&self) {
        self.player.call(|c| c.toggle());
    }
    /// The system takes the sound away: it stops on this phone only (a follower leaves the root).
    pub fn pause_here(&self) {
        self.player.call(|c| c.pause_here());
    }
    pub fn next(&self) {
        self.player.call(|c| c.next());
    }
    pub fn prev(&self) {
        self.player.call(|c| c.prev());
    }
    pub fn seek(&self, seconds: f64) {
        self.player.call(move |c| c.seek(seconds));
    }
    pub fn set_crossfade(&self, seconds: i64) {
        self.player.call(move |c| c.set_crossfade(seconds));
    }
    pub fn set_dj(&self, on: bool) {
        self.player.call(move |c| c.set_dj(on));
    }
    pub fn set_shuffle(&self, on: bool) {
        self.player.call(move |c| c.set_shuffle(on));
    }
    pub fn set_repeat(&self, mode: i64) {
        self.player.call(move |c| c.set_repeat(mode));
    }
    pub fn set_volume(&self, volume: f64) {
        let v = volume.clamp(0.0, 100.0);
        self.player.call(move |c| c.set_volume(v));
    }
    pub fn set_option_json(&self, key: String, json: String) {
        let v = serde_json::from_str(&json).unwrap_or(Value::Null);
        self.player.call(move |c| c.set_option(&key, v));
    }
    pub fn set_theme(&self, name: String) {
        self.player.call(move |c| c.set_theme(&name));
    }
    /// On mobile data: which of the two transcode settings applies.
    pub fn set_mobile(&self, on: bool) {
        self.player.call(move |c| c.set_mobile(on));
    }
    pub fn clear_song_cache(&self) {
        self.player.call(|c| c.clear_song_cache());
    }
    pub fn reset_dj_memory(&self) {
        self.player.call(|c| c.reset_dj_memory());
    }
    pub fn reset_dj_settings(&self) {
        self.player.call(|c| c.reset_dj_settings());
    }
    /// "More like this" (1), "less like this" (-1), neither (0).
    pub fn dj_feedback(&self, song_json: String, dir: i8) {
        let song = serde_json::from_str(&song_json).unwrap_or(Value::Null);
        self.player.call(move |c| c.dj_feedback(song, dir));
    }
    pub fn undo_skip(&self) {
        self.player.call(|c| c.undo_skip());
    }
    pub fn demo(&self) {
        self.player.call(|c| c.demo());
    }
    /// Saves the play queue and clears now playing; blocks up to 2 s (the service is stopping).
    pub fn persist_now(&self) {
        self.player.persist_now();
    }

    /// Cover art bytes from the cache or the server (size: the longer side in px); "sw:<id>" for SUB/WAVE.
    pub async fn cover(&self, id: String, size: u32) -> Option<Vec<u8>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.player.cover(id, size, move |b| {
            let _ = tx.send(b);
        });
        rx.await.ok().flatten()
    }

    /// The song's waveform from disk or the relay; None for SUB/WAVE, or when the relay has none (yet).
    pub async fn waveform(&self, id: String) -> Option<Wave> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.player.waveform(id, move |w| {
            let _ = tx.send(w.map(|w| Wave { peaks: w.peaks, rms: w.rms }));
        });
        rx.await.ok().flatten()
    }

    /// What the server transcodes to: its profiles ("opus-320"), or formats alone ("opus") when nothing tells the
    /// profiles; empty offline or when it can't tell.
    pub async fn transcode_formats(&self) -> Vec<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.player.transcode_formats(move |f| {
            let _ = tx.send(f);
        });
        rx.await.unwrap_or_default()
    }

    pub fn follow(&self, user: String) {
        self.player.sync().follow(&user);
    }
    pub fn unfollow(&self) {
        self.player.sync().unfollow();
    }
    pub fn set_allow_control(&self, on: bool) {
        self.player.sync().set_allow_control(on);
        self.player.settings().set("sync/allowControl", on.into());
    }
    pub fn set_hidden(&self, on: bool) {
        self.player.sync().set_hidden(on);
        self.player.settings().set("sync/hidden", on.into());
    }
    /// The relay goes away (bye: tell followers).
    pub fn stop_sync(&self, bye: bool) {
        self.player.sync().stop(bye);
    }

    pub fn subwave_like(&self) {
        self.player.subwave().like();
    }
    pub fn subwave_request(&self, text: String, name: String) {
        self.player.subwave().send_request(text, name);
    }
}

#[cfg(test)]
mod tests;
