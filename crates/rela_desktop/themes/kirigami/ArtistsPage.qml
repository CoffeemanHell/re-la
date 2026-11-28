// All artists (getArtists), grouped by first letter. Click opens the artist page.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import shared

ArtistsData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: Kirigami.Units.largeSpacing * 3
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
        visible: root.artists !== null && root.artists.length > 0
        PageTitle { text: qsTr("Sanatçılar") }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -Kirigami.Units.largeSpacing * 3; rightMargin: Kirigami.Units.largeSpacing * 3 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true
            model: root.artists || []
            section.property: "letter"
            section.delegate: Plain {
                required property string section
                text: section; topPadding: 14; bottomPadding: 4; leftPadding: 10
                font.weight: Font.Bold; font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.2
            }
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 52
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("artist", { id: row.modelData.id }) }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.w.menu("artist", row.modelData) }

                Rectangle { anchors.fill: parent; radius: Kirigami.Units.cornerRadius; color: hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
                    Cover { Layout.preferredWidth: 38; Layout.preferredHeight: 38; art: row.modelData.coverArt || ""; radius: 19 }
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name || ""; font.weight: Font.Medium }
                    Plain { text: qsTr("%n albüm", "", row.modelData.albumCount || 0); color: Kirigami.Theme.disabledTextColor }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.artists !== null && root.artists.length === 0; text: qsTr("Sanatçı yok") }
    Loading { loading: root.artists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
