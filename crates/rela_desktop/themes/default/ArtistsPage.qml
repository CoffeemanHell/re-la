// All artists (getArtists), grouped by first letter. Click opens the artist page.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared

ArtistsData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        visible: root.artists !== null && root.artists.length > 0
        PageTitle { text: "Sanatçılar" }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true
            model: root.artists || []
            section.property: "letter"
            section.delegate: Plain {
                required property string section
                text: section; topPadding: 14; bottomPadding: 4; leftPadding: 10
                font.weight: Font.Bold; font.pointSize: Qt.application.font.pointSize * 1.2
            }
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 52
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("artist", { id: row.modelData.id }) }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.w.menu("artist", row.modelData) }

                Rectangle { anchors.fill: parent; radius: 4; color: hov.hovered ? palette.alternateBase : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                    spacing: 12
                    Cover { Layout.preferredWidth: 38; Layout.preferredHeight: 38; art: row.modelData.coverArt || ""; radius: 19 }
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name || ""; font.weight: Font.Medium }
                    Plain { text: (row.modelData.albumCount || 0) + " albüm"; color: palette.placeholderText }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.artists !== null && root.artists.length === 0; text: "Sanatçı yok" }
    Loading { loading: root.artists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
