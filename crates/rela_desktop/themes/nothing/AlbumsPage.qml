// All albums as a grid, in the chosen order; 60 more are fetched near the end of the list.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

AlbumsData {
    id: root
    scroller: grid

    readonly property var orders: [{ t: "newest", l: "YENİ EKLENEN" }, { t: "recent", l: "SON DİNLENEN" },
                                   { t: "frequent", l: "ÇOK DİNLENEN" }, { t: "random", l: "RASTGELE" },
                                   { t: "alphabeticalByName", l: "ADA GÖRE" }, { t: "alphabeticalByArtist", l: "SANATÇIYA GÖRE" }]

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: root.orders
                Pill { required property var modelData; text: modelData.l; on: root.type === modelData.t; onClicked: root.order(modelData.t) }
            }
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
            onContentYChanged: if (contentY + height > contentHeight - 2 * cellHeight) root.load(false)
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load(false) // tall window: fill it
        }
    }
    Loading { loading: root.loading && root.albums.count === 0; failed: root.failed && root.albums.count === 0; onRetry: root.load(true) }
}
