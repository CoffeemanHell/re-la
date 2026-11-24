// Songs like one song (getSimilarSongs), fetched when the list is shown or the song changes.
// The data only: a theme puts its list inside.
import QtQuick
import Rela

Item {
    id: similar
    property string songId: ""
    property bool shown: false
    property string forId: ""
    property var songs: []
    property bool loading: false
    property bool failed: false
    function load() {
        const id = songId
        if (!shown || !id || id === forId) return
        forId = id
        songs = []
        loading = true
        failed = false
        Player.get(similar, "getSimilarSongs", { id: id, count: 30 }, r => {
            if (id !== forId) return
            loading = false
            failed = !r
            songs = r ? r.similarSongs.song || [] : []
        })
    }
    onSongIdChanged: load()
    onShownChanged: load()
}
