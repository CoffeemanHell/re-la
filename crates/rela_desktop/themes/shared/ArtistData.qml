// One artist: picture and bio (getArtistInfo2), play top songs / artist radio, popular songs, albums, similar artists.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var scroller // the theme's list or flickable: its place is kept in the history
    property var args
    readonly property var w: Window.window
    property var artist: null
    property var info: null
    property var topSongs: []
    property bool loading: false
    property bool failed: false
    property var saved: null  // from history: what was loaded and the scroll position
    property real pendingY: -1 // restored once the page is tall enough
    readonly property string bio: info && info.biography ? info.biography.replace(/<a[^>]*>.*?<\/a>/g, "").replace(/<[^>]+>/g, "").trim() : ""
    readonly property var similar: info ? (info.similarArtist || []).filter(a => a.id) : [] // only artists in this library

    function load() {
        loading = true
        failed = false
        Player.get(root, "getArtist", { id: args.id }, r => {
            loading = false
            if (!r) return failed = true
            artist = r.artist
            Player.get(root, "getTopSongs", { artist: r.artist.name, count: 50 }, t => topSongs = t ? t.topSongs.song || [] : [])
        })
        Player.get(root, "getArtistInfo2", { id: args.id }, r => info = r ? r.artistInfo2 : null)
    }
    function radio() {
        Player.get(root, "getSimilarSongs2", { id: args.id, count: 50 }, r => { if (r) Player.play(r.similarSongs2.song || [], 0) })
    }
    function saveState() { return { artist: artist, info: info, topSongs: topSongs, y: scroller.contentY } }
    Component.onCompleted: {
        if (!saved) return load()
        artist = saved.artist
        info = saved.info
        topSongs = saved.topSongs
        pendingY = saved.y
    }
}
