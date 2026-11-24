// All artists (getArtists), grouped by first letter. Click opens the artist page.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var saved: null  // from history: the list and the top row
    property var artists: null // flat, each with `letter`; null while loading
    property bool failed: false

    function load() {
        artists = null
        failed = false
        Player.get(root, "getArtists", {}, r => {
            if (!r) return failed = true
            const out = []
            for (const ix of r.artists.index || [])
                for (const a of ix.artist || []) out.push(Object.assign({ letter: ix.name }, a))
            artists = out
        })
    }
    function saveState() { return { artists: artists, row: scroller.indexAt(30, scroller.contentY + 30) } }
    Component.onCompleted: {
        if (!saved) return load()
        artists = saved.artists
        Qt.callLater(() => { if (saved.row > 0) scroller.positionViewAtIndex(saved.row, ListView.Beginning) })
    }
}
