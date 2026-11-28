// Artist tile: round picture, name, album count. Click opens the artist page.
// artist: a getArtists / search3 / getStarred2 artist entry.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Item {
    id: card
    property var artist
    readonly property var w: ApplicationWindow.window // null inside TapHandler (not an Item)
    width: Math.round((Player.options.gridSize || 172) * 150 / 172); height: width + 50 // scales with the album cards
    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: card.w.go("artist", { id: card.artist.id }) }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: card.w.menu("artist", card.artist) }

    Rectangle {
        anchors.fill: parent; anchors.margins: -6
        radius: Kirigami.Units.cornerRadius * 2
        color: hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent"
    }
    ColumnLayout {
        anchors.fill: parent
        spacing: 2
        Cover {
            Layout.preferredWidth: card.width; Layout.preferredHeight: card.width
            art: card.artist.coverArt || ""; radius: card.width / 2
        }
        Item { Layout.preferredHeight: 4 }
        Plain { Layout.fillWidth: true; elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter; text: card.artist.name || ""; font.weight: Font.DemiBold }
        Plain {
            Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
            visible: (card.artist.albumCount || 0) > 0 // search3 sends 0 for unknown
            text: qsTr("%n albüm", "", card.artist.albumCount)
            color: Kirigami.Theme.disabledTextColor
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 0.9
        }
        Item { Layout.fillHeight: true }
    }
}
