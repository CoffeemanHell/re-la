// One genre: its albums (getAlbumList2 byGenre, 60 at a time) and "Karışık çal" (50 random songs of it).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

GenreData {
    id: root
    scroller: grid

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        RowLayout {
            PageTitle { text: (root.args.name || "").toUpperCase(); Layout.fillWidth: true }
            Pill { text: qsTr("KARIŞIK ÇAL"); on: true; enabled: Sync.canControl; onClicked: root.shuffle() }
        }
        GridView {
            id: grid
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            cellWidth: (Player.options.gridSize || 172) + 24; cellHeight: (Player.options.gridSize || 172) + 54 + 24
            model: root.albums
            delegate: Item {
                required property var model
                width: grid.cellWidth; height: grid.cellHeight
                AlbumCard { album: parent.model }
            }
            ScrollBar.vertical: Scroll {}
            onContentYChanged: if (contentY + height > contentHeight - 2 * cellHeight) root.load()
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load() // tall window: fill it
        }
    }
    Empty { visible: !root.loading && !root.failed && root.albums.count === 0; text: qsTr("BU TÜRDE ALBÜM YOK") }
    Loading { loading: root.loading && root.albums.count === 0; failed: root.failed && root.albums.count === 0; onRetry: root.load() }
}
