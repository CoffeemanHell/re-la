// Artist tile: square picture (monochrome until hovered), name, album count. Click opens the artist page.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Item {
    id: card
    property var artist
    readonly property var w: ApplicationWindow.window
    function up(s) { return (s || "").toUpperCase() }
    width: Math.round((Player.options.gridSize || 172) * 150 / 172); height: width + 46 // scales with the album cards
    HoverHandler { cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: card.w.go("artist", { id: card.artist.id }) }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: card.w.menu("artist", card.artist) }

    ColumnLayout {
        anchors.fill: parent
        spacing: 4
        Cover { Layout.preferredWidth: card.width; Layout.preferredHeight: card.width; art: card.artist.coverArt || "" }
        Item { Layout.preferredHeight: 4 }
        Mono { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: card.up(card.artist.name); font.bold: true }
        Mono {
            Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
            visible: (card.artist.albumCount || 0) > 0 // search3 sends 0 for unknown
            text: card.artist.albumCount + " ALBÜM"
            color: App.theme.muted; font.pixelSize: 9
        }
        Item { Layout.fillHeight: true }
    }
}
