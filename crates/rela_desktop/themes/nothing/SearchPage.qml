// Search (search3): artists, albums and songs. The top bar field updates args while this page is open.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

SearchData {
    id: root
    scroller: flick

    component Header: Mono { color: App.theme.muted }

    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: col.implicitHeight + 72
        clip: true
        ScrollBar.vertical: Scroll {}
        ColumnLayout {
            id: col
            x: 36; y: 36; width: parent.width - 72
            spacing: 14
            Header { text: qsTr("SANATÇILAR"); visible: root.artists.length > 0 }
            Flow {
                Layout.fillWidth: true; spacing: 20; visible: root.artists.length > 0
                Repeater { model: root.artists; ArtistCard { required property var modelData; artist: modelData } }
            }
            Header { text: qsTr("ALBÜMLER"); visible: root.albums.length > 0; Layout.topMargin: 10 }
            Flow {
                Layout.fillWidth: true; spacing: 20; visible: root.albums.length > 0
                Repeater { model: root.albums; AlbumCard { required property var modelData; album: modelData } }
            }
            Header { text: qsTr("ŞARKILAR"); visible: root.songs.length > 0; Layout.topMargin: 10 }
            Column {
                Layout.fillWidth: true
                Repeater { model: root.songs; SongRow { required property var modelData; song: modelData } }
            }
        }
    }
    Mono {
        anchors.centerIn: parent
        visible: root.res !== null && !root.artists.length && !root.albums.length && !root.songs.length
        text: qsTr("SONUÇ YOK"); color: App.theme.muted
    }
    Loading { loading: root.res === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
