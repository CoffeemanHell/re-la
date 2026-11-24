// One album (getAlbum): header with play / shuffle / add, then the songs (disc headers on multi-disc albums).
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var album: null
    property var songs: []
    property bool loading: false
    property bool failed: false
    property var saved: null  // from history: the album and the scroll position
    readonly property bool multiDisc: new Set(songs.map(s => s.discNumber || 1)).size > 1

    function load() {
        loading = true
        failed = false
        Player.get(root, "getAlbum", { id: args.id }, r => {
            loading = false
            if (!r) return failed = true
            album = r.album
            songs = r.album.song || []
        })
    }
    function length(d) { return d >= 3600 ? Math.floor(d / 3600) + " sa " + Math.floor(d % 3600 / 60) + " dk" : Math.round(d / 60) + " dk" }
    function shuffled(a) { return Util.shuffled(a) }
    function saveState() { return { album: album, songs: songs, row: scroller.indexAt(30, scroller.contentY) } } // -1: header
    Component.onCompleted: {
        if (!saved) return load()
        album = saved.album
        songs = saved.songs
        Qt.callLater(() => { if (saved.row > 0) scroller.positionViewAtIndex(saved.row, ListView.Beginning) })
    }
}
