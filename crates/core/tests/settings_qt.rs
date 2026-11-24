//! IniSettings against files real QSettings wrote and reads (fixtures/qt-settings.qml run by Qt's `qml` tool).
use rela_core::settings::{IniSettings, Settings};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

/// What the fixture script stores, and what Qt's String(value) gives back for each.
fn expected() -> Vec<(&'static str, Value, &'static str)> {
    vec![
        ("crossfade", json!(5), "5"),
        ("options/autoContinue", json!(false), "false"),
        ("options/relayUrl", json!("ws://h:4534/"), "ws://h:4534/"),
        ("path", json!("C:\\back\\slash"), "C:\\back\\slash"),
        ("shuffle", json!(true), "true"),
        ("sync/hidden", json!(true), "true"),
        ("tabs", json!("a\tb\nc"), "a\tb\nc"),
        ("theme", json!("Şık tema ü"), "Şık tema ü"),
        ("token", json!("abc123"), "abc123"),
        ("url", json!("https://music.example.com"), "https://music.example.com"),
        ("user", json!(" ali, veli = x; \"q\" "), " ali, veli = x; \"q\" "),
        ("volume", json!(0.75), "0.75"),
        ("weird", json!("@leading at"), "@leading at"),
    ]
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn qt_written_file_reads_back() {
    let s = IniSettings::open(fixtures().join("qt-written.conf"));
    for (key, _, text) in expected() {
        assert_eq!(s.get_string(key), text, "{key}");
    }
    assert_eq!(s.get_i64("crossfade", 0), 5);
    assert_eq!(s.get_f64("volume", 0.0), 0.75);
    assert!(s.get_bool("shuffle", false) && s.get_bool("sync/hidden", false) && !s.get_bool("options/autoContinue", true));
}

fn qml() -> Option<String> {
    ["/usr/lib64/qt6/bin/qml", "/usr/lib/qt6/bin/qml", "qml6"]
        .into_iter()
        .find(|q| Command::new(q).arg("--version").output().is_ok())
        .map(str::to_string)
}

#[test]
fn rust_written_file_reads_in_qt() {
    let Some(qml) = qml() else {
        eprintln!("skipped: no Qt qml tool");
        return;
    };
    let dir = std::env::temp_dir().join(format!("re-la-settings-qt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("rust.conf");
    let s = IniSettings::open(file.clone());
    for (key, value, _) in expected() {
        s.set(key, value);
    }
    let out = Command::new(qml)
        .arg(fixtures().join("qt-settings.qml"))
        .args(["--", "read"])
        .arg(&file)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    for (key, _, text) in expected() {
        let line = format!("KV {key}={}", serde_json::to_string(text).unwrap());
        assert!(log.contains(&line), "Qt read {key} differently; wanted {line}\n{log}");
    }
}
