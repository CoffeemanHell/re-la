// Favorite state of a song, album or artist (star / unstar). Flips at once; a failed write flips back and says so.
// item: the object (its `starred` is the starting state); kind picks the id parameter. A theme's heart button shows `on` and calls toggle().
import QtQuick
import Rela

QtObject {
    property var item
    property string kind: "song"
    property var w     // the window: notify()
    property var owner // the button: a reply after it is gone is dropped
    property bool on: false
    property string setId: "" // flipped here: keep ours over the (stale) item object until the item changes
    function sync() { if (!item || item.id !== setId) { setId = ""; on = !!(item && item.starred) } }
    onItemChanged: sync()
    Component.onCompleted: sync()
    function toggle() {
        const was = on
        on = !was
        setId = item.id
        const key = kind === "album" ? "albumId" : kind === "artist" ? "artistId" : "id"
        Player.set(owner, was ? "unstar" : "star", { [key]: item.id }, r => {
            if (r) return
            on = was
            w.notify(qsTr("İşlem başarısız"))
        })
    }
}
