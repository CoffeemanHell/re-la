// Small helpers every theme needs, done by the Rust core.
pragma Singleton
import QtQuick
import Rela

QtObject {
    // Windows draws icon names from its own icon font (Segoe Fluent Icons, MDL2 Assets on 10). Qt knows most names (its
    // qwindowsiconengine.cpp); for these it has none, or another picture, so they go in as the font's character.
    readonly property var glyphs: ({
        "audio-headphones": "\ue7f6",
        "configure": "\ue713",
        "documentinfo": "\ue946",
        "download": "\ue896",
        "drive-harddisk": "\ueda2",
        "edit-clear-all": "\ue74d",
        "edit-rename": "\ue8ac",
        "edit-undo": "\ue7a7",
        "emblem-shared": "\ue72d",
        "input-keyboard": "\ue765",
        "media-optical-audio": "\ue93c",
        "media-playlist-append": "\uecc8",
        "media-playlist-repeat-song": "\ue8ed",
        "radio": "\ue93e",
        "user-identity": "\ue77b",
        "view-media-artist": "\ue77b",
        "view-media-genre": "\ue8ec",
        "view-media-playlist": "\ue8fd",
        "view-media-track": "\uec4f",
        "view-statistics": "\ue9d2"
    })
    // The name to give icon.name on this system.
    function icon(name) { return Qt.platform.os === "windows" && glyphs[name] || name }
    readonly property var cacheSizes: [0, 1, 2, 5, 10, 20, 50] // song cache limits in GB; 0: off
    function dataSize(bytes) { return Qt.locale().formattedDataSize(bytes, 1, Locale.DataSizeTraditionalFormat) }
    // The output devices to pick from, as {name, description}.
    function outputs() { return [{ name: "auto", description: qsTr("Sistem varsayılanı") }].concat(Player.audioDevices) }
    // A copy of the songs in random order.
    function shuffled(songs) { return JSON.parse(Player.shuffledJson(JSON.stringify(songs))) }
    // File name a download is saved under. kind: song (original file), album / playlist (zip).
    function fileName(item, kind) { return Player.downloadNameJson(JSON.stringify(item), kind) }
}
