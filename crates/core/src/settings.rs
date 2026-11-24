//! Settings: a key/value store with Qt's key shape (`key` or `section/key`). The desktop keeps Qt's INI file (the
//! same file QSettings used, so logins survive the switch); Android keeps a JSON file.
use serde_json::{Map, Value};
use std::collections::BTreeMap;
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

/// Writes through a temp file and a rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, bytes: &[u8], private: bool) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp-re-la");
    let _ = std::fs::remove_file(&tmp); // a stale one would keep its old mode
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600); // never readable by others, not even before the chmod below
    if open.open(&tmp).and_then(|mut f| std::io::Write::write_all(&mut f, bytes)).is_err() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let old = std::fs::metadata(path).ok().map(|m| m.permissions().mode() & 0o777);
        // the token is as good as the password: the file is ours alone once it holds one (and stays so)
        let mode = if private { 0o600 } else { old.unwrap_or(0o644) };
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
    }
    let _ = std::fs::rename(&tmp, path);
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
        write_atomic(&self.path, serde_json::to_string_pretty(data).unwrap_or_default().as_bytes(), true);
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

/// A value as the INI file holds it.
#[derive(Debug, Clone, PartialEq)]
enum Ini {
    Text(String),
    /// Something QSettings stored with a type (@Size(...), @Variant(...), a list): kept as written.
    Raw(String),
}

/// Qt's INI format (QSettings::IniFormat), the subset re-la uses: [General] for top-level keys, [section] for
/// `section/key`, strings, bools and numbers as text. Values it can't read as one string are kept verbatim.
/// shortcut: no line continuations (`\` at line end) and no nested groups beyond one level; Qt never wrote them here.
pub struct IniSettings {
    path: PathBuf,
    data: Mutex<BTreeMap<String, BTreeMap<String, Ini>>>, // section ("" = General) -> key -> value
}

fn split_key(key: &str) -> (String, String) {
    match key.split_once('/') {
        Some((section, k)) => (section.to_string(), k.to_string()),
        None => (String::new(), key.to_string()),
    }
}

/// Qt's iniEscapedKey: letters, digits and `_-.` as they are, the rest as %XX (%UXXXX above 0xFF).
fn escape_key(k: &str) -> String {
    let mut out = String::new();
    for c in k.encode_utf16() {
        match char::from_u32(u32::from(c)) {
            Some(ch) if ch.is_ascii_alphanumeric() || "_-.".contains(ch) => out.push(ch),
            _ if c <= 0xFF => out.push_str(&format!("%{c:02X}")),
            _ => out.push_str(&format!("%U{c:04X}")),
        }
    }
    out
}

fn unescape_key(k: &str) -> String {
    let b: Vec<char> = k.chars().collect();
    let mut units: Vec<u16> = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == '%' && i + 1 < b.len() {
            let (start, len) = if b[i + 1] == 'U' { (i + 2, 4) } else { (i + 1, 2) };
            if start + len <= b.len()
                && let Ok(v) = u16::from_str_radix(&b[start..start + len].iter().collect::<String>(), 16)
            {
                units.push(v);
                i = start + len;
                continue;
            }
        }
        let mut buf = [0u16; 2];
        units.extend_from_slice(b[i].encode_utf16(&mut buf));
        i += 1;
    }
    String::from_utf16_lossy(&units)
}

/// Qt's variantToString + iniEscapedString for one value.
fn escape_value(v: &Value) -> String {
    let s = match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if s.starts_with('@') => format!("@{s}"),
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    let mut out = String::new();
    let mut quotes = false;
    let mut escape_next_if_hex = false;
    for c in s.chars() {
        if matches!(c, ';' | ',' | '=') {
            quotes = true;
        }
        if escape_next_if_hex && c.is_ascii_hexdigit() {
            out.push_str(&format!("\\x{:x}", c as u32));
            continue;
        }
        escape_next_if_hex = false;
        match c {
            '\0' => {
                out.push_str("\\0");
                escape_next_if_hex = true;
            }
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) <= 0x1F => {
                out.push_str(&format!("\\x{:x}", c as u32));
                escape_next_if_hex = true;
            }
            c => out.push(c),
        }
    }
    if quotes || out.starts_with(' ') || out.ends_with(' ') {
        format!("\"{out}\"")
    } else {
        out
    }
}

/// Qt's iniUnescapedStringList for one value: quotes, escapes, `;` comments. A value with a bare comma is a list and
/// one starting with a single `@` is a typed value: both are kept raw.
fn unescape_value(raw: &str) -> Ini {
    let t = raw.trim();
    let mut out = String::new();
    let mut quoted = false;
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => return Ini::Raw(raw.trim().to_string()),
            ';' if !quoted => {
                out = out.trim_end().to_string(); // `value ; comment`
                break;
            }
            '\\' => match chars.next() {
                Some('a') => out.push('\u{7}'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('v') => out.push('\u{b}'),
                Some('0') => out.push('\0'),
                Some('x') => {
                    let mut hex = String::new();
                    while let Some(h) = chars.peek().filter(|h| h.is_ascii_hexdigit()) {
                        hex.push(*h);
                        chars.next();
                    }
                    out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                }
                Some(o) => out.push(o), // \" \\ \' \? and anything unknown
                None => {}
            },
            c => out.push(c),
        }
    }
    if let Some(rest) = out.strip_prefix("@@") {
        Ini::Text(format!("@{rest}"))
    } else if out.starts_with('@') {
        Ini::Raw(raw.trim().to_string())
    } else {
        Ini::Text(out)
    }
}

impl IniSettings {
    pub fn open(path: PathBuf) -> IniSettings {
        let mut data: BTreeMap<String, BTreeMap<String, Ini>> = BTreeMap::new();
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut section = String::new();
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with(';') || l.starts_with('#') {
                continue;
            }
            if let Some(name) = l.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                section = match name {
                    "General" => String::new(),
                    "%General" => "General".into(),
                    n => unescape_key(n),
                };
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                data.entry(section.clone()).or_default().insert(unescape_key(k.trim()).replace('\\', "/"), unescape_value(v));
            }
        }
        IniSettings { path, data: Mutex::new(data) }
    }

    fn save(&self, data: &BTreeMap<String, BTreeMap<String, Ini>>) {
        let mut out = String::new();
        for (section, keys) in data.iter().filter(|(_, k)| !k.is_empty()) {
            if !out.is_empty() {
                out.push('\n');
            }
            let name = match section.as_str() {
                "" => "General".to_string(),
                "General" => "%General".to_string(),
                s => escape_key(s),
            };
            out.push_str(&format!("[{name}]\n"));
            for (k, v) in keys {
                let value = match v {
                    Ini::Text(t) => escape_value(&Value::String(t.clone())),
                    Ini::Raw(r) => r.clone(),
                };
                let key = k.split('/').map(escape_key).collect::<Vec<_>>().join("\\");
                out.push_str(&format!("{key}={value}\n"));
            }
        }
        let private = data.get("").is_some_and(|g| g.contains_key("token"));
        write_atomic(&self.path, out.as_bytes(), private);
    }
}

impl Settings for IniSettings {
    fn get(&self, key: &str) -> Option<Value> {
        let (section, k) = split_key(key);
        match self.data.lock().unwrap().get(&section)?.get(&k)? {
            Ini::Text(t) => Some(Value::String(t.clone())),
            Ini::Raw(r) => Some(Value::String(r.clone())),
        }
    }
    fn set(&self, key: &str, value: Value) {
        let (section, k) = split_key(key);
        // stored as Qt would read it back: text; a string that begins with @ keeps it through escape_value's @@
        let text = match &value {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        let mut d = self.data.lock().unwrap();
        d.entry(section).or_default().insert(k, Ini::Text(text));
        self.save(&d);
    }
    fn remove(&self, key: &str) {
        let (section, k) = split_key(key);
        let mut d = self.data.lock().unwrap();
        if d.get_mut(&section).and_then(|s| s.remove(&k)).is_some() {
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

    fn stores(name: &str) -> Vec<Box<dyn Settings>> {
        let d = tmp(name);
        vec![Box::new(JsonSettings::open(d.join("s.json"))), Box::new(IniSettings::open(d.join("s.conf")))]
    }

    // radio off meant "add nothing at queue end": that is autoContinue off now; DJ stays at its default
    #[test]
    fn migrate_radio_off() {
        for s in stores("off") {
            s.set("radio", json!(false));
            migrate_settings(s.as_ref());
            assert!(!s.get_bool("options/autoContinue", true) && s.contains("options/autoContinue"));
            assert!(!s.contains("dj") && !s.contains("radio"));
        }
    }

    // radio on meant client-side picks: that is DJ on now; already-set new keys are kept
    #[test]
    fn migrate_radio_on() {
        for s in stores("on") {
            s.set("radio", json!(true));
            migrate_settings(s.as_ref());
            assert!(s.get_bool("dj", false) && !s.contains("options/autoContinue") && !s.contains("radio"));
            s.set("radio", json!(false));
            s.set("options/autoContinue", json!(true));
            migrate_settings(s.as_ref());
            assert!(s.get_bool("options/autoContinue", false) && s.get_bool("dj", false) && !s.contains("radio"));
        }
    }

    #[test]
    fn values_survive_a_reopen() {
        let d = tmp("reopen");
        for (path, ini) in [(d.join("a.json"), false), (d.join("a.conf"), true)] {
            let open = |p: &PathBuf| -> Box<dyn Settings> {
                if ini { Box::new(IniSettings::open(p.clone())) } else { Box::new(JsonSettings::open(p.clone())) }
            };
            let s = open(&path);
            s.set("volume", json!(0.75));
            s.set("crossfade", json!(5));
            s.set("shuffle", json!(true));
            s.set("options/relayUrl", json!("ws://h:4534/"));
            s.set("user", json!(" ali, veli = x; \"q\" "));
            s.remove("nothing");
            drop(s);
            let s = open(&path);
            assert_eq!(s.get_f64("volume", 0.0), 0.75);
            assert_eq!(s.get_i64("crossfade", 0), 5);
            assert!(s.get_bool("shuffle", false));
            assert_eq!(s.get_string("options/relayUrl"), "ws://h:4534/");
            assert_eq!(s.get_string("user"), " ali, veli = x; \"q\" ");
            s.remove("user");
            assert!(!s.contains("user") && s.get_string("user").is_empty());
        }
    }

    #[test]
    fn ini_sections_and_general() {
        let d = tmp("sections");
        let s = IniSettings::open(d.join("s.conf"));
        s.set("url", json!("https://music.example.com"));
        s.set("options/autoContinue", json!(false));
        s.set("sync/hidden", json!(true));
        let text = std::fs::read_to_string(d.join("s.conf")).unwrap();
        assert!(text.starts_with("[General]\nurl=https://music.example.com\n"), "{text}");
        assert!(text.contains("\n[options]\nautoContinue=false\n") && text.contains("\n[sync]\nhidden=true\n"), "{text}");
    }

    // values QSettings stored with a type, and lists, survive Rust rewriting the file
    #[test]
    fn typed_values_are_kept() {
        let d = tmp("typed");
        let f = d.join("s.conf");
        std::fs::write(&f, "[General]\nlist=a, b\nwindow=@Size(800 600)\n\n[%General]\nx=1\n").unwrap();
        let s = IniSettings::open(f.clone());
        s.set("theme", json!("nothing"));
        let text = std::fs::read_to_string(&f).unwrap();
        assert!(text.contains("\nlist=a, b\n") && text.contains("\nwindow=@Size(800 600)\n"), "{text}");
        assert!(text.contains("[%General]\nx=1\n"), "a real group named General keeps its escape: {text}");
        assert_eq!(s.get_string("General/x"), "1");
    }

    #[cfg(unix)]
    #[test]
    fn token_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let d = tmp("private");
        let s = IniSettings::open(d.join("s.conf"));
        s.set("token", json!("abc"));
        let mode = std::fs::metadata(d.join("s.conf")).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
