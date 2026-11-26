// Playlists (getPlaylists): name, song count, length, owner. "Yeni playlist" makes an empty one.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

PlaylistsData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        RowLayout {
            PageTitle { text: qsTr("PLAYLIST'LER"); Layout.fillWidth: true }
            Pill { text: qsTr("YENİ PLAYLIST"); onClicked: root.create() }
        }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.lists || []
            delegate: ListRow {
                required property var modelData
                showArt: true; art: modelData.coverArt || ""
                title: modelData.name
                detail: qsTr("%n ŞARKI", "", modelData.songCount) + " · " + root.w.fmt(modelData.duration) + (modelData.owner !== Player.user ? " · " + modelData.owner.toUpperCase() : "")
                onClicked: root.w.go("playlist", { id: modelData.id })
                onMenuRequested: root.w.menu("playlist", modelData)

            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.lists !== null && root.lists.length === 0; text: qsTr("PLAYLIST YOK") }
    Loading { loading: root.lists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
