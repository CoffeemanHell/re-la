// Playlists (getPlaylists): name, song count, length, owner. "Yeni playlist" makes an empty one.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var args
    readonly property var w: Window.window
    property var lists: null
    property bool failed: false

    function load() {
        lists = null
        failed = false
        Player.get(root, "getPlaylists", {}, r => { if (!r) return failed = true; lists = r.playlists.playlist || [] })
    }
    function create() {
        w.askName("Yeni playlist", "", name => Player.set(root, "createPlaylist", { name: name }, r => r ? load() : w.notify("İşlem başarısız")))
    }
    Component.onCompleted: load()
}
