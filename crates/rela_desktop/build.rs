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

/// The built-in themes at :/themes/<folder>/ (loaded by path, as user themes are) and the tray icon at :/re-la.svg.
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
    qrc += &format!("</qresource>\n<qresource prefix=\"/\">\n<file alias=\"re-la.svg\">{}</file>\n</qresource>\n", esc(&icon));
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
    let qrc = resources(&root);
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
