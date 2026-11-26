//! re-la on the desktop: one instance per user, QApplication, the player (on first use), and the QML engine that
//! loads Shell.qml (the theme's window, the tray). `--demo` plays sample songs with no server; `--theme=<name>` starts
//! with that theme (saved like a normal switch).
#![cfg_attr(windows, windows_subsystem = "windows")] // no console window
use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};
use rela_desktop::hub::{PLAYER_QT, move_from_nsync, player, to};
use rela_desktop::qtgui::{QApplication, ffi as qt};
use std::ffi::{c_char, c_int};
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::time::Duration;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    #[cfg(windows)]
    fn _putenv_s(name: *const c_char, value: *const c_char) -> c_int;
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    fn mallopt(param: c_int, value: c_int) -> c_int;
}
#[cfg(all(target_os = "linux", target_env = "gnu"))]
const M_ARENA_MAX: c_int = -8; // malloc.h

/// Sets a variable where Qt reads it. On Windows Qt reads the C runtime's copy of the environment, made at start, which
/// std::env::set_var (SetEnvironmentVariableW) does not change; _putenv_s changes both.
#[cfg(windows)]
fn set_env_for_qt(name: &std::ffi::CStr, value: &std::ffi::CStr) {
    // SAFETY: two nul-terminated strings; the C runtime copies them. Called before Qt starts, from the main thread
    unsafe { _putenv_s(name.as_ptr(), value.as_ptr()) };
}
/// Windows' display language: Qt's default follows the regional format, which can be Turkish under an English UI.
#[cfg(windows)]
fn system_ui_language() -> Option<String> {
    // SAFETY: no arguments, no state; returns a LANGID
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    Some(if id & 0x3ff == 0x1f { "tr" } else { "en" }.into()) // 0x1f: LANG_TURKISH, the primary language bits
}
#[cfg(not(windows))]
fn system_ui_language() -> Option<String> {
    None // elsewhere: QLocale's language, as before
}
#[cfg(not(windows))]
const LC_NUMERIC: c_int = 1; // glibc
#[cfg(windows)]
const LC_NUMERIC: c_int = 4; // the UCRT

/// The socket a second launch knocks on.
#[cfg(unix)]
fn instance_socket() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()).map_or_else(std::env::temp_dir, PathBuf::from);
    dir.join(format!("re-la-{}.sock", std::env::var("USER").unwrap_or_default()))
}

/// Single instance: a second launch brings the running window back and ends. A stale socket from a crash is replaced.
#[cfg(unix)]
fn single_instance() -> Option<UnixListener> {
    let path = instance_socket();
    if let Ok(mut s) = UnixStream::connect(&path) {
        let _ = s.set_write_timeout(Some(Duration::from_millis(300)));
        let _ = s.write_all(b"show");
        std::process::exit(0);
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).ok();
    if let Some(l) = &listener {
        let l = l.try_clone().ok()?;
        std::thread::Builder::new()
            .name("re-la-instance".into())
            .spawn(move || {
                for mut s in l.incoming().flatten() {
                    let mut buf = [0u8; 16];
                    let _ = s.set_read_timeout(Some(Duration::from_millis(300)));
                    let _ = s.read(&mut buf);
                    to(&PLAYER_QT, |o| o.raise_requested());
                }
            })
            .ok()?;
    }
    listener
}

/// Windows: the same through a named pipe of this user. Any connection means "show".
#[cfg(windows)]
fn single_instance() {
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
    let name = format!(r"\\.\pipe\re-la-{}", std::env::var("USERNAME").unwrap_or_default());
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
    let _in = rt.enter(); // pipes register with the runtime's reactor
    match ClientOptions::new().open(&name) {
        Ok(c) => {
            let _ = c.try_write(b"show");
            std::process::exit(0);
        }
        Err(e) if e.raw_os_error() == Some(231) => std::process::exit(0), // ERROR_PIPE_BUSY: one runs, mid-knock
        Err(_) => {}
    }
    let Ok(mut server) = ServerOptions::new().first_pipe_instance(true).create(&name) else { return };
    drop(_in);
    let _ = std::thread::Builder::new().name("re-la-instance".into()).spawn(move || {
        rt.block_on(async move {
            loop {
                if server.connect().await.is_err() {
                    return;
                }
                to(&PLAYER_QT, |o| o.raise_requested());
                // the next knock gets a fresh instance; this one is dropped
                let Ok(next) = ServerOptions::new().create(&name) else { return };
                server = next;
            }
        });
    });
}

/// Session logout and kill send SIGTERM: quit through the event loop, so the queue is saved after exec() returns.
#[cfg(unix)]
fn quit_on_signals() {
    let _ = std::thread::Builder::new().name("re-la-signals".into()).spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("signal runtime");
        rt.block_on(async {
            use tokio::signal::unix::{SignalKind, signal};
            let (Ok(mut term), Ok(mut int)) = (signal(SignalKind::terminate()), signal(SignalKind::interrupt())) else { return };
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            if PLAYER_QT.lock().unwrap().is_none() {
                std::process::exit(1); // nothing loaded: nothing to save
            }
            to(&PLAYER_QT, |_| qt::QCoreApplication::quit()); // on the Qt thread; a second signal does nothing more
        });
    });
}

fn main() {
    // mpv fills its song cache (the whole file) from a new thread per song. With glibc's many arenas each such thread
    // can land in another one, which keeps the freed songs: RSS grew by a file per song. One arena reuses that memory.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: sets one number in glibc's malloc; called before any other thread exists
    unsafe {
        mallopt(M_ARENA_MAX, 1)
    };
    cxx_qt::init_crate!(rela_desktop);
    let _instance = single_instance(); // kept open for the whole run
    move_from_nsync(); // before anything reads the settings, the cache or the themes

    #[cfg(windows)]
    if std::env::var_os("QT_QUICK_CONTROLS_STYLE").is_none() {
        // the Default theme's look on Windows: one style on 10 and 11, light or dark like the system
        set_env_for_qt(c"QT_QUICK_CONTROLS_STYLE", c"FluentWinUI3");
    }
    // Qt Widgets' application, not QGuiApplication: KDE's org.kde.desktop style and the tray's fallback need it
    let mut app = QApplication::new();
    qt::QCoreApplication::set_organization_name(&QString::from("re-la"));
    qt::QCoreApplication::set_application_name(&QString::from("re-la"));
    cxx_qt_lib::QGuiApplication::set_desktop_file_name(&QString::from("re-la"));
    qt::QGuiApplication::set_quit_on_last_window_closed(false); // closing the window can leave it playing in the tray
    // SAFETY: a nul-terminated literal; the main thread, before any mpv deck exists
    unsafe { setlocale(LC_NUMERIC, c"C".as_ptr()) }; // libmpv needs it; Qt set the system locale (tr_TR: decimal comma)
    #[cfg(unix)]
    quit_on_signals();

    let mut engine = QQmlApplicationEngine::new();
    let Some(mut e) = engine.as_mut() else { std::process::exit(1) };
    // The language, before anything shows text: Turkish on a Turkish system, English on any other (RELA_LANG=en / tr
    // overrides it). QML gets it from i18n/qml_en.qm, the core's status lines from rela_core::i18n.
    let mut ae = rela_desktop::qtgui::app_engine(e.as_mut());
    let lang = std::env::var("RELA_LANG").ok().filter(|l| !l.is_empty()).unwrap_or_else(|| system_ui_language().unwrap_or_else(|| ae.ui_language().to_string()));
    // no English catalog in this build: the core follows the Turkish QML
    let lang = if rela_core::i18n::is_english(&lang) && option_env!("RELA_TURKISH_ONLY").is_none() { "en" } else { "tr" };
    rela_core::i18n::set_language(lang);
    ae.as_mut().set_ui_language(&QString::from(lang));

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--demo") {
        player().call(|c| c.demo());
    }
    if let Some(name) = args.iter().find_map(|a| a.strip_prefix("--theme=")) {
        let name = name.to_string();
        player().call(move |c| c.set_theme(&name)); // before the engine loads: Player starts with it
    }

    e.as_mut().add_import_path(&QString::from("qrc:/themes")); // `import shared`: the pages' data and actions
    e.as_mut().on_object_creation_failed(|_, _| std::process::exit(1)).release(); // Shell.qml itself broken
    e.as_mut().load(&QUrl::from("qrc:/qt/qml/Rela/qml/Shell.qml"));
    let code = app.pin_mut().exec();

    // where we left off, for the next start; followers keep playing on their own, now
    rela_desktop::mpris::quit();
    player().persist_now();
    player().sync().stop(true);
    drop(engine);
    std::process::exit(code);
}

#[cfg(all(test, windows))]
mod tests {
    unsafe extern "C" {
        fn _wgetenv(name: *const u16) -> *const u16; // what Qt's qEnvironmentVariable reads
    }

    fn crt(name: &str) -> Option<String> {
        let w: Vec<u16> = name.encode_utf16().chain([0]).collect();
        // SAFETY: w ends with 0; the test's thread is the only one touching this variable
        let p = unsafe { _wgetenv(w.as_ptr()) };
        (!p.is_null()).then(|| {
            // SAFETY: p is a nul-terminated wide string from the C runtime, read up to its 0
            let n = (0..).take_while(|&i| unsafe { *p.add(i) } != 0).count();
            // SAFETY: the n units before that 0
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
        })
    }

    #[test]
    fn qt_sees_the_style_we_set() {
        // SAFETY: a variable no other test reads; this test binary has no thread reading the environment meanwhile
        unsafe { std::env::set_var("RELA_TEST_STD", "x") };
        assert_eq!(crt("RELA_TEST_STD"), None, "set_var does not reach the C runtime's copy (why set_env_for_qt exists)");
        super::set_env_for_qt(c"RELA_TEST_CRT", c"FluentWinUI3");
        assert_eq!(crt("RELA_TEST_CRT").as_deref(), Some("FluentWinUI3"));
    }
}
