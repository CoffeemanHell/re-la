# Development

## Layout

```
re-la (Qt/QML + Rust core) ──wss──▶ Caddy /re-with ──▶ re-with (Rust, 127.0.0.1:4534)
      │                                        │
      └── https (Subsonic API) ──▶ Navidrome ◀──┘ ping (login check)
```

| Path | What it does |
|---|---|
| `crates/rela_desktop/themes/` | QML themes, built into the binary as a Qt resource at `:/themes/<folder>/` (`shared` is the `import shared` module) |
| `crates/rela_desktop/tests/qml/` | QML tests against fake singletons (`Rela/`) |
| `crates/rela_desktop/extras/` | The Linux desktop file, icon and rpm spec; `windows/package.sh` builds the Windows zip |
| `crates/proto` | Relay protocol v1 message types, shared by the relay and the core |
| `crates/core` | The Rust core (no Qt): `sync` is the relay connection (online list, following, clock, commands), as a socket-free `SyncMachine` and the `SyncClient` that drives it; `subwave` is the SUB/WAVE listener API (parsing, and the client that detects, polls, fetches lyrics and hearts, follows requests); `player` is playback: `PlayerCore` decides everything (queue, two decks, crossfade, gapless, drift while listening together, reports and scrobbles, the DJ, stations, the saved queue) and is tested against a fake deck, `Player` runs it on its own thread with the relay and SUB/WAVE clients; plus the Subsonic client, settings (Qt's INI file on the desktop), the cover cache and the queue/DJ helpers |
| `crates/mpv` | The desktop deck: one libmpv (libmpv2) per deck |
| `crates/rela_desktop` | The desktop app (`re-la`). `main.rs`: single instance, QApplication, SIGTERM, the QML engine. The `Rela` QML module: `Player` (`qml/Player.qml`: the Rust `PlayerCore` plus `get`/`set`, whose `done` callbacks stay in JS, `copy` through a `TextEdit`, and the calls that take JS values as JSON), `Sync`, `Subwave` and `App` (theme tokens, theme list, build date; `app.rs` finds the themes) as singletons, and `qml/Shell.qml` (the theme's window, the tray, close to tray). All over the one player of the process (`hub.rs`); `image://cover/` (`covers.rs`), MPRIS on the session bus (`mpris.rs`, zbus), the KDE button-scroll filter (`wheel.rs`) |
| `crates/android` | The core for the Android app through UniFFI (`librela.so`, Kotlin package `uniffi.rela`): the same `Player` as the desktop, with the same calls; songs, replies and options cross as JSON text. The token, the salt and station passwords stay in Rust: Kotlin gets stream URLs only as a deck's `load`/`append` argument, and covers as bytes (`Core.cover`) |
| `android/` | The Android app (Kotlin, Jetpack Compose, Material 3 Expressive; its icons are Material Symbols Rounded as `res/drawable/ic_<name>.xml`, `_fill` for the filled ones). `playback/`: the ExoPlayer decks the core drives (`ExoDeck`), the media session's facade player and the foreground service (`RelaService`); `core/`: the `re-la` singleton (the core and its state as flows), songs, lyrics; `ui/`: the screens, covers through Coil from `Core.cover` |
| `relay/` | The relay, see [relay/README.md](../relay/README.md) |

No hand-written C++ and no `.js` files: the QApplication (Qt Widgets: KDE's `org.kde.desktop` style and the tray's
fallback need it) comes from `cxx-qt-lib-extras`. The build still needs a C++ compiler, for the code cxx-qt generates,
and QML's own expressions and functions are JavaScript.

## Build and test

Cargo builds everything (cxx-qt-build compiles the QML module and puts the built-in themes in as a Qt resource); the
first build downloads the crates, so it needs the network once.

```bash
cargo build --release --locked -p rela_desktop   # target/release/re-la
cargo test --workspace   # Rust: core, mpv deck (plays generated WAVs to mpv's null output), relay, desktop objects
# QML click tests: the themes' pages against fake singletons (crates/rela_desktop/tests/qml/Rela/)
QT_QPA_PLATFORM=offscreen QML_XHR_ALLOW_FILE_READ=1 XDG_CONFIG_HOME=$(mktemp -d) \
  /usr/lib64/qt6/bin/qmltestrunner -input crates/rela_desktop/tests/qml -import crates/rela_desktop/tests/qml -import crates/rela_desktop/themes
```

Debug switches: `RELA_AUDIO_LOG=1` prints what the audio output says about itself on each play; `RELA_AO=null`
plays to no sound device (mpv's audio outputs to try instead of `pipewire,pulse,alsa,`), for a run without one;
`RELA_WHEEL_LOG=1` prints every wheel event the button-scroll filter sees.

### Android

Needs the Android SDK (platform 37) with an NDK, JDK 21, and `cargo install cargo-ndk` plus
`rustup target add aarch64-linux-android`. `android/build-rust.sh` builds `librela.so` for 64-bit ARM (the app needs
Android 15+) into `app/src/main/jniLibs/` and writes the Kotlin bindings into `app/src/main/kotlin/uniffi/` (both
git-ignored); run it again after a change in `crates/`. Then Gradle builds the app:

```bash
android/build-rust.sh
cd android && ./gradlew assembleDebug testDebugUnitTest   # app/build/outputs/apk/debug/app-debug.apk
adb install -r app/build/outputs/apk/debug/app-debug.apk   # "app.rela.debug", next to a release build
```

The debug build is debuggable, so Compose runs much slower there: on a mid-range phone about 70% janky frames
scrolling the album grid and 1.2 s to the first frame, against under 1% and 0.3 s without it. Judge smoothness, and use
the app day to day, with the release build (`app.rela`, its own login):

```bash
./gradlew assembleRelease && adb install -r app/build/outputs/apk/release/app-release.apk
```

That one is signed with this computer's debug key, unless `RELA_KEYSTORE` (the key file, full path) and
`RELA_KEYSTORE_PASSWORD` are set. The release key is made once and kept outside the repo, with a backup: an APK signed
with another key can't update the installed app (Android asks to uninstall first, which deletes its login).

```bash
keytool -genkeypair -keystore ~/rela.keystore -storetype PKCS12 -alias rela -keyalg RSA -keysize 4096 -validity 10000 -dname CN=re-la
```

The `android` workflow signs with it when the repository has it as secrets:

```bash
base64 -w0 ~/rela.keystore | gh secret set ANDROID_KEYSTORE
gh secret set ANDROID_KEYSTORE_PASSWORD
```

To try the phone against a server on this computer, `adb reverse tcp:4533 tcp:4533` and log in to
`http://127.0.0.1:4533`. The relay is looked for at the same address under `/re-with` (here `ws://127.0.0.1:4533/re-with`),
or at the `relayUrl` option when it is set.

## Windows

The same app, built in MSYS2 UCRT64 (Qt 6, MinGW, Rust's `x86_64-pc-windows-gnu`) and shipped as a folder in a zip
(`crates/rela_desktop/extras/windows/package.sh`). What differs from Linux: one instance through a named pipe; media
keys and the media flyout through SMTC (`smtc.rs`) instead of MPRIS; the Fluent style and the system's icon font (`Util.icon`);
settings in `%APPDATA%\re-la\re-la.ini`, brought over once from the C++ app's registry key
(`HKCU\Software\nsync\nsync`); the cache in `%LOCALAPPDATA%\re-la\re-la\cache`. The Windows tests run in CI only.

**The name.** The app was Nsync and the relay nsync-relay. On its first start re-la moves Nsync's folders to its own
(`hub::move_from_nsync`: settings, cache and themes, on Linux and Windows); the relay is at `<server>/re-with` now.

## CI and releases

- A change in `crates/` builds Linux and Windows and runs the tests on both; the Windows zip is the run's
  `re-la-windows-x64` artifact. A change in `relay/` or `crates/proto` runs the relay tests.
- A change in `android/` or `crates/` builds the Android APKs and runs the unit tests and lint, where a warning fails
  (`android.yml`). The APKs are the run's `rela-android` (release; `rela-android-temporary-key` without the key
  above) and `rela-android-debug` artifacts. No Android release yet.
- `security.yml` runs `cargo deny check` (dependencies: known holes, licenses, sources; rules in `deny.toml`) and zizmor
  (the workflows), when they change and every Monday. Actions are pinned to commits; Dependabot moves the pins.
- Every push to `main` that changes the app publishes a release: the last `v*` tag's last number goes up by one.
- To pick a version yourself, push a tag: `git tag v0.1.0 && git push origin v0.1.0`. Tags with a dash (`-alpha`) are pre-releases.

## Relay protocol (v1)

Every message is a JSON object `{"type": ...}`, at most 1 MB, with at most 1000 songs in a queue.

Client → relay: `hello {v, u, t, s, hidden}` (must be first), `time {c}`, `state`, `follow {user}`, `unfollow`, `cmd {op, args, qrev}`, `presence {hidden}`, `bye`.

Relay → client: `welcome {you, now}`, `time {c, s}`, `users [...]`, `state`, `following {root}`, `cmd {from, op, args, qrev}`, `ended {reason}`, `error {code, message}`.

The `users` list has one entry per visible online user: `{user, following, station, title, artist, playing}`. `station` is the radio station name, or `null`. While a user is on a station their `state` carries `radio {name, title, artist}`; the relay lists `station`, `title` and `artist` from it, accepting strings only and cutting `name` to 100 and `title`/`artist` to 200 characters. Without a usable `radio.name` the entry shows the queue song, under the same limits.

Rules:

- **Login:** the relay calls Navidrome's `ping` with the user's token. The token is never stored or logged.
- **One connection per user:** a new connection replaces the old one.
- **Slow readers:** a client that stops reading (64 messages waiting) is dropped, so it never holds the relay's memory.
- **Following:** following someone joins the root of their session. The host keeps the state and the relay forwards it to followers.
- **Commands:** a follower's `cmd` reaches the host only if the host allows control. Index-based commands are ignored if the queue changed (`qrev`).
- **Host leaves:** after `bye` followers are released at once. After a dropped connection the session waits 60 s.
- **Clock:** clients take the fastest of 5 time samples (`offset = s − (c + rtt/2)`) and repeat every 30 s. Drift over 150 ms changes playback speed by 2%; over 2 s it seeks.

## Rules for commits

Never commit real server addresses, user names or tokens. Use `music.example.com` in examples.
