// Starred songs, albums and artists (getStarred2), one tab each.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

FavoritesData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        visible: root.res !== null && (root.songs.length || root.albums.length || root.artists.length)
        PageTitle { text: qsTr("Favoriler") }
        TabBar {
            id: tabs
            currentIndex: root.tab; onCurrentIndexChanged: root.tab = currentIndex
            TabButton { text: qsTr("Şarkılar (%1)").arg(root.songs.length); width: implicitWidth }
            TabButton { text: qsTr("Albümler (%1)").arg(root.albums.length); width: implicitWidth }
            TabButton { text: qsTr("Sanatçılar (%1)").arg(root.artists.length); width: implicitWidth }
        }
        StackLayout {
            currentIndex: tabs.currentIndex
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.rightMargin: -24 // scroll bars at the window edge; the song list keeps the gutter
            ColumnLayout {
                spacing: 2
                TrackHeader { Layout.fillWidth: true; Layout.rightMargin: 24 }
                ListView {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true; spacing: 2; rightMargin: 24
                    model: root.songs
                    delegate: TrackRow { required property var modelData; song: modelData; list: root.songs }
                    ScrollBar.vertical: Scroll {}
                }
            }

            GridView {
                clip: true
                cellWidth: (Player.options.gridSize || 172) + 24; cellHeight: (Player.options.gridSize || 172) + 60 + 24
                model: root.albums
                delegate: Item { required property var modelData; width: GridView.view.cellWidth; height: GridView.view.cellHeight; AlbumCard { x: 6; y: 6; album: parent.modelData } }
                ScrollBar.vertical: ScrollBar {}
            }
            GridView {
                clip: true
                cellWidth: Math.round((Player.options.gridSize || 172) * 150 / 172) + 24; cellHeight: Math.round((Player.options.gridSize || 172) * 150 / 172) + 50 + 24
                model: root.artists
                delegate: Item { required property var modelData; width: GridView.view.cellWidth; height: GridView.view.cellHeight; ArtistCard { x: 6; y: 6; artist: parent.modelData } }
                ScrollBar.vertical: ScrollBar {}
            }
        }
    }
    Empty { visible: root.res !== null && !root.songs.length && !root.albums.length && !root.artists.length; text: qsTr("Henüz favori yok") }
    Loading { loading: root.res === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
