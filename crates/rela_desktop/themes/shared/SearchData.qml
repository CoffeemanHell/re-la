// Search (search3): artists, albums and songs. The top bar field updates args while this page is open.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    property var saved: null  // from history: results and scroll
    property var res: null    // searchResult3; null while the request runs
    property bool failed: false
    property bool ready: false
    readonly property var artists: res ? res.artist || [] : []
    readonly property var albums: res ? res.album || [] : []
    readonly property var songs: res ? res.song || [] : []

    function load() {
        const q = args.query
        res = null
        failed = false
        Player.get(root, "search3", { query: q, artistCount: 6, albumCount: 12, songCount: 30 }, r => {
            if (q !== args.query) return // typed on: a newer request is coming
            if (!r) return failed = true
            res = r.searchResult3 || {}
        })
    }
    function saveState() { return { res: res, y: scroller.contentY } }
    onArgsChanged: if (ready) load()
    Component.onCompleted: {
        ready = true
        if (!saved) return load()
        res = saved.res
        Qt.callLater(() => scroller.contentY = saved.y)
    }
}
