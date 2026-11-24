// "Playlist'e ekle": the user's own playlists (loaded when opened) and "Yeni playlist…" on top.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import Rela

Menu {
    id: m
    property var songs: []
    property var w             // the window (Menu is a popup: no ApplicationWindow attached here)
    property var lists: null
    readonly property var ids: songs.map(s => s.id)
    function done(r, what) { w.notify(r ? what : "İşlem başarısız") }
    onAboutToShow: {
        lists = null
        Player.get(m, "getPlaylists", {}, r => lists = r ? (r.playlists.playlist || []).filter(p => p.owner === Player.user) : [])
    }
    MenuItem {
        text: "Yeni playlist…"
        icon.name: "list-add"
        onTriggered: m.w.askName("Yeni playlist", "", name => Player.set(m, "createPlaylist", { name: name, songId: m.ids }, r => m.done(r, "Playlist oluşturuldu: " + name)))
    }
    MenuSeparator {}
    MenuItem { text: "Yükleniyor…"; enabled: false; visible: m.lists === null; height: visible ? implicitHeight : 0 }
    Instantiator {
        model: m.lists || []
        delegate: MenuItem {
            required property var modelData
            text: modelData.name
            onTriggered: Player.set(m, "updatePlaylist", { playlistId: modelData.id, songIdToAdd: m.ids }, r => m.done(r, modelData.name + " listesine eklendi"))
        }
        onObjectAdded: (i, o) => m.insertItem(i + 3, o)
        onObjectRemoved: (i, o) => m.removeItem(o)
    }
}
