// Playlists (getPlaylists): name, song count, length, owner. "Yeni playlist" makes an empty one.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

PlaylistsData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        RowLayout {
            PageTitle { text: "Playlist'ler"; Layout.fillWidth: true }
            Button { text: "Yeni playlist"; icon.name: "list-add"; onClicked: root.create() }
        }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.lists || []
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 60
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("playlist", { id: row.modelData.id }) }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.w.menu("playlist", row.modelData) }

                Rectangle { anchors.fill: parent; radius: 4; color: hov.hovered ? palette.alternateBase : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                    spacing: 12
                    Cover { Layout.preferredWidth: 44; Layout.preferredHeight: 44; art: row.modelData.coverArt || ""; radius: 4 }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 1
                        Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name; font.weight: Font.Medium }
                        Plain {
                            Layout.fillWidth: true; elide: Text.ElideRight; color: palette.placeholderText
                            text: row.modelData.songCount + " şarkı · " + root.w.fmt(row.modelData.duration) + (row.modelData.owner !== Player.user ? " · " + row.modelData.owner : "")
                        }
                    }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.lists !== null && root.lists.length === 0; text: "Playlist yok" }
    Loading { loading: root.lists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
