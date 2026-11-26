// Album tile: cover, name, "artist · year". Click opens the album page; hovering shows a play button on the
// cover (plays without opening the page), a heart and ⋮; right click opens the album menu.
// album: a getAlbumList2 / getArtist entry (or a ListModel row with the same roles).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Item {
    id: card
    property var album
    readonly property var w: ApplicationWindow.window // null inside TapHandler (not an Item)
    width: Player.options.gridSize || 172; height: width + 60 // card size: Ayarlar → Genel → Izgara boyutu
    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: card.w.go("album", { id: card.album.id }) }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: card.w.menu("album", card.album) }
    function play() {
        Player.get(card, "getAlbum", { id: card.album.id }, r => r ? Player.play(r.album.song || [], 0) : card.w.notify(qsTr("İşlem başarısız")))
    }
    Rectangle {
        anchors.fill: parent; anchors.margins: -6
        radius: 8
        color: hov.hovered ? palette.alternateBase : "transparent"
    }
    ColumnLayout {
        anchors.fill: parent
        spacing: 2
        Cover {
            Layout.preferredWidth: card.width; Layout.preferredHeight: card.width
            art: card.album.coverArt || ""; radius: 6
            RoundButton {
                objectName: "cardPlay"
                anchors.centerIn: parent
                width: 52; height: 52
                visible: hov.hovered && Sync.canControl
                highlighted: true
                icon.name: "media-playback-start"; icon.width: 24; icon.height: 24
                ToolTip.visible: hovered; ToolTip.text: qsTr("Çal"); ToolTip.delay: 500
                onClicked: card.play()
            }
            Rectangle {
                anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 6
                width: tools.width + 4; height: tools.height + 4; radius: height / 2
                color: Qt.rgba(palette.window.r, palette.window.g, palette.window.b, 0.8)
                visible: hov.hovered
                Row {
                    id: tools
                    anchors.centerIn: parent
                    Heart { item: card.album; kind: "album" }
                    ToolButton { text: "⋮"; implicitWidth: 28; implicitHeight: 32; onClicked: card.w.menu("album", card.album) }
                }
            }
        }

        Item { Layout.preferredHeight: 4 }
        Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: card.album.name || ""; font.weight: Font.DemiBold }
        Plain {
            Layout.fillWidth: true; elide: Text.ElideRight
            text: (card.album.artist || "") + (card.album.year ? " · " + card.album.year : "")
            color: palette.placeholderText
            font.pointSize: Qt.application.font.pointSize * 0.9
        }
        Item { Layout.fillHeight: true }
    }
}
