// All albums as a grid, in the chosen order; 60 more are fetched near the end of the list.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property string type: args.type || "newest"
    property bool loading: false
    property bool failed: false
    property bool more: true
    property int gen: 0   // bumped on a new order: replies for the old one are dropped
    property var saved: null  // from history: the albums already loaded and the scroll position
    property var loaded: []
    property bool restoring: false // while the grid settles on the saved albums, scrolling must not fetch more

    function load(reset) {
        if (reset) { albums.clear(); loaded = []; more = true; failed = false; loading = false; gen++ }
        if (loading || !more || restoring) return
        loading = true
        const g = gen
        Player.get(root, "getAlbumList2", { type: type, size: 60, offset: albums.count }, r => {
            if (g !== gen) return
            loading = false
            if (!r) return failed = true
            const list = r.albumList2.album || []
            more = list.length === 60
            for (const a of list) { albums.append(a); loaded.push(a) }
        })
    }
    function order(t) {
        type = t
        w.history[w.at].args = { type: t } // back returns to this order
        load(true)
    }
    function saveState() { return { albums: loaded, more: more, y: scroller.contentY } }
    Component.onCompleted: {
        if (!saved) return load(true)
        restoring = true
        loaded = saved.albums
        albums.append(loaded) // all at once: one by one would look like a short page and fetch more
        more = saved.more
        Qt.callLater(() => { scroller.contentY = saved.y; restoring = false })
    }
    readonly property ListModel albums: ListModel {}
}
