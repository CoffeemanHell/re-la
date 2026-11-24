// One genre: its albums (getAlbumList2 byGenre, 60 at a time) and "Karışık çal" (50 random songs of it).
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
    property var saved: null  // from history: the albums already loaded and the scroll position
    property var loaded: []
    property bool restoring: false // while the grid settles on the saved albums, scrolling must not fetch more

    function load() {
        if (loading || !more || restoring) return
        loading = true
        failed = false
        Player.get(root, "getAlbumList2", { type: "byGenre", genre: args.name, size: 60, offset: albums.count }, r => {
            loading = false
            if (!r) return failed = true
            const list = r.albumList2.album || []
            more = list.length === 60
            for (const a of list) { albums.append(a); loaded.push(a) }
        })
    }
    function shuffle() {
        Player.get(root, "getRandomSongs", { genre: args.name, size: 50 }, r => {
            const songs = r ? r.randomSongs.song || [] : []
            if (songs.length) Player.play(songs, 0)
        })
    }
    function saveState() { return { albums: loaded, more: more, y: scroller.contentY } }
    Component.onCompleted: {
        if (!saved) return load()
        restoring = true
        loaded = saved.albums
        albums.append(loaded)
        more = saved.more
        Qt.callLater(() => { scroller.contentY = saved.y; restoring = false })
    }
    readonly property ListModel albums: ListModel {}
}
