//! The desktop's settings: QSettings on an INI file. On Linux QML's `Settings` (the window size) uses the same file; Qt
//! keeps one copy of it for the whole process, so neither overwrites the other.
use cxx::UniquePtr;
use cxx_qt_lib::{QAnyStringView, QString, QVariant};
use rela_core::settings::Settings;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Format(i32);
// SAFETY: a repr(transparent) i32, as the C++ enum is
unsafe impl cxx::ExternType for Format {
    type Id = cxx::type_id!("QSettings::Format");
    type Kind = cxx::kind::Trivial;
}
const INI: Format = Format(1); // QSettings::IniFormat

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!(<QtCore/QSettings>);
        include!("cxx-qt-lib/common.h");
        include!("cxx-qt-lib/qanystringview.h");
        type QAnyStringView<'a> = cxx_qt_lib::QAnyStringView<'a>;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        #[namespace = "QSettings"]
        type Format = super::Format;
        type QSettings;
        #[namespace = "rust::cxxqtlib1"]
        #[cxx_name = "make_unique"]
        fn open(file: &QString, format: Format) -> UniquePtr<QSettings>;
        fn value(self: &QSettings, key: QAnyStringView) -> QVariant;
        #[cxx_name = "setValue"]
        fn set_value(self: Pin<&mut QSettings>, key: QAnyStringView, value: &QVariant);
        fn remove(self: Pin<&mut QSettings>, key: QAnyStringView);
    }
}

pub struct QtSettings {
    path: PathBuf,
}

impl QtSettings {
    pub fn open(path: PathBuf) -> QtSettings {
        QtSettings { path }
    }

    /// A QSettings per call: they share Qt's copy of the file, and each may live on any thread. It writes when dropped.
    fn qt(&self) -> UniquePtr<ffi::QSettings> {
        ffi::open(&QString::from(&*self.path.to_string_lossy()), INI)
    }
}

/// The token is as good as the password: the file is ours alone before anything is written (Qt keeps its mode).
fn private(_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(dir) = _path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::OpenOptions::new().append(true).create(true).mode(0o600).open(_path);
        let _ = std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o600));
    }
}

impl Settings for QtSettings {
    fn get(&self, key: &str) -> Option<Value> {
        let v = self.qt().value(QAnyStringView::from(key));
        v.value::<QString>().map(|s| Value::String(s.to_string()))
    }
    fn set(&self, key: &str, value: Value) {
        // as text, as the file holds it: a value reads back the same before and after a restart
        let text = match value {
            Value::String(s) => s,
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        private(&self.path);
        self.qt().pin_mut().set_value(QAnyStringView::from(key), &QVariant::from(&QString::from(&*text)));
    }
    fn remove(&self, key: &str) {
        private(&self.path);
        self.qt().pin_mut().remove(QAnyStringView::from(key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("re-la-qsettings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("re-la.conf")
    }

    // a file from before QSettings (re-la's own INI code wrote it): the login and the rest are still there, also after
    // a write
    #[test]
    fn an_older_file_reads_back() {
        let f = file("older");
        std::fs::write(&f, include_str!("../tests/fixtures/settings.conf")).unwrap();
        let s = QtSettings::open(f);
        for _ in 0..2 {
            for (key, text) in [
                ("crossfade", "5"),
                ("odd key ü/é", "x\u{1}f"),
                ("options/autoContinue", "false"),
                ("options/columns", "artist,album,duration"),
                ("options/relayUrl", "ws://h:4534/"),
                ("path", "C:\\back\\slash"),
                ("salt", "f00d"),
                ("shuffle", "true"),
                ("sync/hidden", "true"),
                ("tabs", "a\tb\nc"),
                ("theme", "Şık tema ü"),
                ("token", "abc123"),
                ("url", "https://music.example.com"),
                ("user", " ali, veli = x; \"q\" "),
                ("volume", "0.75"),
                ("weird", "@leading at"),
            ] {
                assert_eq!(s.get_string(key), text, "{key}");
            }
            assert!(s.get_bool("shuffle", false) && !s.get_bool("options/autoContinue", true));
            assert_eq!((s.get_i64("crossfade", 0), s.get_f64("volume", 0.0)), (5, 0.75));
            assert!(!s.contains("nothing") && s.get_string("nothing").is_empty());
            s.set("dj", json!(true));
        }
    }

    #[test]
    fn values_are_in_the_file_at_once_and_go_when_removed() {
        let f = file("values");
        let s = QtSettings::open(f.clone());
        s.set("url", json!("https://music.example.com"));
        s.set("volume", json!(0.75));
        s.set("crossfade", json!(5));
        s.set("options/autoContinue", json!(false));
        s.set("user", json!(" ali, veli = x; \"q\" "));
        let text = std::fs::read_to_string(&f).unwrap().replace("\r\n", "\n");
        assert!(text.starts_with("[General]\n") && text.contains("\nurl=https://music.example.com\n"), "{text}");
        assert!(text.contains("\n[options]\nautoContinue=false\n"), "{text}");
        assert_eq!((s.get_i64("crossfade", 0), s.get_f64("volume", 0.0)), (5, 0.75));
        assert_eq!(s.get_string("user"), " ali, veli = x; \"q\" ");
        s.remove("user");
        assert!(!s.contains("user") && !std::fs::read_to_string(&f).unwrap().contains("ali"));
    }

    // QML's Settings writes the window size to the same file while the app runs
    #[test]
    fn what_another_qsettings_wrote_stays() {
        let f = file("window");
        let s = QtSettings::open(f.clone());
        s.set("volume", json!(50));
        ffi::open(&QString::from(&*f.to_string_lossy()), INI).pin_mut().set_value(QAnyStringView::from("window/w"), &QVariant::from(&1500));
        s.set("volume", json!(60));
        let text = std::fs::read_to_string(&f).unwrap();
        assert!(text.contains("w=1500") && text.contains("volume=60"), "{text}");
    }

    #[test]
    fn threads_share_it() {
        let f = file("threads");
        let s = std::sync::Arc::new(QtSettings::open(f));
        let all: Vec<_> = (0..8)
            .map(|t| {
                let s = s.clone();
                std::thread::spawn(move || (0..20).for_each(|i| s.set(&format!("t{t}/k{i}"), json!(i))))
            })
            .collect();
        all.into_iter().for_each(|t| t.join().unwrap());
        for t in 0..8 {
            for i in 0..20 {
                assert_eq!(s.get_i64(&format!("t{t}/k{i}"), -1), i, "t{t}/k{i}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let mode = |f: &Path| std::fs::metadata(f).unwrap().permissions().mode() & 0o777;
        let f = file("private");
        QtSettings::open(f.clone()).set("token", json!("abc"));
        assert_eq!(mode(&f), 0o600);
        // a file Qt made before the login (the window size): others could read it
        std::fs::write(&f, "[window]\nw=1\n").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        QtSettings::open(f.clone()).set("token", json!("abc"));
        assert_eq!(mode(&f), 0o600);
    }
}
