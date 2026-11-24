// Right-click menu for a song, album, artist or playlist (nothing style: mono, uppercase). Main.qml keeps one (win.menu); a page may own one too.
// Control actions follow Sync.canControl; writes (favorite, rating, playlists, share) are left to the server.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import shared
import Rela

Menu {
    id: m
    property string kind: "song"
    property var item: ({})
    property int queueIndex: -1  // a queue row: offers "KUYRUKTAN ÇIKAR"
    property var removeFn: null  // a playlist row: offers "PLAYLIST'TEN ÇIKAR"
    property var w               // the window (a Menu is a popup: no ApplicationWindow attached here)
    readonly property bool ctl: Sync.canControl && !Player.station.id
    readonly property bool isSong: kind === "song"
    palette.window: App.theme.surface; palette.base: App.theme.surface; palette.text: App.theme.fg; palette.windowText: App.theme.fg
    palette.light: App.theme.surface2; palette.mid: App.theme.line; palette.dark: App.theme.line; palette.highlight: App.theme.fg; palette.highlightedText: App.theme.bg
    font.family: App.theme.font; font.pixelSize: 11

    component Act: MenuItem { height: visible ? implicitHeight : 0 } // hidden items take no room
    component Sep: MenuSeparator { height: visible ? implicitHeight : 0 }

    readonly property ItemActions act: ItemActions { kind: m.kind; item: m.item; w: m.w; owner: m } // a property, not a child: children become menu items

    onAboutToShow: {
        for (let i = 0; i < count; i++) { // the rating submenu's own item: songs only
            const it = itemAt(i) as MenuItem
            if (it && it.subMenu === rate) { it.visible = isSong; it.height = isSong ? it.implicitHeight : 0 }
        }
    }

    Act { text: "ÇAL"; enabled: m.ctl; onTriggered: m.isSong ? Player.add([m.item], 0) : m.act.playAll(false) }
    Act { text: "KARIŞTIR"; visible: m.kind === "album" || m.kind === "playlist"; enabled: m.ctl; onTriggered: m.act.playAll(true) }
    Act { text: "SIRADAKİ YAP"; visible: m.isSong || m.kind === "album"; enabled: m.ctl; onTriggered: m.act.withSongs(s => Player.add(s, 1)) }
    Act { text: "SIRAYA EKLE"; visible: m.kind !== "artist"; enabled: m.ctl; onTriggered: m.act.withSongs(s => Player.add(s, 2)) }
    Act {
        text: "ŞARKI RADYOSU"; visible: m.isSong; enabled: m.ctl
        onTriggered: { const it = m.item; Player.get(m, "getSimilarSongs", { id: it.id, count: 50 }, r => { if (!m.act.fail(r)) Player.play([it].concat(r.similarSongs.song || []), 0) }) }
    }
    Act {
        text: "SANATÇI RADYOSU"; visible: m.kind === "artist"; enabled: m.ctl
        onTriggered: Player.get(m, "getSimilarSongs2", { id: m.item.id, count: 50 }, r => { if (!m.act.fail(r)) Player.play(r.similarSongs2.song || [], 0) })
    }
    Sep { visible: m.kind !== "artist" }
    Act { text: "PLAYLIST'E EKLE…"; visible: m.isSong || m.kind === "album"; onTriggered: m.act.withSongs(s => { m.plMenu.songs = s; m.plMenu.popup() }) }
    Act { text: "İNDİR…"; visible: m.kind !== "artist"; onTriggered: m.w.download(m.item, m.kind) }
    Act { text: "BİLGİLERİ GÖSTER…"; visible: m.isSong; onTriggered: m.w.info(m.item.id) }
    Act { text: "PAYLAŞ…"; visible: m.isSong || m.kind === "album"; onTriggered: m.act.share() }
    Sep { visible: m.kind !== "playlist" }
    Act {
        text: m.item && m.item.starred ? "FAVORİLERDEN ÇIKAR" : "FAVORİLERE EKLE"
        visible: m.kind !== "playlist"; onTriggered: m.act.star()
    }
    Act { text: "KUYRUKTAN ÇIKAR"; visible: m.isSong && m.queueIndex >= 0; enabled: m.ctl; onTriggered: Player.remove(m.queueIndex) }
    Act { text: "PLAYLIST'TEN ÇIKAR"; visible: !!m.removeFn; onTriggered: m.removeFn() }
    Menu {
        id: rate
        title: "DERECELENDİRME"
        component Rate: MenuItem {
            property int n
            text: n ? "★".repeat(n) : "YOK"
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
