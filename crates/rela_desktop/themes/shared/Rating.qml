// Star rating 0..5 of a song (setRating): rate(n) sets it, the same n again clears it. A failed write goes back and says so.
import QtQuick
import Rela

QtObject {
    property var item
    property var w     // the window: notify()
    property var owner // the stars: a reply after they are gone is dropped
    property int rating: 0
    property string ratedId: "" // rated here: keep ours over the (stale) song object until the song changes
    function sync() { if (!item || item.id !== ratedId) { ratedId = ""; rating = item && item.userRating ? item.userRating : 0 } }
    onItemChanged: sync()
    Component.onCompleted: sync()
    function rate(n) {
        const was = rating, id = item.id
        rating = n === rating ? 0 : n
        ratedId = id
        Player.set(owner, "setRating", { id: id, rating: rating }, r => {
            if (r) return
            rating = was
            w.notify(qsTr("İşlem başarısız"))
        })
    }
}
