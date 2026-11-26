// All albums as a grid, in the chosen order; 60 more are fetched near the end of the list.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

AlbumsData {
    id: root
    scroller: grid

    readonly property var orders: [{ t: "newest", l: qsTr("Yeni eklenen") }, { t: "recent", l: qsTr("Son dinlenen") },
                                   { t: "frequent", l: qsTr("Çok dinlenen") }, { t: "random", l: qsTr("Rastgele") },
                                   { t: "alphabeticalByName", l: qsTr("Ada göre") }, { t: "alphabeticalByArtist", l: qsTr("Sanatçıya göre") }]

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        RowLayout {
            Plain { text: qsTr("Albümler"); font.pointSize: Qt.application.font.pointSize * 1.6; font.weight: Font.Bold; Layout.fillWidth: true }
            ComboBox {
                model: root.orders
                textRole: "l"; valueRole: "t"
                Component.onCompleted: currentIndex = indexOfValue(root.type)
                onActivated: root.order(currentValue)
            }
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
            onContentYChanged: if (contentY + height > contentHeight - 2 * cellHeight) root.load(false)
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load(false) // tall window: fill it
        }
    }
    Loading { loading: root.loading && root.albums.count === 0; failed: root.failed && root.albums.count === 0; onRetry: root.load(true) }
}
