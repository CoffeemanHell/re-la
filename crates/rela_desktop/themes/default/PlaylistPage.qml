// One playlist (getPlaylist): play, shuffle; its owner can rename, delete and remove songs.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

PlaylistData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        visible: root.pl !== null
        RowLayout {
            spacing: 20
            Cover { Layout.preferredWidth: 140; Layout.preferredHeight: 140; art: root.pl ? root.pl.coverArt || "" : ""; radius: 8 }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 4
                PageTitle { Layout.fillWidth: true; elide: Text.ElideRight; text: root.pl ? root.pl.name : "" }
                Plain {
                    color: palette.placeholderText
                    text: root.pl ? root.songs.length + " şarkı · " + root.w.fmt(root.pl.duration) + " · " + root.pl.owner : ""
                }
                RowLayout {
                    Layout.topMargin: 8; spacing: 8
                    Button { text: "Çal"; icon.name: "media-playback-start"; highlighted: true; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.songs, 0) }
                    Button { text: "Karıştır"; icon.name: "media-playlist-shuffle"; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.shuffled(root.songs), 0) }
                    Button { text: "Yeniden adlandır"; icon.name: Util.icon("edit-rename"); visible: root.editable; onClicked: root.rename() }
                    Button { text: "Sil"; icon.name: "edit-delete"; visible: root.editable; onClicked: root.remove() }
                }
            }
        }
        TrackHeader { Layout.fillWidth: true }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
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
    Empty { visible: root.pl !== null && root.songs.length === 0; text: "Bu playlist boş" }
    Loading { loading: root.pl === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
