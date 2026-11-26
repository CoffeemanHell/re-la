// Album tile: square cover (monochrome until hovered), name, "artist · year". Click opens the album page; hovering
// shows a play circle on the cover (plays without opening the page), a heart and ⋮; right click opens the album menu.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Item {
    id: card
    property var album
    readonly property var w: ApplicationWindow.window
    function up(s) { return (s || "").toUpperCase() }
    width: Player.options.gridSize || 172; height: width + 54 // card size: Ayarlar → Genel → Izgara boyutu
    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: card.w.go("album", { id: card.album.id }) }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: card.w.menu("album", card.album) }
    function play() {
        Player.get(card, "getAlbum", { id: card.album.id }, r => r ? Player.play(r.album.song || [], 0) : card.w.notify(qsTr("İşlem başarısız")))
    }
    ColumnLayout {
        anchors.fill: parent
        spacing: 4
        Cover {
            Layout.preferredWidth: card.width; Layout.preferredHeight: card.width; art: card.album.coverArt || ""
            Circle {
                objectName: "cardPlay"
                anchors.centerIn: parent
                size: 48; filled: true; text: "▶"
                visible: hov.hovered && Sync.canControl
                onClicked: card.play()
            }
            Rectangle {
                anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 6
                width: tools.width + 8; height: tools.height + 4; radius: height / 2
                color: App.theme.bg; border.width: 1; border.color: App.theme.line
                visible: hov.hovered
                Row {
                    id: tools
                    anchors.centerIn: parent
                    spacing: 2
                    Heart { implicitHeight: 22; item: card.album; kind: "album" }
                    Pill { implicitHeight: 22; text: "⋮"; onClicked: card.w.menu("album", card.album) }
                }
            }
        }

        Item { Layout.preferredHeight: 4 }
        Mono { Layout.fillWidth: true; text: card.up(card.album.name); font.bold: true }
        Mono {
            Layout.fillWidth: true
            text: card.up(card.album.artist) + (card.album.year ? " · " + card.album.year : "")
            color: App.theme.muted; font.pixelSize: 9
        }
        Item { Layout.fillHeight: true }
    }
}
