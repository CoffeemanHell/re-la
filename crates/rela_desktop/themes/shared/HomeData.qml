// Home: four strips of albums from getAlbumList2. "TÜMÜ" opens the Albums page with that order.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var strips: [] // [[album list type, title]]: the theme names them
    property var lists: ({})   // type -> albums
    property int pending: 0
    property bool failed: false
    property var saved: null  // from history: strips and scroll positions
    property var stripX: ({})  // type -> strip scroll
    readonly property bool empty: Object.keys(lists).every(t => lists[t].length === 0)

    function load() {
        lists = {}
        failed = false
        pending = strips.length
        for (const [type] of strips)
            Player.get(root, "getAlbumList2", { type: type, size: 20 }, r => {
                pending--
                if (!r) return failed = true
                const l = Object.assign({}, lists)
                l[type] = r.albumList2.album || []
                lists = l
            })
    }
    function saveState() { return { lists: lists, y: scroller.contentY, x: stripX } }
    Component.onCompleted: {
        if (!saved) return load()
        lists = saved.lists
        Qt.callLater(() => scroller.contentY = saved.y)
    }
}
