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
    function done(r, what) { w.notify(r ? what : qsTr("İşlem başarısız")) }
    onAboutToShow: {
        lists = null
        Player.get(m, "getPlaylists", {}, r => lists = r ? (r.playlists.playlist || []).filter(p => p.owner === Player.user) : [])
    }
    MenuItem {
        text: qsTr("Yeni playlist…")
        icon.name: "list-add"
        onTriggered: m.w.askName(qsTr("Yeni playlist"), "", name => Player.set(m, "createPlaylist", { name: name, songId: m.ids }, r => m.done(r, qsTr("Playlist oluşturuldu: %1").arg(name))))
    }
    MenuSeparator {}
    MenuItem { text: qsTr("Yükleniyor…"); enabled: false; visible: m.lists === null; height: visible ? implicitHeight : 0 }
    Instantiator {
        model: m.lists || []
        delegate: MenuItem {
            required property var modelData
            text: modelData.name
            onTriggered: Player.set(m, "updatePlaylist", { playlistId: modelData.id, songIdToAdd: m.ids }, r => m.done(r, qsTr("%1 listesine eklendi").arg(modelData.name)))
        }
        onObjectAdded: (i, o) => m.insertItem(i + 3, o)
        onObjectRemoved: (i, o) => m.removeItem(o)
    }
}
