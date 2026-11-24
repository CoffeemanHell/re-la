// Small helpers every theme needs, done by the Rust core.
pragma Singleton
import QtQuick
import Rela

QtObject {
    // A copy of the songs in random order.
    function shuffled(songs) { return JSON.parse(Player.shuffledJson(JSON.stringify(songs))) }
    // File name a download is saved under. kind: song (original file), album / playlist (zip).
    function fileName(item, kind) { return Player.downloadNameJson(JSON.stringify(item), kind) }
}
