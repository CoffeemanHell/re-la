# Themes

Two themes are built in: **Default** (uses the system look: Breeze on KDE; on Windows the Fluent style, light or dark like the system, with its icon font) and **Nothing** (black and white, dot-matrix font, red accent).

## Your own theme

Create a folder:

- Windows: `%APPDATA%\re-la\re-la\themes\<name>\`
- Linux: `~/.local/share/re-la/re-la/themes/<name>/`

For new colors and fonts, a `theme.json` is enough. Default takes its colors from the system, so base a colored theme on Nothing:

```json
{
  "name": "Purple",
  "base": "nothing",
  "bg": "#120d1a", "surface": "#1b1426", "surface2": "#261c36",
  "fg": "#f3eefc", "muted": "#9a8fb0", "line": "#2e2440",
  "accent": "#b388ff", "accentFg": "#120d1a",
  "radius": 14,
  "font": "Inter", "fontSize": 13,
  "fonts": ["fonts/Inter.ttf"]
}
```

`fonts` paths are relative to the theme folder.

To change the layout, copy a built-in theme folder (`crates/rela_desktop/themes/default/` or `crates/rela_desktop/themes/nothing/`). `Main.qml` draws the window and each page is its own file (`HomePage.qml`, `AlbumPage.qml`, `NowPlayingPage.qml`, ...).

A theme only draws. What a page loads and does lives once, in `crates/rela_desktop/themes/shared/`, and is built into the app, so a copied theme finds it from any folder with `import shared`:

```qml
import shared

ArtistData {          // artist, topSongs, similar, load(), radio() ... come from shared/ArtistData.qml
    id: root
    scroller: flick   // the page's list or Flickable: its place is kept for back / forward
    Flickable { id: flick; /* the drawing */ }
}
```

- `…Data` (`AlbumData`, `PlaylistData`, `SearchData`, ...): one per page. `LyricsData` and `SimilarSongs` are the same for parts of Now Playing.
- `Navigation`: the page history behind the window's `go`, `back`, `forward`.
- `ItemActions`, `Favorite`, `Rating`: what a menu, a heart and the stars do.
- `Shortcuts` (the keyboard shortcuts, the user's or the standard ones: `Shortcut { sequences: Shortcuts.sequences("keyNext") }`) and `KeyCapture` (takes new keys for one).
- `Columns` (the track table's columns) and `Util` (`shuffled`, `fileName`, `icon`).
- Write icons as `icon.name: Util.icon("radio")`. On Linux the name comes from the system's icon theme. Windows has an icon font instead: a name that neither Qt nor `Util.icon` knows shows nothing there.

A new feature goes into `shared/` first; each theme then adds only how it looks.

## What QML can use

The app is the `Rela` module: `import Rela` gives the singletons `Player`, `Sync`, `Subwave` and `App` (see `crates/rela_desktop/`), typed, so `qmllint` checks a theme against them.

- `Player`: queue, current song, settings and commands. The full list is the `qproperty` and `qinvokable` lines in `crates/rela_desktop/src/player.rs`, plus the functions in `crates/rela_desktop/qml/Player.qml` (`get`, `set`, `play`, `add`, `setOption`, `download`). The most used:
  - `Player.get(page, method, params, r => ...)`: read from Navidrome (`get*` and `search3` only). `r` is the Subsonic response, or `null` on error.
  - `Player.set(page, method, params, r => ...)`: write, limited to `star`, `unstar`, `createPlaylist`, `updatePlaylist`, `deletePlaylist`, `setRating`, `createShare`.
  - `Player.queueLength`: the number of songs in the queue. Use it instead of `Player.queue.length`: each read of `Player.queue` copies every song.
  - `Player.play(songs, i)`, `Player.add(songs, where)` (`0` now, `1` next, `2` last).
  - `Player.shuffle`, `Player.repeat` (`0` off, `1` all, `2` one), `Player.dj`, `Player.bitrate`. `Player.waveform` and `Player.waveformRms` (0..1) are the current song's levels from the relay, empty until they come or when the relay has none.
  - `Player.options` and `Player.setOption(name, value)`.
  - `Player.online`; `Player.offline`: no server, but songs of the queue are on disk: show the player, not the login.
  - `Player.audioDevices` (`[{name, description}]`, after `Player.refreshAudioDevices()`), `Player.cacheBytes`, `Player.clearSongCache()`.
- `Sync`: the relay connection (online users, following, listeners).
- `Subwave`: the SUB/WAVE station that is playing (`active`, `now`, `coverUrl`, `elapsedMs`, `lyrics`, `request`, `like()`, `sendRequest(text, name)`). The full list is in `crates/rela_desktop/src/subwave.rs`.
- `App` (`crates/rela_desktop/src/app.rs`):
  - `App.theme`: the `theme.json` contents, plus `App.theme.dir` for the folder's address.
  - `App.themeNames`: the theme folders. `App.themeTitles[folder]` is the `name` from that theme's `theme.json`, shown in the settings.
  - `App.buildDate`: when the app was built, as text (`12.11.2026 14:05`, UTC). `App.trayAvailable`: the desktop has a tray.
- The window (`ApplicationWindow.window`): `go`, `back`, `notify`, `confirm`, `menu(type, item, options)`, `info(songId)`, `download(item, type)`, `openSettings(category)`.
- Text from outside (song, artist and user names, ...): set `textFormat: Text.PlainText`, as `Plain.qml` (Default) and `Mono.qml` (Nothing) do. A bare `Text` or `Label` reads a tag in a name as HTML, and an `<img>` there fetches from anywhere.
- Cover art: `Image { source: "image://cover/" + song.coverArt }`. The app downloads it; themes never see the Navidrome token.

**A theme written for an older re-la** (with `player.`, `theme.`, ... and no `import Rela`) no longer loads: those lowercase names were removed. To update it, add `import Rela` to each file and write `Player.` for `player.`, `Sync.` for `sync.`, `Subwave.` for `subwave.`, and `App.theme.` for `theme.` (likewise `App.themeNames`, `App.themeTitles`, `App.trayAvailable`, `App.buildDate`). Until then re-la opens the default theme instead and says why at the bottom.

Available modules: `QtQuick`, `QtQuick.Controls`, `QtQuick.Controls.Basic`, `QtQuick.Layouts`, `QtQuick.Effects`, `QtQuick.Dialogs`, `QtCore`.

To work on a theme without a server, run it with sample songs:

```bash
re-la --demo --theme=<name>
re-la --demo --settings --theme=<name>   # opens with the settings window
```

If a theme fails to load, the previous one stays and the error is shown at the bottom.
