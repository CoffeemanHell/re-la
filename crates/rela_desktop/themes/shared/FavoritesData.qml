// Starred songs, albums and artists (getStarred2), one pill each.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var args
    property var saved: null  // from history: the reply and the tab
    property var res: null    // starred2; null while loading
    property bool failed: false
    property int tab: 0
    readonly property var songs: res ? res.song || [] : []
    readonly property var albums: res ? res.album || [] : []
    readonly property var artists: res ? res.artist || [] : []

    function load() {
        res = null
        failed = false
        Player.get(root, "getStarred2", {}, r => { if (!r) return failed = true; res = r.starred2 || {} })
    }
    function saveState() { return { res: res, tab: tab } }
    Component.onCompleted: {
        if (!saved) return load()
        res = saved.res
        tab = saved.tab
    }
}
