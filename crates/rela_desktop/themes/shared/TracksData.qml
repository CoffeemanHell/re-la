// Every song (search3 with an empty query), 100 at a time while scrolling. Double click plays from that row.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    property bool loading: false
    property bool failed: false
    property bool more: true
    property var saved: null  // from history: the songs already loaded and the top row
    property var loaded: []   // plain song objects; the model only holds indexes into it
    property bool restoring: false

    function load() {
        if (loading || !more || restoring) return
        loading = true
        failed = false
        Player.get(root, "search3", { query: "", songCount: 100, songOffset: loaded.length, artistCount: 0, albumCount: 0 }, r => {
            loading = false
            if (!r) return failed = true
            const list = r.searchResult3.song || []
            more = scroller.length === 100
            for (const s of list) { loaded.push(s); rows.append({ i: loaded.length - 1 }) }
        })
    }
    function saveState() { return { songs: loaded, more: more, row: scroller.indexAt(30, scroller.contentY + 30) } }
    Component.onCompleted: {
        if (!saved) return load()
        restoring = true
        loaded = saved.songs
        rows.append(loaded.map((s, i) => ({ i: i })))
        more = saved.more
        Qt.callLater(() => { if (saved.row > 0) scroller.positionViewAtIndex(saved.row, ListView.Beginning); restoring = false })
    }
    readonly property ListModel rows: ListModel {}
}
