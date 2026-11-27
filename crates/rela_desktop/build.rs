use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};
use std::path::Path;

/// The built-in themes (`:/themes/<folder>`): each folder's theme.json, for App to read without Qt's resource system.
fn builtin_themes(themes: &Path) {
    let mut out = String::from("pub const BUILTIN: &[Builtin] = &[\n");
    let mut dirs: Vec<_> = std::fs::read_dir(themes).unwrap().flatten().map(|e| e.path()).collect();
    dirs.sort();
    // default first: it is the fallback and the source of missing tokens
    dirs.sort_by_key(|d| d.file_name().is_none_or(|n| n != "default"));
    for d in dirs.iter().filter(|d| d.join("theme.json").is_file()) {
        let name = d.file_name().unwrap().to_string_lossy();
        let json = d.join("theme.json");
        out += &format!("    Builtin {{ name: {name:?}, json: include_str!({json:?}), main: {} }},\n", d.join("Main.qml").is_file());
    }
    out += "];\n";
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("builtin_themes.rs"), out).unwrap();
    println!("cargo::rerun-if-changed={}", themes.display());
}

/// Qt's lrelease: next to the qmake cxx-qt builds with (QMAKE, else qmake6 / qmake on PATH), else on PATH.
fn lrelease() -> Option<std::path::PathBuf> {
    use std::process::Command;
    let qmakes = std::env::var("QMAKE").ok().into_iter().chain(["qmake6".to_string(), "qmake".to_string()]);
    for q in qmakes {
        for var in ["QT_HOST_BINS", "QT_HOST_LIBEXECS", "QT_INSTALL_BINS"] {
            let Ok(o) = Command::new(&q).args(["-query", var]).output() else { continue };
            let exe = Path::new(String::from_utf8_lossy(&o.stdout).trim()).join("lrelease").with_extension(std::env::consts::EXE_EXTENSION);
            // a qtchooser wrapper with no Qt behind it exists but fails: probe it as the PATH ones are
            if o.status.success() && exe.is_file() && Command::new(&exe).arg("-version").output().is_ok_and(|o| o.status.success()) {
                return Some(exe);
            }
        }
    }
    ["lrelease-qt6", "lrelease"].into_iter().map(Into::into).find(|p: &std::path::PathBuf| Command::new(p).arg("-version").output().is_ok_and(|o| o.status.success()))
}

/// i18n/qml_<lang>.ts -> OUT_DIR/qml_<lang>.qm. QQmlApplicationEngine loads them from i18n/ next to Shell.qml for
/// Qt.uiLanguage (main.rs sets it). Without lrelease the app builds, in Turkish only.
fn translations(src: &Path) -> Vec<(String, std::path::PathBuf)> {
    println!("cargo::rerun-if-env-changed=QMAKE");
    let dir = src.join("i18n");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut ts: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ts")).collect();
    ts.sort();
    let Some(tool) = lrelease() else {
        println!("cargo::warning=lrelease (Qt Linguist tools) not found: re-la is built without translations, in Turkish only (after installing it: touch crates/rela_desktop/build.rs)");
        println!("cargo::rerun-if-env-changed=PATH");
        println!("cargo::rustc-env=RELA_TURKISH_ONLY=1"); // main.rs: the core then stays Turkish too
        return vec![];
    };
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    ts.iter()
        .map(|t| {
            let name = t.with_extension("qm").file_name().unwrap().to_string_lossy().into_owned();
            let qm = out.join(&name);
            let ok = std::process::Command::new(&tool).arg("-silent").arg(t).arg("-qm").arg(&qm).status().is_ok_and(|s| s.success());
            assert!(ok, "{} failed on {}", tool.display(), t.display());
            (name, qm)
        })
        .collect()
}

/// The built-in themes at :/themes/<folder>/ (loaded by path, as user themes are), the tray icon at :/re-la.svg and the
/// translations at :/qt/qml/Rela/qml/i18n/.
fn resources(src: &Path) -> std::path::PathBuf {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
    let themes = src.join("themes");
    let mut files = vec![];
    walk(&themes, &mut files);
    files.sort();
    let esc = |p: &Path| p.to_string_lossy().replace('\\', "/").replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;");
    let mut qrc = String::from("<RCC>\n<qresource prefix=\"/themes\">\n");
    for f in &files {
        qrc += &format!("<file alias=\"{}\">{}</file>\n", esc(f.strip_prefix(&themes).unwrap()), esc(f));
        println!("cargo::rerun-if-changed={}", f.display());
    }
    let icon = src.join("extras/linux/re-la.svg");
    qrc += &format!("</qresource>\n<qresource prefix=\"/\">\n<file alias=\"re-la.svg\">{}</file>\n", esc(&icon));
    for (name, qm) in translations(src) {
        qrc += &format!("<file alias=\"qt/qml/Rela/qml/i18n/{name}\">{}</file>\n", esc(&qm));
    }
    qrc += "</qresource>\n";
    qrc += "</RCC>\n";
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("rela.qrc"); // a C++ name: Q_INIT_RESOURCE(rela_qrc)
    std::fs::write(&out, qrc).unwrap();
    out
}

/// "12.11.2026 14:05" (UTC): SOURCE_DATE_EPOCH when set, so the same source builds byte for byte the same.
fn build_date() {
    println!("cargo::rerun-if-env-changed=SOURCE_DATE_EPOCH");
    let secs = std::env::var("SOURCE_DATE_EPOCH").ok().and_then(|s| s.parse::<i64>().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
    });
    let t = time::OffsetDateTime::from_unix_timestamp(secs).unwrap();
    println!("cargo::rustc-env=RELA_BUILD_DATE={:02}.{:02}.{} {:02}:{:02}", t.day(), u8::from(t.month()), t.year(), t.hour(), t.minute());
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    builtin_themes(&root.join("themes"));
    build_date();
    let qrc = resources(root);
    // re-la: Player (qml/Player.qml over the Rust PlayerCore), Sync, Subwave and App as singletons; Shell.qml is what
    // main loads first
    CxxQtBuilder::new_qml_module(
        QmlModule::new("Rela").qml_file(QmlFile::from("qml/Player.qml").singleton(true)).qml_file("qml/Shell.qml"),
    )
        .qt_module("Qml")
        .qt_module("Quick")
        .file("src/sync.rs")
        .file("src/subwave.rs")
        .file("src/player.rs")
        .file("src/covers.rs")
        .file("src/qtgui.rs")
        .file("src/settings.rs")
        .file("src/wheel.rs")
        .file("src/app.rs")
        .qrc(qrc)
        .build();
}
