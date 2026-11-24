//! Settings: a key/value store with Qt's key shape (`key` or `section/key`). The desktop keeps them in QSettings
//! (crates/rela_desktop); Android keeps a JSON file.
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub trait Settings: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&self, key: &str, value: Value);
    fn remove(&self, key: &str);

    fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    /// QVariant::toString(): numbers and bools as text, missing as "".
    fn get_string(&self, key: &str) -> String {
        match self.get(key) {
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => String::new(),
        }
    }
    /// QVariant::toBool(): a string is true unless it is "", "0" or "false".
    fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some(Value::Bool(b)) => b,
            Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
            Some(Value::String(s)) => !(s.is_empty() || s == "0" || s.eq_ignore_ascii_case("false")),
            _ => default,
        }
    }
    fn get_i64(&self, key: &str, default: i64) -> i64 {
        match self.get(key) {
            Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(default),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(default),
            Some(Value::Bool(b)) => b as i64,
            _ => default,
        }
    }
    fn get_f64(&self, key: &str, default: f64) -> f64 {
        match self.get(key) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(default),
            _ => default,
        }
    }
}

/// One-time renames of saved settings: radio on -> dj on, radio off -> options/autoContinue off.
pub fn migrate_settings(s: &dyn Settings) {
    if !s.contains("radio") {
        return;
    }
    if s.get_bool("radio", false) {
        if !s.contains("dj") {
            s.set("dj", Value::Bool(true));
        }
    } else if !s.contains("options/autoContinue") {
        s.set("options/autoContinue", Value::Bool(false));
    }
    s.remove("radio");
}

/// The user's config dir: XDG_CONFIG_HOME, else ~/.config.
#[cfg(not(windows))]
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"))
}

/// Where the desktop app keeps its settings: QSettings' file on Linux (org "re-la", app "re-la").
#[cfg(not(windows))]
pub fn desktop_settings_path() -> PathBuf {
    config_dir().join("re-la").join("re-la.conf")
}

/// On Windows: where QSettings' INI format would put it (the C++ app used the registry, see [`qt_registry_value`]).
#[cfg(windows)]
pub fn desktop_settings_path() -> PathBuf {
    PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("re-la").join("re-la.ini")
}

/// The app was called Nsync: its folder `old` becomes `new` (a rename, nothing copied), once — not when `new` is
/// already there. `file`: (old name, new name) of a file inside to rename too. An old parent left empty goes. True
/// when it moved.
pub fn move_once(old: &Path, new: &Path, file: Option<(&str, &str)>) -> bool {
    if new.exists() || !old.is_dir() {
        return false;
    }
    if let Some(p) = new.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if std::fs::rename(old, new).is_err() {
        return false;
    }
    if let Some((a, b)) = file {
        let _ = std::fs::rename(new.join(a), new.join(b));
    }
    if let Some(p) = old.parent() {
        let _ = std::fs::remove_dir(p); // only when empty
    }
    true
}

/// A registry value as the C++ app's QSettings wrote it on Windows (HKCU\Software\nsync\nsync).
#[derive(Debug, Clone)]
pub enum RegData {
    Sz(String),
    Dword(u32),
    Qword(u64),
    Other,
}

/// What a registry value was to QSettings, for bringing the old Windows settings over once. Qt's own types
/// (`@Variant(...)`, `@ByteArray(...)`: the window size and such), lists (the C++ app wrote none) and binary values are
/// left behind.
pub fn qt_registry_value(d: RegData) -> Option<Value> {
    match d {
        RegData::Sz(s) if s == "true" || s == "false" => Some(Value::Bool(s == "true")),
        RegData::Sz(s) if s.starts_with("@@") => Some(Value::String(s[1..].to_string())), // Qt's escape for a leading @
        RegData::Sz(s) if s.starts_with('@') => None,
        RegData::Sz(s) => Some(Value::String(s)),
        RegData::Dword(n) => Some(n.into()),
        RegData::Qword(n) => Some(n.into()),
        RegData::Other => None,
    }
}

/// Writes through a temp file and a rename, so a crash never leaves half a file. The token is as good as the password:
/// the file is ours alone.
fn write_private(path: &Path, bytes: &[u8]) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp-re-la");
    let _ = std::fs::remove_file(&tmp); // a stale one would keep its old mode
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    if open.open(&tmp).and_then(|mut f| std::io::Write::write_all(&mut f, bytes)).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// A JSON object in a file, flat keys: Android's store.
pub struct JsonSettings {
    path: PathBuf,
    data: Mutex<Map<String, Value>>,
}

impl JsonSettings {
    pub fn open(path: PathBuf) -> JsonSettings {
        let data = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        JsonSettings { path, data: Mutex::new(data) }
    }
    fn save(&self, data: &Map<String, Value>) {
        write_private(&self.path, serde_json::to_string_pretty(data).unwrap_or_default().as_bytes());
    }
}

impl Settings for JsonSettings {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: Value) {
        let mut d = self.data.lock().unwrap();
        d.insert(key.into(), value);
        self.save(&d);
    }
    fn remove(&self, key: &str) {
        let mut d = self.data.lock().unwrap();
        if d.remove(key).is_some() {
            self.save(&d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn qt_registry_value_maps_types() {
        use RegData::*;
        assert_eq!(qt_registry_value(Sz("true".into())), Some(json!(true)));
        assert_eq!(qt_registry_value(Sz("false".into())), Some(json!(false)));
        assert_eq!(qt_registry_value(Sz("https://music.example.com".into())), Some(json!("https://music.example.com")));
        assert_eq!(qt_registry_value(Dword(80)), Some(json!(80)));
        assert_eq!(qt_registry_value(Qword(5)), Some(json!(5)));
        assert_eq!(qt_registry_value(Sz("@Variant(\0\0)".into())), None, "a Qt type only QSettings reads");
        assert_eq!(qt_registry_value(Sz("@ByteArray(x)".into())), None);
        assert_eq!(qt_registry_value(Sz("@@home".into())), Some(json!("@home")), "Qt's escaped @");
        assert_eq!(qt_registry_value(Other), None);
    }

    #[test]
    fn the_old_names_folder_moves_once() {
        let d = tmp("move");
        let (old, new) = (d.join("nsync/nsync"), d.join("re-la/re-la"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("nsync.conf"), "[General]\nx=1\n").unwrap();
        assert!(move_once(&old, &new, Some(("nsync.conf", "re-la.conf"))));
        assert_eq!(std::fs::read_to_string(new.join("re-la.conf")).unwrap(), "[General]\nx=1\n");
        assert!(!old.exists());
        assert!(!d.join("nsync").exists(), "the emptied old parent goes too");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("nsync.conf"), "older").unwrap();
        assert!(!move_once(&old, &new, Some(("nsync.conf", "re-la.conf"))), "the new one is there: nothing moves");
        assert_eq!(std::fs::read_to_string(new.join("re-la.conf")).unwrap(), "[General]\nx=1\n");
        assert!(!move_once(&d.join("none"), &d.join("new2"), None), "no old folder");
        assert!(!d.join("new2").exists());
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("re-la-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn store(name: &str) -> JsonSettings {
        JsonSettings::open(tmp(name).join("s.json"))
    }

    // radio off meant "add nothing at queue end": that is autoContinue off now; DJ stays at its default
    #[test]
    fn migrate_radio_off() {
        let s = store("off");
        s.set("radio", json!(false));
        migrate_settings(&s);
        assert!(!s.get_bool("options/autoContinue", true) && s.contains("options/autoContinue"));
        assert!(!s.contains("dj") && !s.contains("radio"));
    }

    // radio on meant client-side picks: that is DJ on now; already-set new keys are kept
    #[test]
    fn migrate_radio_on() {
        let s = store("on");
        s.set("radio", json!(true));
        migrate_settings(&s);
        assert!(s.get_bool("dj", false) && !s.contains("options/autoContinue") && !s.contains("radio"));
        s.set("radio", json!(false));
        s.set("options/autoContinue", json!(true));
        migrate_settings(&s);
        assert!(s.get_bool("options/autoContinue", false) && s.get_bool("dj", false) && !s.contains("radio"));
    }

    #[test]
    fn values_survive_a_reopen() {
        let path = tmp("reopen").join("a.json");
        let s = JsonSettings::open(path.clone());
        s.set("volume", json!(0.75));
        s.set("crossfade", json!(5));
        s.set("shuffle", json!(true));
        s.set("options/relayUrl", json!("ws://h:4534/"));
        s.set("user", json!(" ali, veli = x; \"q\" "));
        s.remove("nothing");
        drop(s);
        let s = JsonSettings::open(path);
        assert_eq!(s.get_f64("volume", 0.0), 0.75);
        assert_eq!(s.get_i64("crossfade", 0), 5);
        assert!(s.get_bool("shuffle", false));
        assert_eq!(s.get_string("options/relayUrl"), "ws://h:4534/");
        assert_eq!(s.get_string("user"), " ali, veli = x; \"q\" ");
        s.remove("user");
        assert!(!s.contains("user") && s.get_string("user").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn token_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let f = tmp("private").join("s.json");
        JsonSettings::open(f.clone()).set("token", json!("abc"));
        assert_eq!(std::fs::metadata(f).unwrap().permissions().mode() & 0o777, 0o600);
    }
}
