// Genres (getGenres) by name, with album and song counts. Click opens the genre page.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var saved: null
    property var genres: null
    property bool failed: false

    function load() {
        genres = null
        failed = false
        Player.get(root, "getGenres", {}, r => {
            if (!r) return failed = true
            genres = (r.genres.genre || []).slice().sort((a, b) => a.value.localeCompare(b.value))
        })
    }
    function saveState() { return { genres: genres, row: scroller.indexAt(30, scroller.contentY + 30) } }
    Component.onCompleted: {
        if (!saved) return load()
        genres = saved.genres
        Qt.callLater(() => { if (saved.row > 0) scroller.positionViewAtIndex(saved.row, ListView.Beginning) })
    }
}
