// What can be done with a song, album, artist or playlist: play it, share it, make it a favorite.
// A theme's menu (or a page's buttons) calls these; no drawing here.
import QtQuick
import Rela

QtObject {
    property string kind: "song"
    property var item: ({})
    property var w     // the window: notify()
    property var owner // the menu or page asking: a reply after it is gone is dropped

    function fail(r) { if (!r) w.notify(qsTr("İşlem başarısız")); return !r }
    // The songs behind the item. Artist: the discography, albums by year, songs by disc and track.
    function songs(cb) {
        const it = item
        if (kind === "song") return cb([it])
        if (kind === "album") return Player.get(owner, "getAlbum", { id: it.id }, r => cb(r ? r.album.song || [] : null))
        if (kind === "playlist") return Player.get(owner, "getPlaylist", { id: it.id }, r => cb(r ? r.playlist.entry || [] : null))
        Player.get(owner, "getArtist", { id: it.id }, r => {
            if (!r) return cb(null)
            const albums = (r.artist.album || []).slice().sort((a, b) => (a.year || 0) - (b.year || 0) || (a.name || "").localeCompare(b.name || ""))
            const parts = []
            let left = albums.length
            if (!left) return cb([])
            albums.forEach((a, i) => Player.get(owner, "getAlbum", { id: a.id }, r2 => {
                parts[i] = r2 ? (r2.album.song || []).slice().sort((x, y) => (x.discNumber || 1) - (y.discNumber || 1) || (x.track || 0) - (y.track || 0)) : []
                if (--left === 0) cb([].concat(...parts))
            }))
        })
    }
    function withSongs(f) { songs(s => { if (!fail(s) && s.length) f(s) }) }
    function playAll(shuffle) { withSongs(s => Player.play(shuffle ? Util.shuffled(s) : s, 0)) }
    function share() {
        Player.set(owner, "createShare", { id: item.id }, r => {
            const url = r && r.shares && r.shares.share && r.shares.share[0] ? r.shares.share[0].url : ""
            if (!url) return w.notify(qsTr("İşlem başarısız"))
            Player.copy(url)
            w.notify(qsTr("Link kopyalandı"))
        })
    }
    // the DJ: more (1) or less (-1) like this song, 0 takes it back
    function djFeedback(dir) {
        Player.djFeedback(item, dir)
        w.notify(dir > 0 ? qsTr("DJ benzerlerini daha çok çalacak") : dir < 0 ? qsTr("DJ bunu daha az çalacak") : qsTr("Geri bildirim kaldırıldı"))
    }
    function star() {
        const on = !!item.starred
        const key = kind === "album" ? "albumId" : kind === "artist" ? "artistId" : "id"
        Player.set(owner, on ? "unstar" : "star", { [key]: item.id }, r => { if (!fail(r)) w.notify(on ? qsTr("Favorilerden çıkarıldı") : qsTr("Favorilere eklendi")) })
    }
}
