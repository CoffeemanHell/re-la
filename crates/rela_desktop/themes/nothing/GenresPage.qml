// Genres (getGenres) by name, with album and song counts. Click opens the genre page.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared

GenresData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        visible: root.genres !== null && root.genres.length > 0
        PageTitle { text: qsTr("TÜRLER") }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.genres || []
            delegate: ListRow {
                required property var modelData
                title: modelData.value; detail: qsTr("%n ALBÜM", "", modelData.albumCount) + " · " + qsTr("%n ŞARKI", "", modelData.songCount)
                onClicked: root.w.go("genre", { name: modelData.value })
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.genres !== null && root.genres.length === 0; text: qsTr("TÜR YOK") }
    Loading { loading: root.genres === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
