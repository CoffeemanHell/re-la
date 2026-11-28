// Genres (getGenres) by name, with album and song counts. Click opens the genre page.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import shared

GenresData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: Kirigami.Units.largeSpacing * 3
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
        visible: root.genres !== null && root.genres.length > 0
        PageTitle { text: qsTr("Türler") }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -Kirigami.Units.largeSpacing * 3; rightMargin: Kirigami.Units.largeSpacing * 3 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.genres || []
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 44
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("genre", { name: row.modelData.value }) }
                Rectangle { anchors.fill: parent; radius: Kirigami.Units.cornerRadius; color: hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.value; font.weight: Font.Medium }
                    Plain { text: qsTr("%n albüm", "", row.modelData.albumCount) + " · " + qsTr("%n şarkı", "", row.modelData.songCount); color: Kirigami.Theme.disabledTextColor }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.genres !== null && root.genres.length === 0; text: qsTr("Tür yok") }
    Loading { loading: root.genres === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
