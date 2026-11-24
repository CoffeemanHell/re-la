//! The one Player of the process (created on first use) and the routes from its events to the QObjects QML sees:
//! each QObject registers its Qt thread here when it is created; an event for one that is gone is dropped.
use core::pin::Pin;
use cxx_qt::{CxxQtThread, Threading};
use cxx_qt_lib::QString;
use rela_core::player::{Changes, Player, PlayerListener, PlayerSnapshot};
use rela_core::settings::{IniSettings, Settings, desktop_settings_path, move_once};
use rela_core::subwave::{SubwaveEvent, SubwaveSnapshot};
use rela_core::sync::{SyncEvent, SyncSnapshot};
use serde_json::Value;
use crate::mpris::Event;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

static PLAYER: OnceLock<Player> = OnceLock::new();
pub static PLAYER_QT: Mutex<Option<CxxQtThread<crate::player::ffi::RustPlayer>>> = Mutex::new(None);
pub(crate) static SYNC_QT: Mutex<Option<CxxQtThread<crate::sync::ffi::RustSync>>> = Mutex::new(None);
pub(crate) static SUBWAVE_QT: Mutex<Option<CxxQtThread<crate::subwave::ffi::RustSubwave>>> = Mutex::new(None);

/// Where Qt's CacheLocation is on Windows (org "re-la", app "re-la"): queue.json and the covers.
#[cfg(windows)]
fn cache_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("re-la").join("re-la").join("cache")
}

/// The app was Nsync: its settings, cache and themes move to the new name's folders on the first start.
#[cfg(windows)]
pub fn move_from_nsync() {
    let roaming = PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default());
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default());
    move_once(&roaming.join("nsync"), &roaming.join("re-la"), Some(("nsync.ini", "re-la.ini")));
    move_once(&roaming.join("re-la").join("nsync"), &roaming.join("re-la").join("re-la"), None); // the themes
    move_once(&local.join("nsync").join("nsync"), &local.join("re-la").join("re-la"), None);
}

/// The C++ app kept its settings in the registry (QSettings' native format on Windows): brought over once, before
/// the INI file exists. The registry is only read.
#[cfg(windows)]
fn import_registry(s: &dyn Settings) {
    use rela_core::settings::{RegData, qt_registry_value};
    use windows_registry::{CURRENT_USER, Key, Type};
    fn walk(key: &Key, prefix: &str, s: &dyn Settings) {
        for (name, v) in key.values().into_iter().flatten() {
            let d = match v.ty() {
                Type::String | Type::ExpandString => String::try_from(v).map_or(RegData::Other, RegData::Sz),
                Type::U32 => u32::try_from(v).map_or(RegData::Other, RegData::Dword),
                Type::U64 => u64::try_from(v).map_or(RegData::Other, RegData::Qword),
                _ => RegData::Other,
            };
            if let Some(value) = qt_registry_value(d) {
                s.set(&format!("{prefix}{name}"), value);
            }
        }
        for sub in key.keys().into_iter().flatten() {
            if let Ok(k) = key.open(&sub) {
                walk(&k, &format!("{prefix}{sub}/"), s);
            }
        }
    }
    if let Ok(root) = CURRENT_USER.open("Software\\nsync\\nsync") {
        walk(&root, "", s);
    }
}

#[cfg(not(windows))]
fn xdg(var: &str, home_default: &str) -> PathBuf {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(home_default))
}

/// Where Qt's CacheLocation is on Linux (org "re-la", app "re-la"): queue.json and the covers.
#[cfg(not(windows))]
fn cache_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache").join("re-la").join("re-la")
}

/// The app was Nsync: its settings, cache and themes move to the new name's folders on the first start.
#[cfg(not(windows))]
pub fn move_from_nsync() {
    let config = rela_core::settings::config_dir();
    move_once(&config.join("nsync"), &config.join("re-la"), Some(("nsync.conf", "re-la.conf")));
    let cache = xdg("XDG_CACHE_HOME", ".cache");
    move_once(&cache.join("nsync").join("nsync"), &cache.join("re-la").join("re-la"), None);
    let data = xdg("XDG_DATA_HOME", ".local/share");
    move_once(&data.join("nsync").join("nsync"), &data.join("re-la").join("re-la"), None); // the themes
}

/// The player, with the settings file QSettings used (logins survive the switch) and two mpv decks. libmpv needs
/// LC_NUMERIC "C" before this runs (main.rs sets it first).
pub fn player() -> &'static Player {
    PLAYER.get_or_init(|| {
        let path = desktop_settings_path();
        #[cfg(windows)]
        let fresh = !path.exists();
        let settings: Arc<dyn Settings> = Arc::new(IniSettings::open(path));
        #[cfg(windows)]
        if fresh {
            import_registry(settings.as_ref());
        }
        // RELA_AO: mpv's audio outputs to try instead (e.g. "null" for a run without a sound device)
        let ao = std::env::var("RELA_AO").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "pipewire,pulse,alsa,".into());
        let p = Player::new(settings, cache_dir(), move |i, sink| rela_mpv::mpv_deck(i, sink, &ao), Arc::new(Front));
        crate::mpris::start(cache_dir());
        p
    })
}

pub fn to<T: Threading>(slot: &Mutex<Option<CxxQtThread<T>>>, f: impl FnOnce(Pin<&mut T>) + Send + 'static) {
    if let Some(t) = slot.lock().unwrap().as_ref() {
        let _ = t.queue(f);
    }
}

struct Front;

impl PlayerListener for Front {
    fn on_player(&self, changes: Changes, snapshot: PlayerSnapshot) {
        crate::mpris::send(Event::Player { position: changes.position, snapshot: Box::new(snapshot.clone()) });
        to(&PLAYER_QT, move |o| o.apply(changes, snapshot));
    }
    fn on_reply(&self, id: u64, reply: Option<Value>, call: bool) {
        let json = reply.unwrap_or(Value::Null).to_string();
        to(&PLAYER_QT, move |o| o.replied(id as i32, QString::from(json.as_str()), call));
    }
    fn on_sync(&self, event: SyncEvent, snapshot: SyncSnapshot) {
        crate::mpris::send(Event::CanControl(snapshot.can_control));
        to(&SYNC_QT, move |o| crate::sync::deliver(o, event, snapshot));
    }
    fn on_subwave(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot) {
        crate::mpris::send(Event::Subwave(Box::new(snapshot.clone())));
        to(&SUBWAVE_QT, move |o| crate::subwave::deliver(o, event, snapshot));
    }
}
