// Starred songs, albums and artists (getStarred2), one pill each.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

FavoritesData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        visible: root.res !== null && (root.songs.length || root.albums.length || root.artists.length)
        PageTitle { text: "FAVORİLER" }
        Row {
            spacing: 8
            Pill { text: "ŞARKILAR " + root.songs.length; on: root.tab === 0; onClicked: root.tab = 0 }
            Pill { text: "ALBÜMLER " + root.albums.length; on: root.tab === 1; onClicked: root.tab = 1 }
            Pill { text: "SANATÇILAR " + root.artists.length; on: root.tab === 2; onClicked: root.tab = 2 }
        }
        StackLayout {
            currentIndex: root.tab
            Layout.fillWidth: true; Layout.fillHeight: true
            ColumnLayout {
                spacing: 2
                TrackHeader { Layout.fillWidth: true }
                ListView {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true
                    model: root.songs
                    delegate: TrackRow { required property var modelData; song: modelData; list: root.songs }
                    ScrollBar.vertical: Scroll {}
                }
            }

            GridView {
                clip: true
                cellWidth: (Player.options.gridSize || 172) + 24; cellHeight: (Player.options.gridSize || 172) + 54 + 24
                model: root.albums
                delegate: Item { required property var modelData; width: GridView.view.cellWidth; height: GridView.view.cellHeight; AlbumCard { album: parent.modelData } }
                ScrollBar.vertical: Scroll {}
            }
            GridView {
                clip: true
                cellWidth: Math.round((Player.options.gridSize || 172) * 150 / 172) + 24; cellHeight: Math.round((Player.options.gridSize || 172) * 150 / 172) + 46 + 24
                model: root.artists
                delegate: Item { required property var modelData; width: GridView.view.cellWidth; height: GridView.view.cellHeight; ArtistCard { artist: parent.modelData } }
                ScrollBar.vertical: Scroll {}
            }
        }
    }
    Empty { visible: root.res !== null && !root.songs.length && !root.albums.length && !root.artists.length; text: "HENÜZ FAVORİ YOK" }
    Loading { loading: root.res === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
