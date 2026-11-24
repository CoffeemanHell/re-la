// "Playlist'e ekle": the user's own playlists (loaded when opened) and "Yeni playlist…" on top.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import Rela

Menu {
    id: m
    property var songs: []
    property var w             // the window (Menu is a popup: no ApplicationWindow attached here)
    property var lists: null
    readonly property var ids: songs.map(s => s.id)
    function done(r, what) { w.notify(r ? what : "İşlem başarısız") }
    palette.window: App.theme.surface; palette.base: App.theme.surface; palette.text: App.theme.fg; palette.windowText: App.theme.fg
    palette.light: App.theme.surface2; palette.mid: App.theme.line; palette.dark: App.theme.line; palette.highlight: App.theme.fg; palette.highlightedText: App.theme.bg
    font.family: App.theme.font; font.pixelSize: 11
    onAboutToShow: {
        lists = null
        Player.get(m, "getPlaylists", {}, r => lists = r ? (r.playlists.playlist || []).filter(p => p.owner === Player.user) : [])
    }
    MenuItem {
        text: "YENİ PLAYLIST…"
        onTriggered: m.w.askName("Yeni playlist", "", name => Player.set(m, "createPlaylist", { name: name, songId: m.ids }, r => m.done(r, "Playlist oluşturuldu: " + name)))
    }
    MenuSeparator {}
    MenuItem { text: "YÜKLENİYOR…"; enabled: false; visible: m.lists === null; height: visible ? implicitHeight : 0 }
    Instantiator {
        model: m.lists || []
        delegate: MenuItem {
            required property var modelData
            text: (modelData.name || "").toUpperCase()
            onTriggered: Player.set(m, "updatePlaylist", { playlistId: modelData.id, songIdToAdd: m.ids }, r => m.done(r, modelData.name + " listesine eklendi"))
        }
        onObjectAdded: (i, o) => m.insertItem(i + 3, o)
        onObjectRemoved: (i, o) => m.removeItem(o)
    }
}
