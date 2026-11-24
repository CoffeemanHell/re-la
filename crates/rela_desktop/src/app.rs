//! `App` for themes (import Rela): the theme being drawn and a few facts about the app; and finding themes. A theme is
//! a folder with `theme.json` (tokens) and optionally `Main.qml` (the whole UI); without one it uses the Main.qml of
//! its `base` (default: "default"). Searched in the user data dir, then next to the exe, then the built-in ones
//! (`:/themes/<folder>`, a Qt resource).
use crate::hub::player;
use crate::qt::to_qvariant_map;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString, QStringList};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Builtin {
    pub name: &'static str,
    pub json: &'static str,
    pub main: bool, // has its own Main.qml
}

include!(concat!(env!("OUT_DIR"), "/builtin_themes.rs")); // BUILTIN, from build.rs

#[derive(Clone, Debug, PartialEq)]
pub enum Dir {
    Fs(PathBuf),
    Builtin(&'static str),
}

impl Dir {
    fn builtin<'a>(&self, builtin: &'a [Builtin]) -> Option<&'a Builtin> {
        let Dir::Builtin(n) = self else { return None };
        builtin.iter().find(|b| b.name == *n)
    }
    fn json(&self, builtin: &[Builtin]) -> Map<String, Value> {
        let text = match self {
            Dir::Fs(p) => std::fs::read_to_string(p.join("theme.json")).unwrap_or_default(),
            Dir::Builtin(_) => self.builtin(builtin).map(|b| b.json.to_string()).unwrap_or_default(),
        };
        match serde_json::from_str(&text) {
            Ok(Value::Object(o)) => o,
            _ => Map::new(),
        }
    }
    fn has_main(&self, builtin: &[Builtin]) -> bool {
        match self {
            Dir::Fs(p) => p.join("Main.qml").is_file(),
            Dir::Builtin(_) => self.builtin(builtin).is_some_and(|b| b.main),
        }
    }
    /// The folder's URL with a trailing slash, or a file in it.
    fn url(&self, file: &str) -> String {
        match self {
            Dir::Fs(p) => url::Url::from_directory_path(p).map(|u| format!("{u}{file}")).unwrap_or_default(),
            Dir::Builtin(n) => format!("qrc:/themes/{n}/{file}"),
        }
    }
    /// A path QFontDatabase can open.
    fn file(&self, file: &str) -> String {
        match self {
            Dir::Fs(p) => p.join(file).to_string_lossy().into_owned(),
            Dir::Builtin(n) => format!(":/themes/{n}/{file}"),
        }
    }
}

/// The folder of a theme: the first root that has `<name>/theme.json`; else the built-in default.
pub fn theme_dir(name: &str, roots: &[PathBuf], builtin: &[Builtin]) -> Dir {
    if !name.is_empty() && !name.contains(['/', '\\']) && name != ".." {
        for r in roots {
            if r.join(name).join("theme.json").is_file() {
                return Dir::Fs(r.join(name));
            }
        }
        if let Some(b) = builtin.iter().find(|b| b.name == name) {
            return Dir::Builtin(b.name);
        }
    }
    Dir::Builtin("default")
}

/// The theme folders in search order (a name once), and each one's title (theme.json "name", else the folder).
pub fn theme_names(roots: &[PathBuf], builtin: &[Builtin]) -> (Vec<String>, Map<String, Value>) {
    let mut names: Vec<String> = vec![];
    let mut titles = Map::new();
    let mut add = |n: String, dir: Dir| {
        if names.contains(&n) {
            return;
        }
        let title = dir.json(builtin).get("name").cloned().unwrap_or_else(|| Value::String(n.clone()));
        titles.insert(n.clone(), title);
        names.push(n);
    };
    for r in roots {
        let mut found: Vec<String> = std::fs::read_dir(r)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().join("theme.json").is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        found.sort(); // QDir's order
        for n in found {
            add(n.clone(), Dir::Fs(r.join(&n)));
        }
    }
    for b in builtin {
        add(b.name.into(), Dir::Builtin(b.name));
    }
    (names, titles)
}

/// A custom theme from before `import Rela` (it used the removed lowercase names) would open with every binding
/// failing. shortcut: only its Main.qml is checked; a theme that imports re-la there but not in a page still fails on
/// that page.
pub fn outdated(main_qml: &str) -> bool {
    !main_qml.contains("import Rela")
}

#[derive(Debug, PartialEq)]
pub struct Theme {
    pub main_url: String,
    pub tokens: Map<String, Value>,
    pub fonts: Vec<String>,
    pub outdated: bool,
}

/// What loading `name` means: its Main.qml (or its base's), the tokens (theme.json over the default theme's, plus
/// `dir` for theme images), the fonts to add.
pub fn resolve(name: &str, roots: &[PathBuf], builtin: &[Builtin]) -> Theme {
    let mut dir = theme_dir(name, roots, builtin);
    let old = match &dir {
        Dir::Fs(p) => std::fs::read_to_string(p.join("Main.qml")).ok().is_some_and(|t| outdated(&t)),
        Dir::Builtin(_) => false,
    };
    if old {
        dir = Dir::Builtin("default");
    }
    let mut tokens = Dir::Builtin("default").json(builtin); // defaults for missing tokens
    tokens.extend(dir.json(builtin));
    let fonts = tokens.get("fonts").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(|f| dir.file(f)).collect();
    tokens.insert("dir".into(), Value::String(dir.url("")));
    let main_url = if dir.has_main(builtin) {
        dir.url("Main.qml")
    } else {
        let base = tokens.get("base").and_then(Value::as_str).unwrap_or("default").to_string();
        theme_dir(&base, roots, builtin).url("Main.qml")
    };
    Theme { main_url, tokens, fonts, outdated: old }
}

/// The user data dir's themes (Qt's AppDataLocation for org/app "re-la"), then the ones next to the exe.
fn roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    let data = PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default());
    #[cfg(not(windows))]
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"));
    let mut r = vec![data.join("re-la/re-la/themes")];
    if let Some(exe) = std::env::current_exe().ok().as_deref().and_then(Path::parent) {
        r.push(exe.join("themes"));
    }
    r
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qmap.h");
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QMap_QString_QVariant, theme, READ, NOTIFY = theme_changed)] // theme.json over the default's tokens, plus `dir`
        #[qproperty(QStringList, theme_names, cxx_name = "themeNames", READ, CONSTANT)] // the theme folders
        #[qproperty(QMap_QString_QVariant, theme_titles, cxx_name = "themeTitles", READ, CONSTANT)] // folder -> its "name"
        #[qproperty(bool, tray_available, cxx_name = "trayAvailable", READ, WRITE, NOTIFY = tray_changed)] // set by Shell.qml
        #[qproperty(QString, build_date, cxx_name = "buildDate", READ, CONSTANT)] // "12.11.2026 14:05", UTC
        type App = super::AppRust;
    }

    unsafe extern "RustQt" {
        #[qsignal]
        #[cxx_name = "themeChanged"]
        fn theme_changed(self: Pin<&mut App>);
        #[qsignal]
        #[cxx_name = "trayChanged"]
        fn tray_changed(self: Pin<&mut App>);
        /// image://cover/ for this engine (Shell.qml, first thing): its types are registered once QML imports re-la.
        #[qinvokable]
        #[cxx_name = "addCoverProvider"]
        fn add_cover_provider(self: Pin<&mut App>);
        /// Makes `name` the theme being drawn (tokens, fonts) and returns the Main.qml URL to load.
        #[qinvokable]
        fn load(self: Pin<&mut App>, name: &QString) -> QString;
    }
}

pub struct AppRust {
    theme: QMap<QMapPair_QString_QVariant>,
    theme_names: QStringList,
    theme_titles: QMap<QMapPair_QString_QVariant>,
    tray_available: bool,
    build_date: QString,
}

impl Default for AppRust {
    fn default() -> Self {
        let (names, titles) = theme_names(&roots(), BUILTIN);
        let mut list = QStringList::default();
        for n in &names {
            list.append(QString::from(n.as_str()));
        }
        AppRust {
            theme: QMap::default(),
            theme_names: list,
            theme_titles: to_qvariant_map(&Value::Object(titles)),
            tray_available: false,
            build_date: QString::from(env!("RELA_BUILD_DATE")),
        }
    }
}

static FONTS: Mutex<Option<HashSet<String>>> = Mutex::new(None); // the font database keeps every add: once each

impl ffi::App {
    fn add_cover_provider(self: Pin<&mut Self>) {
        let me = &*self as *const ffi::App as *const crate::qtgui::ffi::QObject; // QObject is its base
        // SAFETY: me is this live App; qmlEngine only reads its QML context
        let engine = unsafe { crate::qtgui::ffi::qml_engine(me) };
        // SAFETY: null or the engine that made this App, which outlives it
        match (unsafe { engine.as_mut() }, crate::covers::new_provider()) {
            // the engine owns the provider; its bases are one chain down to QQmlImageProviderBase's QObject
            // SAFETY: e is not moved (Qt objects never are); p is a fresh CoverProvider the engine takes over
            (Some(e), Some(p)) => unsafe {
                Pin::new_unchecked(e).add_image_provider(&QString::from("cover"), p as *mut cxx_qt_lib::QQmlImageProviderBase)
            },
            _ => eprintln!("covers: no engine or CoverProvider is not registered"),
        }
    }
    fn load(mut self: Pin<&mut Self>, name: &QString) -> QString {
        let name = name.to_string();
        let t = resolve(&name, &roots(), BUILTIN);
        if t.outdated {
            eprintln!("{name}: no `import Rela` in Main.qml, see docs/themes.md; loading the default theme");
            let msg = format!("“{name}” teması eski bir sürüm için yazılmış (import Rela yok); varsayılan tema açıldı.");
            player().call(move |c| c.set_status(msg));
        }
        {
            let mut added = FONTS.lock().unwrap();
            let added = added.get_or_insert_with(HashSet::new);
            for f in t.fonts {
                if added.insert(f.clone()) {
                    crate::qtgui::ffi::QFontDatabase::add_application_font(&QString::from(f.as_str()));
                }
            }
        }
        self.as_mut().rust_mut().theme = to_qvariant_map(&Value::Object(t.tokens));
        self.as_mut().theme_changed();
        QString::from(t.main_url.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: &[Builtin] = &[
        Builtin { name: "default", json: r##"{"name": "Default", "bg": "#000", "radius": 10, "fonts": []}"##, main: true },
        Builtin { name: "nothing", json: r#"{"name": "Nothing", "radius": 0, "fonts": ["fonts/Doto.ttf"]}"#, main: true },
    ];

    fn root(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("re-la-themes-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn theme(root: &Path, name: &str, json: &str, main: Option<&str>) {
        std::fs::create_dir_all(root.join(name)).unwrap();
        std::fs::write(root.join(name).join("theme.json"), json).unwrap();
        if let Some(m) = main {
            std::fs::write(root.join(name).join("Main.qml"), m).unwrap();
        }
    }

    #[test]
    fn names_in_search_order_once_each() {
        let (user, exe) = (root("names-user"), root("names-exe"));
        theme(&user, "zeta", r#"{"name": "Zeta"}"#, None);
        theme(&user, "alpha", "{}", None);
        theme(&exe, "nothing", r#"{"name": "Shadow"}"#, None); // next to the exe, before the built-in one
        std::fs::create_dir_all(user.join("not-a-theme")).unwrap();
        let (names, titles) = theme_names(&[user, exe], B);
        assert_eq!(names, ["alpha", "zeta", "nothing", "default"]);
        assert_eq!(titles["alpha"], "alpha", "no name: the folder");
        assert_eq!(titles["zeta"], "Zeta");
        assert_eq!(titles["nothing"], "Shadow");
    }

    #[test]
    fn builtin_theme() {
        let t = resolve("nothing", &[], B);
        assert_eq!(t.main_url, "qrc:/themes/nothing/Main.qml");
        assert_eq!((t.tokens["radius"].as_i64(), t.tokens["bg"].as_str()), (Some(0), Some("#000")), "own over the default's");
        assert_eq!(t.tokens["dir"], "qrc:/themes/nothing/");
        assert_eq!(t.fonts, [":/themes/nothing/fonts/Doto.ttf"]);
        assert!(!t.outdated);
        assert_eq!(resolve("../etc", &[], B).main_url, "qrc:/themes/default/Main.qml", "not a folder name: the default");
        assert_eq!(resolve("missing", &[], B).main_url, "qrc:/themes/default/Main.qml");
    }

    #[test]
    fn user_theme_without_main_uses_its_base() {
        let user = root("base");
        theme(&user, "mine", r#"{"base": "nothing", "accent": "red", "fonts": ["a.ttf"]}"#, None);
        let t = resolve("mine", std::slice::from_ref(&user), B);
        assert_eq!(t.main_url, "qrc:/themes/nothing/Main.qml");
        assert_eq!(t.tokens["accent"], "red");
        let dir = url::Url::from_directory_path(user.join("mine")).unwrap().to_string();
        assert_eq!(t.tokens["dir"], dir.as_str());
        assert_eq!(t.fonts, [user.join("mine").join("a.ttf").to_string_lossy()]);
    }

    #[test]
    fn user_theme_with_main_and_outdated_one() {
        let user = root("main");
        theme(&user, "new", "{}", Some("import QtQuick\nimport Rela\nWindow {}"));
        theme(&user, "old", r#"{"accent": "red"}"#, Some("import QtQuick\nWindow { color: theme.bg }"));
        let t = resolve("new", std::slice::from_ref(&user), B);
        assert!(t.main_url.ends_with("/new/Main.qml") && t.main_url.starts_with("file:///"), "{}", t.main_url);
        let t = resolve("old", std::slice::from_ref(&user), B);
        assert!(t.outdated);
        assert_eq!(t.main_url, "qrc:/themes/default/Main.qml");
        assert_eq!(t.tokens["dir"], "qrc:/themes/default/");
        assert!(!t.tokens.contains_key("accent"), "the default theme's tokens, not the old theme's");
    }
}
