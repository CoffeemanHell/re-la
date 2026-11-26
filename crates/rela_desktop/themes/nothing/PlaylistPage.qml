// One playlist (getPlaylist): play, shuffle; its owner can rename, delete and remove songs.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

PlaylistData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        visible: root.pl !== null
        RowLayout {
            spacing: 20
            Cover { Layout.preferredWidth: 140; Layout.preferredHeight: 140; art: root.pl ? root.pl.coverArt || "" : "" }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 4
                PageTitle { Layout.fillWidth: true; text: root.pl ? root.pl.name.toUpperCase() : "" }
                Mono {
                    color: App.theme.muted
                    text: root.pl ? qsTr("%n ŞARKI", "", root.songs.length) + " · " + root.w.fmt(root.pl.duration) + " · " + root.pl.owner.toUpperCase() : ""
                }
                RowLayout {
                    Layout.topMargin: 8; spacing: 8
                    Pill { text: qsTr("ÇAL"); on: true; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.songs, 0) }
                    Pill { text: qsTr("KARIŞTIR"); enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.shuffled(root.songs), 0) }
                    Pill { text: qsTr("YENİDEN ADLANDIR"); visible: root.editable; onClicked: root.rename() }
                    Pill { text: qsTr("SİL"); danger: true; visible: root.editable; onClicked: root.remove() }
                }
            }
        }
        TrackHeader { Layout.fillWidth: true }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.songs
            delegate: TrackRow {

                required property var modelData
                song: modelData; list: root.songs
                removable: root.editable && !root.busy
                onRemoveRequested: root.removeAt(index)
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.pl !== null && root.songs.length === 0; text: qsTr("BU PLAYLIST BOŞ") }
    Loading { loading: root.pl === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
