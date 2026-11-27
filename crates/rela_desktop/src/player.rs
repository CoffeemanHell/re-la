//! `PlayerCore`: the properties and calls over the Rust player. QML sees it as `Player` through
//! qml/Player.qml, which adds get/set (their `done` callbacks live in JS) and the calls that take JS values
//! (passed here as JSON text). Values flow one way: the core's snapshots set the properties; a setter asks the core.
use crate::hub::{PLAYER_QT, player};
use crate::qt::{to_qvariant, to_qvariant_map};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use rela_core::player::{Changes, PlayerSnapshot};
use serde_json::{Map, Value, json};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qmap.h");
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
        include!("cxx-qt-lib/qlist.h");
        type QList_QVariant = cxx_qt_lib::QList<cxx_qt_lib::QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element] // the base of Player.qml, which themes see as Player
        #[qproperty(bool, online, READ, NOTIFY = status_changed)]
        #[qproperty(bool, offline, READ, NOTIFY = status_changed)] // no server, but songs of the queue are on disk
        #[qproperty(bool, signed_in, cxx_name = "signedIn", READ, NOTIFY = status_changed)] // has a saved token; may still be connecting
        #[qproperty(bool, login_failed, cxx_name = "loginFailed", READ, NOTIFY = status_changed)] // status says why the last connect failed
        #[qproperty(QString, status, READ, NOTIFY = status_changed)]
        #[qproperty(QString, user, READ, NOTIFY = status_changed)]
        #[qproperty(bool, connecting, READ, NOTIFY = status_changed)]
        #[qproperty(QList_QVariant, queue, READ, NOTIFY = queue_changed)] // each read copies every song into JS: prefer queueLength
        #[qproperty(i32, queue_length, cxx_name = "queueLength", READ, NOTIFY = queue_changed)]
        #[qproperty(i32, index, READ, NOTIFY = state_changed)]
        #[qproperty(QMap_QString_QVariant, current, READ, NOTIFY = state_changed)]
        #[qproperty(bool, playing, READ, NOTIFY = state_changed)]
        #[qproperty(i32, crossfade, READ, WRITE = set_crossfade, NOTIFY = state_changed)]
        #[qproperty(bool, exclusive, READ, WRITE = set_exclusive, NOTIFY = state_changed)]
        #[qproperty(bool, dj, READ, WRITE = set_dj, NOTIFY = state_changed)] // queue end: on = client-side picks, off = similar songs
        #[qproperty(QMap_QString_QVariant, station, READ, NOTIFY = state_changed)] // {id, name} of the playing internet radio
        #[qproperty(bool, shuffle, READ, WRITE = set_shuffle, NOTIFY = state_changed)]
        #[qproperty(i32, repeat, READ, WRITE = set_repeat, NOTIFY = state_changed)] // 0 off, 1 all, 2 one
        #[qproperty(f64, volume, READ, WRITE = set_volume, NOTIFY = volume_changed)]
        #[qproperty(f64, position, READ, NOTIFY = position_changed)]
        #[qproperty(f64, duration, READ, NOTIFY = position_changed)]
        #[qproperty(QString, signal_path, cxx_name = "signalPath", READ, NOTIFY = position_changed)]
        #[qproperty(i32, bitrate, READ, NOTIFY = position_changed)] // kbps of the playing audio, 0 unknown
        #[qproperty(QString, stream_info, cxx_name = "streamInfo", READ, NOTIFY = stream_info_changed)]
        #[qproperty(QList_QVariant, waveform, READ, NOTIFY = waveform_changed)] // the relay's, empty until it comes
        #[qproperty(QList_QVariant, waveform_rms, cxx_name = "waveformRms", READ, NOTIFY = waveform_changed)]
        #[qproperty(QMap_QString_QVariant, options, READ, NOTIFY = options_changed)]
        #[qproperty(f64, cache_bytes, cxx_name = "cacheBytes", READ, NOTIFY = options_changed)] // what the song cache holds
        #[qproperty(QList_QVariant, audio_devices, cxx_name = "audioDevices", READ, NOTIFY = options_changed)] // [{name, description}], after refreshAudioDevices
        #[qproperty(QString, theme, READ, WRITE = set_theme, NOTIFY = theme_changed)]
        #[cxx_name = "PlayerCore"]
        type RustPlayer = super::PlayerRust;
    }

    unsafe extern "RustQt" {
        #[qsignal]
        #[cxx_name = "stateChanged"]
        fn state_changed(self: Pin<&mut RustPlayer>);
        /// After stateChanged, only when the songs differ from the last time.
        #[qsignal]
        #[cxx_name = "queueChanged"]
        fn queue_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "volumeChanged"]
        fn volume_changed(self: Pin<&mut RustPlayer>);
        /// Only when position, duration, bitrate or signalPath changed.
        #[qsignal]
        #[cxx_name = "positionChanged"]
        fn position_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "statusChanged"]
        fn status_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "optionsChanged"]
        fn options_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "themeChanged"]
        fn theme_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "waveformChanged"]
        fn waveform_changed(self: Pin<&mut RustPlayer>);
        #[qsignal]
        #[cxx_name = "streamInfoChanged"]
        fn stream_info_changed(self: Pin<&mut RustPlayer>);
        /// A request's answer as JSON ("null" for null); call false: the login changed, drop the callback.
        #[qsignal]
        fn replied(self: Pin<&mut RustPlayer>, id: i32, json: QString, call: bool);
        /// MPRIS Raise: bring the window back.
        #[qsignal]
        #[cxx_name = "raiseRequested"]
        fn raise_requested(self: Pin<&mut RustPlayer>);

        #[cxx_name = "setCrossfade"]
        fn set_crossfade(self: Pin<&mut RustPlayer>, seconds: i32);
        #[cxx_name = "setExclusive"]
        fn set_exclusive(self: Pin<&mut RustPlayer>, on: bool);
        #[cxx_name = "setDj"]
        fn set_dj(self: Pin<&mut RustPlayer>, on: bool);
        #[cxx_name = "setShuffle"]
        fn set_shuffle(self: Pin<&mut RustPlayer>, on: bool);
        #[cxx_name = "setRepeat"]
        fn set_repeat(self: Pin<&mut RustPlayer>, mode: i32);
        #[cxx_name = "setVolume"]
        fn set_volume(self: Pin<&mut RustPlayer>, v: f64);
        #[cxx_name = "setTheme"]
        fn set_theme(self: Pin<&mut RustPlayer>, name: QString);

        /// Only what the login form shows (url, user): themes are user code and must not read the token or salt.
        #[qinvokable]
        fn setting(self: &RustPlayer, key: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "setOptionJson"]
        fn set_option_json(self: &RustPlayer, key: &QString, json: &QString);
        #[qinvokable]
        #[cxx_name = "refreshAudioDevices"]
        fn refresh_audio_devices(self: &RustPlayer);
        #[qinvokable]
        #[cxx_name = "clearSongCache"]
        fn clear_song_cache(self: &RustPlayer);
        #[qinvokable]
        #[cxx_name = "resetDjMemory"]
        fn reset_dj_memory(self: &RustPlayer);
        #[qinvokable]
        fn login(self: &RustPlayer, server: &QString, user: &QString, password: &QString);
        #[qinvokable]
        fn logout(self: &RustPlayer);
        /// Player.get/set: false = not allowed (the wrapper calls done(null) at once); else the answer comes by replied(id).
        #[qinvokable]
        fn request(self: &RustPlayer, id: i32, method: &QString, params_json: &QString, write: bool) -> bool;
        #[qinvokable]
        #[cxx_name = "playJson"]
        fn play_json(self: &RustPlayer, songs_json: &QString, start: i32);
        #[qinvokable]
        #[cxx_name = "addJson"]
        fn add_json(self: &RustPlayer, songs_json: &QString, where_: i32);
        #[qinvokable]
        #[cxx_name = "playStation"]
        fn play_station(self: &RustPlayer, id: &QString);
        #[qinvokable]
        fn jump(self: &RustPlayer, i: i32);
        #[qinvokable]
        fn remove(self: &RustPlayer, i: i32);
        #[qinvokable]
        #[cxx_name = "move"]
        fn move_song(self: &RustPlayer, from: i32, to: i32);
        #[qinvokable]
        #[cxx_name = "clearUpcoming"]
        fn clear_upcoming(self: &RustPlayer);
        #[qinvokable]
        fn toggle(self: &RustPlayer);
        #[qinvokable]
        fn next(self: &RustPlayer);
        #[qinvokable]
        fn prev(self: &RustPlayer);
        #[qinvokable]
        fn seek(self: Pin<&mut RustPlayer>, seconds: f64);
        /// file: a file:// URL (FileDialog's selectedFile as text).
        #[qinvokable]
        #[cxx_name = "downloadTo"]
        fn download_to(self: &RustPlayer, id: &QString, file_url: &QString);
        /// The songs in random order (Util.shuffled).
        #[qinvokable]
        #[cxx_name = "shuffledJson"]
        fn shuffled_json(self: &RustPlayer, songs_json: &QString) -> QString;
        /// The file name a download is saved under (Util.fileName). kind: song, album, playlist.
        #[qinvokable]
        #[cxx_name = "downloadNameJson"]
        fn download_name_json(self: &RustPlayer, item_json: &QString, kind: &QString) -> QString;
        /// On quit: our play queue to the server, now playing cleared; blocks up to 1.5 s.
        #[qinvokable]
        #[cxx_name = "persistNow"]
        fn persist_now(self: &RustPlayer);
        /// true once ever: the tray's "still playing" hint.
        #[qinvokable]
        #[cxx_name = "trayHint"]
        fn tray_hint(self: &RustPlayer) -> bool;
        /// --demo: sample songs, no server.
        #[qinvokable]
        fn demo(self: &RustPlayer);
        #[qinvokable]
        #[cxx_name = "setStatus"]
        fn set_status(self: &RustPlayer, text: &QString);
    }

    impl cxx_qt::Threading for RustPlayer {}
    impl cxx_qt::Initialize for RustPlayer {}
}

#[derive(Default)]
pub struct PlayerRust {
    online: bool,
    offline: bool,
    cache_bytes: f64,
    audio_devices: QList<QVariant>,
    signed_in: bool,
    login_failed: bool,
    status: QString,
    user: QString,
    connecting: bool,
    queue: QList<QVariant>,
    queue_length: i32,
    index: i32,
    current: QMap<QMapPair_QString_QVariant>,
    playing: bool,
    crossfade: i32,
    exclusive: bool,
    dj: bool,
    station: QMap<QMapPair_QString_QVariant>,
    shuffle: bool,
    repeat: i32,
    volume: f64,
    position: f64,
    duration: f64,
    signal_path: QString,
    bitrate: i32,
    stream_info: QString,
    waveform: QList<QVariant>,
    waveform_rms: QList<QVariant>,
    waveform_id: String, // the song the waveform lists are (or will be) for
    seek_hold: Option<SeekHold>,
    options: QMap<QMapPair_QString_QVariant>,
    theme: QString,
}

/// A seek the core may not have taken yet: until then its snapshots still carry the old place, and showing it would
/// throw the seek bar back for a frame.
struct SeekHold {
    to: f64,
    at: std::time::Instant,
    song: String,
}

/// The position to show: the seek target while the core reports somewhere else (at most 3 s, same song), then the
/// core's.
fn held(hold: &mut Option<SeekHold>, reported: f64, song: &str, now: std::time::Instant) -> f64 {
    if let Some(h) = hold {
        if h.song == song && (reported - h.to).abs() >= 1.0 && now.duration_since(h.at).as_secs_f64() < 3.0 {
            return h.to;
        }
        *hold = None;
    }
    reported
}

fn json_value(text: &QString) -> Value {
    serde_json::from_str(&text.to_string()).unwrap_or(Value::Null)
}

fn json_list(text: &QString) -> Vec<Value> {
    match json_value(text) {
        Value::Array(a) => a,
        _ => vec![],
    }
}

impl cxx_qt::Initialize for ffi::RustPlayer {
    fn initialize(mut self: Pin<&mut Self>) {
        *PLAYER_QT.lock().unwrap() = Some(self.qt_thread());
        // shown before the first event: main() reads the theme right away
        if let Some(s) = player().snapshot() {
            {
                let mut r = self.as_mut().rust_mut();
                r.volume = s.volume;
                r.theme = QString::from(s.theme.as_str());
            }
            self.as_mut().apply(Changes::all(), s);
        }
    }
}

impl ffi::RustPlayer {
    /// Takes a snapshot; signals per group. Volume and theme change only through their
    /// setters (shown at once), so the snapshot's copy of them is not taken again.
    pub(crate) fn apply(mut self: Pin<&mut Self>, ch: Changes, s: PlayerSnapshot) {
        {
            let mut r = self.as_mut().rust_mut();
            r.online = s.online;
            r.offline = s.offline;
            r.signed_in = s.signed_in;
            r.login_failed = s.login_failed;
            r.status = QString::from(s.status.as_str());
            r.user = QString::from(s.user.as_str());
            r.connecting = s.connecting;
            if ch.queue {
                r.queue = s.queue.iter().map(to_qvariant).collect();
                r.queue_length = s.queue.len() as i32;
            }
            r.index = s.index as i32;
            r.current = to_qvariant_map(&s.current);
            r.playing = s.playing;
            r.crossfade = s.crossfade as i32;
            r.exclusive = s.exclusive;
            r.dj = s.dj;
            r.station = to_qvariant_map(&Value::Object(s.station));
            r.shuffle = s.shuffle;
            r.repeat = s.repeat as i32;
            let song = s.current["id"].as_str().unwrap_or_default();
            r.position = held(&mut r.seek_hold, s.position, song, std::time::Instant::now());
            r.duration = s.duration;
            r.signal_path = QString::from(s.signal_path.as_str());
            r.bitrate = s.bitrate as i32;
            r.stream_info = QString::from(s.stream_info.as_str());
            if ch.options {
                r.cache_bytes = s.cache_bytes as f64;
                r.audio_devices = s.audio_devices.iter().map(|(n, d)| to_qvariant(&json!({"name": n, "description": d}))).collect();
                r.options = to_qvariant_map(&Value::Object(s.options));
            }
        }
        self.as_mut().follow_waveform(s.current["id"].as_str().unwrap_or_default());
        if ch.status {
            self.as_mut().status_changed();
        }
        if ch.state {
            self.as_mut().state_changed();
        }
        if ch.queue {
            self.as_mut().queue_changed();
        }
        if ch.position {
            self.as_mut().position_changed();
        }
        if ch.stream_info {
            self.as_mut().stream_info_changed();
        }
        if ch.options {
            self.as_mut().options_changed();
        }
    }

    /// A new song empties the waveform at once and asks the relay for its own; an answer for a song no longer
    /// playing is dropped.
    fn follow_waveform(mut self: Pin<&mut Self>, id: &str) {
        if self.rust().waveform_id == id {
            return;
        }
        {
            let mut r = self.as_mut().rust_mut();
            r.waveform_id = id.to_string();
            r.waveform = QList::default();
            r.waveform_rms = QList::default();
        }
        self.as_mut().waveform_changed();
        if id.is_empty() {
            return;
        }
        let (qt, want) = (self.qt_thread(), id.to_string());
        player().waveform(id.to_string(), move |w| {
            let Some(w) = w else { return };
            let _ = qt.queue(move |mut p| {
                if p.rust().waveform_id != want {
                    return;
                }
                let list = |v: &[f32]| v.iter().map(|&x| QVariant::from(&f64::from(x))).collect();
                {
                    let mut r = p.as_mut().rust_mut();
                    r.waveform = list(&w.peaks);
                    r.waveform_rms = list(&w.rms);
                }
                p.waveform_changed();
            });
        });
    }

    fn set_crossfade(self: Pin<&mut Self>, seconds: i32) {
        player().call(move |c| c.set_crossfade(seconds as i64));
    }
    fn set_exclusive(self: Pin<&mut Self>, on: bool) {
        player().call(move |c| c.set_exclusive(on));
    }
    fn set_dj(self: Pin<&mut Self>, on: bool) {
        player().call(move |c| c.set_dj(on));
    }
    fn set_shuffle(self: Pin<&mut Self>, on: bool) {
        player().call(move |c| c.set_shuffle(on));
    }
    fn set_repeat(self: Pin<&mut Self>, mode: i32) {
        player().call(move |c| c.set_repeat(mode as i64));
    }
    pub(crate) fn set_volume(mut self: Pin<&mut Self>, v: f64) {
        let v = v.clamp(0.0, 100.0);
        if v != self.rust().volume {
            self.as_mut().rust_mut().volume = v;
            self.as_mut().volume_changed();
        }
        player().call(move |c| c.set_volume(v));
    }
    fn set_theme(mut self: Pin<&mut Self>, name: QString) {
        if name == self.rust().theme {
            return;
        }
        let n = name.to_string();
        self.as_mut().rust_mut().theme = name;
        self.as_mut().theme_changed();
        player().call(move |c| c.set_theme(&n));
    }

    fn setting(&self, key: &QString) -> QString {
        let k = key.to_string();
        if k != "url" && k != "user" {
            return QString::default();
        }
        QString::from(player().settings().get_string(&k).as_str())
    }
    fn set_option_json(&self, key: &QString, json: &QString) {
        let (k, v) = (key.to_string(), json_value(json));
        player().call(move |c| c.set_option(&k, v));
    }
    fn refresh_audio_devices(&self) {
        player().call(|c| c.refresh_audio_devices());
    }
    fn clear_song_cache(&self) {
        player().call(|c| c.clear_song_cache());
    }
    fn reset_dj_memory(&self) {
        player().call(|c| c.reset_dj_memory());
    }
    fn login(&self, server: &QString, user: &QString, password: &QString) {
        let (s, u, p) = (server.to_string(), user.to_string(), password.to_string());
        player().call(move |c| c.login(&s, &u, &p));
    }
    fn logout(&self) {
        player().call(|c| c.logout());
    }
    fn request(&self, id: i32, method: &QString, params_json: &QString, write: bool) -> bool {
        let m = method.to_string();
        let allowed = if write { rela_core::queue::writable(&m) } else { rela_core::queue::readable(&m) };
        if !allowed {
            return false;
        }
        let params = match json_value(params_json) {
            Value::Object(o) => o,
            _ => Map::new(),
        };
        player().call(move |c| {
            c.request(id as u64, &m, params, write);
        });
        true
    }
    fn play_json(&self, songs_json: &QString, start: i32) {
        let songs = json_list(songs_json);
        player().call(move |c| c.play(songs, start as i64));
    }
    fn add_json(&self, songs_json: &QString, where_: i32) {
        let songs = json_list(songs_json);
        player().call(move |c| c.add(songs, where_));
    }
    fn shuffled_json(&self, songs_json: &QString) -> QString {
        QString::from(Value::Array(rela_core::queue::shuffled(json_list(songs_json))).to_string())
    }
    fn download_name_json(&self, item_json: &QString, kind: &QString) -> QString {
        QString::from(rela_core::queue::download_name(&json_value(item_json), &kind.to_string()))
    }
    fn play_station(&self, id: &QString) {
        let id = id.to_string();
        player().call(move |c| c.play_station(&id));
    }
    fn jump(&self, i: i32) {
        player().call(move |c| c.jump(i as i64));
    }
    fn remove(&self, i: i32) {
        player().call(move |c| c.remove(i as i64));
    }
    fn move_song(&self, from: i32, to: i32) {
        player().call(move |c| c.move_song(from as i64, to as i64));
    }
    fn clear_upcoming(&self) {
        player().call(|c| c.clear_upcoming());
    }
    fn toggle(&self) {
        player().call(|c| c.toggle());
    }
    fn next(&self) {
        player().call(|c| c.next());
    }
    fn prev(&self) {
        player().call(|c| c.prev());
    }
    /// Shown at once (the seek bar's binding takes it back when released), and held until the core has it.
    fn seek(mut self: Pin<&mut Self>, seconds: f64) {
        {
            let mut r = self.as_mut().rust_mut();
            let song = r.waveform_id.clone(); // the playing song's id (follow_waveform keeps it)
            r.seek_hold = Some(SeekHold { to: seconds, at: std::time::Instant::now(), song });
            r.position = seconds;
        }
        self.position_changed();
        player().call(move |c| c.seek(seconds));
    }
    fn download_to(&self, id: &QString, file_url: &QString) {
        let id = id.to_string();
        let text = file_url.to_string();
        let path = url::Url::parse(&text).ok().and_then(|u| u.to_file_path().ok()).unwrap_or_else(|| text.into());
        player().call(move |c| c.download(&id, path));
    }
    fn persist_now(&self) {
        crate::mpris::quit();
        player().persist_now();
    }
    fn tray_hint(&self) -> bool {
        let s = player().settings();
        if s.get_bool("trayHintShown", false) {
            return false;
        }
        s.set("trayHintShown", true.into());
        true
    }
    fn demo(&self) {
        player().call(|c| c.demo());
    }
    fn set_status(&self, text: &QString) {
        let t = text.to_string();
        player().call(move |c| c.set_status(t));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_seek_target_holds_until_the_core_catches_up() {
        let t0 = Instant::now();
        let mut hold = Some(SeekHold { to: 99.0, at: t0, song: "a".into() });
        // a snapshot from before the seek reached the core: still the old place
        assert_eq!(held(&mut hold, 130.0, "a", t0 + Duration::from_millis(16)), 99.0);
        assert!(hold.is_some());
        // the core has it (played a little since)
        assert_eq!(held(&mut hold, 99.4, "a", t0 + Duration::from_millis(200)), 99.4);
        assert!(hold.is_none());
        assert_eq!(held(&mut hold, 130.0, "a", t0 + Duration::from_millis(300)), 130.0, "no hold: as reported");
    }

    #[test]
    fn a_hold_ends_after_three_seconds_or_with_the_song() {
        let t0 = Instant::now();
        let mut hold = Some(SeekHold { to: 99.0, at: t0, song: "a".into() });
        assert_eq!(held(&mut hold, 130.0, "a", t0 + Duration::from_secs(3)), 130.0);
        assert!(hold.is_none());
        let mut hold = Some(SeekHold { to: 99.0, at: t0, song: "a".into() });
        assert_eq!(held(&mut hold, 2.0, "b", t0 + Duration::from_millis(16)), 2.0);
        assert!(hold.is_none());
    }
}
