// All artists (getArtists), grouped by first letter. Click opens the artist page.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

ArtistsData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        visible: root.artists !== null && root.artists.length > 0
        PageTitle { text: qsTr("SANATÇILAR") }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.artists || []
            section.property: "letter"
            section.delegate: Mono { required property string section; text: section; color: App.theme.accent; topPadding: 16; bottomPadding: 6; font.pixelSize: 13 }
            delegate: ListRow {
                required property var modelData
                showArt: true; art: modelData.coverArt || ""
                title: modelData.name || ""; detail: qsTr("%n ALBÜM", "", modelData.albumCount || 0)
                onClicked: root.w.go("artist", { id: modelData.id })
                onMenuRequested: root.w.menu("artist", modelData)

            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.artists !== null && root.artists.length === 0; text: qsTr("SANATÇI YOK") }
    Loading { loading: root.artists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
