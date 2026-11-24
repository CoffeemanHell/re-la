// Favorite toggle (star / unstar). Flips at once; a failed write flips back and says so.
// item: a song, album or artist object (its `starred` is the starting state); kind picks the id parameter.
import QtQuick
import QtQuick.Controls.Basic
import shared

Pill {
    id: h
    property var item
    property string kind: "song"
    readonly property var w: ApplicationWindow.window
    on: fav.on
    readonly property Favorite fav: Favorite { item: h.item; kind: h.kind; w: h.w; owner: h }
    text: on ? "♥" : "♡"
    onClicked: fav.toggle()
}
