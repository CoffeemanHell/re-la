// Playlists (getPlaylists): name, song count, length, owner. "Yeni playlist" makes an empty one.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

PlaylistsData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: Kirigami.Units.largeSpacing * 3
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
        RowLayout {
            PageTitle { text: qsTr("Playlist'ler"); Layout.fillWidth: true }
            Button { text: qsTr("Yeni playlist"); icon.name: "list-add"; onClicked: root.create() }
        }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -Kirigami.Units.largeSpacing * 3; rightMargin: Kirigami.Units.largeSpacing * 3 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.lists || []
            delegate: Item {
                id: row
                required property var modelData
                width: ListView.view.width - ListView.view.rightMargin; height: 60
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.w.go("playlist", { id: row.modelData.id }) }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.w.menu("playlist", row.modelData) }

                Rectangle { anchors.fill: parent; radius: Kirigami.Units.cornerRadius; color: hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent" }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
                    Cover { Layout.preferredWidth: 44; Layout.preferredHeight: 44; art: row.modelData.coverArt || ""; radius: Kirigami.Units.cornerRadius }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 1
                        Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name; font.weight: Font.Medium }
                        Plain {
                            Layout.fillWidth: true; elide: Text.ElideRight; color: Kirigami.Theme.disabledTextColor
                            text: row.modelData.owner !== Player.user ? qsTr("%n şarkı · %1 · %2", "", row.modelData.songCount).arg(root.w.fmt(row.modelData.duration)).arg(row.modelData.owner)
                                                                : qsTr("%n şarkı · %1", "", row.modelData.songCount).arg(root.w.fmt(row.modelData.duration))
                        }
                    }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.lists !== null && root.lists.length === 0; text: qsTr("Playlist yok") }
    Loading { loading: root.lists === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
