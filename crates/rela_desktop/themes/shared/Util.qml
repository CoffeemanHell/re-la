// Small helpers every theme needs, done by the Rust core.
pragma Singleton
import QtQuick
import Rela

QtObject {
    // The name to give icon.name. Plain theme names everywhere: the Windows build ships the Breeze icon theme.
    function icon(name) { return name }
    // Tool tips as plain text (they carry song names: never markup). The text item is the tip's content item, or its
    // first child under the KDE style (org.kde.desktop wraps the Label in an Item).
    function plainTips(tip) {
        const c = tip && tip.contentItem
        const t = c && (c.textFormat !== undefined ? c : c.children[0])
        if (t && t.textFormat !== undefined) t.textFormat = Text.PlainText
    }
    readonly property var cacheSizes: [0, 1, 2, 5, 10, 20, 50] // song cache limits in GB; 0: off
    function dataSize(bytes) { return Qt.locale().formattedDataSize(bytes, 1, Locale.DataSizeTraditionalFormat) }
    // The output devices to pick from, as {name, description}.
    function outputs() { return [{ name: "auto", description: qsTr("Sistem varsayılanı") }].concat(Player.audioDevices) }
    // A copy of the songs in random order.
    function shuffled(songs) { return JSON.parse(Player.shuffledJson(JSON.stringify(songs))) }
    // File name a download is saved under. kind: song (original file), album / playlist (zip).
    function fileName(item, kind) { return Player.downloadNameJson(JSON.stringify(item), kind) }
}
