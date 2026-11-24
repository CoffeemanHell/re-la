# re-la

A small native music player for [Navidrome](https://www.navidrome.org/) that lets people listen to the same song at the same time. Built with Qt 6 (QML) and libmpv. Runs on Windows, Linux and Android. The interface is in Turkish only.

## Download

Get the latest build from [Releases](../../releases). re-la was called Nsync: on the first start it takes over Nsync's login, settings and queue.

- **Windows:** `re-la-*-windows-x64.zip`. Unpack it anywhere and run `re-la.exe` from the folder; nothing to install. Your login and settings from the older single-exe versions come over on the first start.
- **Linux (Fedora):** `re-la-*-fedoraNN-x86_64.rpm`, built for that Fedora release. Install with `sudo dnf install ./re-la-*.rpm`; it uses the system Qt and libmpv, so it follows your desktop theme. Other distros: build from source (below).
- **Android:** `re-la-*-android-arm64.apk`, for Android 15 or newer on 64-bit ARM.

## Build on Linux

Requires Qt 6.11.2+ (with Qt Widgets and the Qt.labs.platform QML module), libmpv from mpv 0.41 or newer, Rust (cargo, rustc 1.88+) and a C++17 compiler. The first build needs the network (it fetches the Rust crates).

```bash
cargo build --release --locked -p rela_desktop
./target/release/re-la
```

## Build on Windows

In the MSYS2 UCRT64 shell, with the packages and libmpv listed at the top of
[`package.sh`](crates/rela_desktop/extras/windows/package.sh):

```bash
LIBMPV=/c/libmpv bash crates/rela_desktop/extras/windows/package.sh
```

The folder is `dist/re-la/`, the zip next to it.

## Android

The Android app shares the player and listening together with the desktop (the Rust core) and has its own Material 3
screens. To build it yourself, see [Development](docs/development.md#android).

## Listening together

Listening together needs the relay running next to Navidrome. See [relay/README.md](relay/README.md). Without the relay, re-la works as a normal player.

## More

- [Using re-la](docs/usage.md)
- [Themes](docs/themes.md)
- [Development](docs/development.md)

## Thanks

re-la is inspired by [Navic](https://codeberg.org/paige/Navic) and [Supersonic](https://github.com/supersonic-app/supersonic).

## License

re-la and re-with are free software under the [GNU AGPL 3.0](LICENSE) (`AGPL-3.0-only`). Parts by others keep their own license:

- The Android icons (`android/app/src/main/res/drawable/`, all but `ic_launcher_*`) are Google's [Material Symbols](https://github.com/google/material-design-icons), without their tint; `pause_to_play.xml` animates two of them: [Apache-2.0](LICENSES/Apache-2.0.txt).
- The fonts Doto and Space Mono (`crates/rela_desktop/themes/nothing/fonts/`): SIL Open Font License, its text next to them.

libmpv and FFmpeg are [GPL](LICENSES/GPL-3.0.txt)/LGPL, and Qt is [LGPL-3.0](LICENSES/LGPL-3.0.txt); the Windows zip includes them. Their sources: [mpv](https://github.com/mpv-player/mpv), [FFmpeg](https://ffmpeg.org/), [the libmpv build](https://github.com/shinchiro/mpv-winbuild-cmake), [Qt and the other DLLs, as MSYS2 builds them](https://github.com/msys2/MINGW-packages).
