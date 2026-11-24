// One genre: its albums (getAlbumList2 byGenre, 60 at a time) and "Karışık çal" (50 random songs of it).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

GenreData {
    id: root
    scroller: grid

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        RowLayout {
            PageTitle { text: root.args.name; Layout.fillWidth: true; elide: Text.ElideRight }
            Button { text: "Karışık çal"; icon.name: "media-playlist-shuffle"; enabled: Sync.canControl; onClicked: root.shuffle() }
        }
        GridView {
            id: grid
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.rightMargin: -24 // scroll bar at the window edge
            clip: true
            cellWidth: (Player.options.gridSize || 172) + 24; cellHeight: (Player.options.gridSize || 172) + 60 + 24
            model: root.albums
            delegate: Item {
                required property var model
                width: grid.cellWidth; height: grid.cellHeight
                AlbumCard { x: 6; y: 6; album: parent.model }
            }
            ScrollBar.vertical: ScrollBar {}
            onContentYChanged: if (contentY + height > contentHeight - 2 * cellHeight) root.load()
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load() // tall window: fill it
        }
    }
    Empty { visible: !root.loading && !root.failed && root.albums.count === 0; text: "Bu türde albüm yok" }
    Loading { loading: root.loading && root.albums.count === 0; failed: root.failed && root.albums.count === 0; onRetry: root.load() }
}
