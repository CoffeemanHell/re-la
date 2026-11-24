// Right-click menu for a song, album, artist or playlist. Main.qml keeps one (win.menu); a page may own one too.
// Control actions follow Sync.canControl; writes (favorite, rating, playlists, share) are left to the server.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import shared
import Rela

Menu {
    id: m
    property string kind: "song"
    property var item: ({})
    property int queueIndex: -1  // a queue row: offers "Kuyruktan çıkar"
    property var removeFn: null  // a playlist row: offers "Playlist'ten çıkar"
    property var w               // the window (a Menu is a popup: no ApplicationWindow attached here)
    readonly property bool ctl: Sync.canControl && !Player.station.id
    readonly property bool isSong: kind === "song"

    component Act: MenuItem { height: visible ? implicitHeight : 0 } // hidden items take no room
    component Sep: MenuSeparator { height: visible ? implicitHeight : 0 }

    readonly property ItemActions act: ItemActions { kind: m.kind; item: m.item; w: m.w; owner: m } // a property, not a child: children become menu items

    onAboutToShow: {
        for (let i = 0; i < count; i++) { // the rating submenu's own item: songs only
            const it = itemAt(i) as MenuItem
            if (it && it.subMenu === rate) { it.visible = isSong; it.height = isSong ? it.implicitHeight : 0 }
        }
    }

    Act { text: "Çal"; icon.name: "media-playback-start"; enabled: m.ctl; onTriggered: m.isSong ? Player.add([m.item], 0) : m.act.playAll(false) }
    Act { text: "Karıştır"; icon.name: "media-playlist-shuffle"; visible: m.kind === "album" || m.kind === "playlist"; enabled: m.ctl; onTriggered: m.act.playAll(true) }
    Act { text: "Sıradaki yap"; icon.name: "media-playlist-append"; visible: m.isSong || m.kind === "album"; enabled: m.ctl; onTriggered: m.act.withSongs(s => Player.add(s, 1)) }
    Act { text: "Sıraya ekle"; icon.name: "list-add"; visible: m.kind !== "artist"; enabled: m.ctl; onTriggered: m.act.withSongs(s => Player.add(s, 2)) }
    Act {
        text: "Şarkı radyosu"; icon.name: "radio"; visible: m.isSong; enabled: m.ctl
        onTriggered: { const it = m.item; Player.get(m, "getSimilarSongs", { id: it.id, count: 50 }, r => { if (!m.act.fail(r)) Player.play([it].concat(r.similarSongs.song || []), 0) }) }
    }
    Act {
        text: "Sanatçı radyosu"; icon.name: "radio"; visible: m.kind === "artist"; enabled: m.ctl
        onTriggered: Player.get(m, "getSimilarSongs2", { id: m.item.id, count: 50 }, r => { if (!m.act.fail(r)) Player.play(r.similarSongs2.song || [], 0) })
    }
    Sep { visible: m.kind !== "artist" }
    Act { text: "Playlist'e ekle…"; icon.name: "view-media-playlist"; visible: m.isSong || m.kind === "album"; onTriggered: m.act.withSongs(s => { m.plMenu.songs = s; m.plMenu.popup() }) }
    Act { text: "İndir…"; icon.name: "download"; visible: m.kind !== "artist"; onTriggered: m.w.download(m.item, m.kind) }
    Act { text: "Bilgileri göster…"; icon.name: "documentinfo"; visible: m.isSong; onTriggered: m.w.info(m.item.id) }
    Act { text: "Paylaş…"; icon.name: "emblem-shared"; visible: m.isSong || m.kind === "album"; onTriggered: m.act.share() }
    Sep { visible: m.kind !== "playlist" }
    Act {
        text: m.item && m.item.starred ? "Favorilerden çıkar" : "Favorilere ekle"
        icon.name: "emblem-favorite"; visible: m.kind !== "playlist"; onTriggered: m.act.star()
    }
    Act { text: "Kuyruktan çıkar"; icon.name: "list-remove"; visible: m.isSong && m.queueIndex >= 0; enabled: m.ctl; onTriggered: Player.remove(m.queueIndex) }
    Act { text: "Playlist'ten çıkar"; icon.name: "list-remove"; visible: !!m.removeFn; onTriggered: m.removeFn() }
    Menu {
        id: rate
        title: "Derecelendirme"
        component Rate: MenuItem {
            property int n
            text: n ? "★".repeat(n) : "Yok"
            onTriggered: Player.set(m, "setRating", { id: m.item.id, rating: n }, r => m.act.fail(r))
        }
        Rate { n: 0 }
        Rate { n: 1 }
        Rate { n: 2 }
        Rate { n: 3 }
        Rate { n: 4 }
        Rate { n: 5 }
    }

    property PlaylistMenu plMenu: PlaylistMenu { w: m.w } // a property, not a child: children become submenus

}
