// One playlist (getPlaylist): play, shuffle; its owner can rename, delete and remove songs.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var saved: null  // from history: the playlist and the top row
    property var pl: null
    property var songs: []
    property bool failed: false
    property bool busy: false // one removal at a time: indexes shift after each
    readonly property bool editable: pl !== null && pl.owner === Player.user

    function load() {
        pl = null
        failed = false
        Player.get(root, "getPlaylist", { id: args.id }, r => {
            if (!r) return failed = true
            pl = r.playlist
            songs = pl.entry || []
        })
    }
    function fail() { w.notify("İşlem başarısız") }
    function removeAt(i) {
        if (busy) return
        const before = songs
        busy = true
        songs = songs.filter((s, k) => k !== i)
        Player.set(root, "updatePlaylist", { playlistId: pl.id, songIndexToRemove: [i] }, r => {
            busy = false
            if (!r) { songs = before; fail() }
        })
    }
    function rename() {
        w.askName("Yeniden adlandır", pl.name, name => {
            const before = pl.name
            pl = Object.assign({}, pl, { name: name })
            Player.set(root, "updatePlaylist", { playlistId: pl.id, name: name }, r => { if (!r) { pl = Object.assign({}, pl, { name: before }); fail() } })
        })
    }
    function remove() {
        w.confirm("Playlist silinsin mi?", () => Player.set(root, "deletePlaylist", { id: pl.id }, r => r ? w.go("playlists") : fail()))
    }
    function shuffled(a) { return Util.shuffled(a) }
    function saveState() { return { pl: pl, songs: songs, row: scroller.indexAt(30, scroller.contentY + 30) } }
    Component.onCompleted: {
        if (!saved) return load()
        pl = saved.pl
        songs = saved.songs
        Qt.callLater(() => { if (saved.row > 0) scroller.positionViewAtIndex(saved.row, ListView.Beginning) })
    }
}
