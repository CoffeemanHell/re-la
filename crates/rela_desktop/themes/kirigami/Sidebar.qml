// The left sidebar: Home, the pages, then Settings and Listen together. Narrow windows get icons only.
import QtQuick
import org.kde.kirigami as Kirigami
import shared
import Rela

Kirigami.GlobalDrawer {
    id: drawer
    required property var w
    modal: false
    collapsible: true
    collapsed: w.width < 1100
    collapseButtonVisible: false // the window's width decides
    enabled: w.usable
    visible: w.usable
    drawerOpen: w.usable

    component Nav: Kirigami.Action {
        required property string target
        objectName: "nav_" + target
        checkable: true
        checked: drawer.w.page === target
        onTriggered: { drawer.w.go(target); checked = Qt.binding(() => drawer.w.page === target) }
    }
    actions: [
        Nav { target: "home"; icon.name: "go-home"; text: qsTr("Ana sayfa") },
        Nav { target: "nowplaying"; icon.name: Util.icon("audio-headphones"); text: qsTr("Şimdi çalan") },
        Nav { target: "favorites"; icon.name: "emblem-favorite"; text: qsTr("Favoriler") },
        Nav { target: "albums"; icon.name: Util.icon("media-optical-audio"); text: qsTr("Albümler") },
        Nav { target: "artists"; icon.name: Util.icon("view-media-artist"); text: qsTr("Sanatçılar") },
        Nav { target: "genres"; icon.name: Util.icon("view-media-genre"); text: qsTr("Türler") },
        Nav { target: "playlists"; icon.name: Util.icon("view-media-playlist"); text: qsTr("Playlist'ler") },
        Nav { target: "tracks"; icon.name: Util.icon("view-media-track"); text: qsTr("Tüm parçalar") },
        Nav { target: "radio"; icon.name: Util.icon("radio"); text: qsTr("Radyo") },
        Kirigami.Action { separator: true },
        Kirigami.Action {
            objectName: "together"
            icon.name: "system-users"
            text: qsTr("Birlikte dinle")
            visible: Sync.available
            onTriggered: drawer.w.openTogether()
        },
        Kirigami.Action { objectName: "settings"; icon.name: Util.icon("configure"); text: qsTr("Ayarlar"); onTriggered: drawer.w.openSettings("Genel") }
    ]
}
