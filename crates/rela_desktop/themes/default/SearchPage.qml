// Search (search3): artists, albums and songs. The toolbar field updates args while this page is open.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared

SearchData {
    id: root
    scroller: flick

    component Header: Plain { font.pointSize: Qt.application.font.pointSize * 1.35; font.weight: Font.Bold }

    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: col.implicitHeight + 48
        clip: true
        ScrollBar.vertical: ScrollBar {}
        ColumnLayout {
            id: col
            x: 24; y: 24; width: parent.width - 48
            spacing: 14
            Header { text: qsTr("Sanatçılar"); visible: root.artists.length > 0 }
            Flow {
                Layout.fillWidth: true; spacing: 20; visible: root.artists.length > 0
                Repeater { model: root.artists; ArtistCard { required property var modelData; artist: modelData } }
            }
            Header { text: qsTr("Albümler"); visible: root.albums.length > 0; Layout.topMargin: 10 }
            Flow {
                Layout.fillWidth: true; spacing: 20; visible: root.albums.length > 0
                Repeater { model: root.albums; AlbumCard { required property var modelData; album: modelData } }
            }
            Header { text: qsTr("Şarkılar"); visible: root.songs.length > 0; Layout.topMargin: 10 }
            Column {
                Layout.fillWidth: true; spacing: 2
                Repeater { model: root.songs; SongRow { required property var modelData; song: modelData } }
            }
        }
    }
    Plain {
        anchors.centerIn: parent
        visible: root.res !== null && !root.artists.length && !root.albums.length && !root.songs.length
        text: qsTr("Sonuç yok")
        color: palette.placeholderText
    }
    Loading { loading: root.res === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
