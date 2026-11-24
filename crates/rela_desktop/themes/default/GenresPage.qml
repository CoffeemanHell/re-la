// Genres (getGenres) by name, with album and song counts. Click opens the genre page.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared

GenresData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        visible: root.genres !== null && root.genres.length > 0
        PageTitle { text: "Türler" }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.genres || []
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 44
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("genre", { name: row.modelData.value }) }
                Rectangle { anchors.fill: parent; radius: 4; color: hov.hovered ? palette.alternateBase : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.value; font.weight: Font.Medium }
                    Plain { text: row.modelData.albumCount + " albüm · " + row.modelData.songCount + " şarkı"; color: palette.placeholderText }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.genres !== null && root.genres.length === 0; text: "Tür yok" }
    Loading { loading: root.genres === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
