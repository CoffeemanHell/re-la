// Home: four strips of albums from getAlbumList2. "Tümü" opens the Albums page with that order.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared

HomeData {
    id: root
    scroller: flick

    strips: [["recent", "Son dinlenenler"], ["newest", "Yeni eklenenler"],
             ["frequent", "Çok dinlenenler"], ["random", "Rastgele"]]

    Flickable {
        id: flick
        anchors.fill: parent
        visible: !root.empty
        contentHeight: col.implicitHeight + 48
        clip: true
        ScrollBar.vertical: ScrollBar {}
        ColumnLayout {
            id: col
            x: 24; y: 24; width: parent.width - 48
            spacing: 28
            Repeater {
                model: root.strips
                ColumnLayout {
                    id: strip
                    required property var modelData
                    readonly property var albums: root.lists[modelData[0]] || []
                    Layout.fillWidth: true
                    visible: albums.length > 0
                    spacing: 10
                    RowLayout {
                        Plain { text: strip.modelData[1]; font.pointSize: Qt.application.font.pointSize * 1.35; font.weight: Font.Bold; Layout.fillWidth: true }
                        // the strip doesn't take the wheel (the page scrolls); these move it a page at a time
                        ToolButton { icon.name: "go-previous"; enabled: list.contentX > 0; onClicked: list.page(-1) }
                        ToolButton { icon.name: "go-next"; enabled: list.contentX < list.contentWidth - list.width - 1; onClicked: list.page(1) }
                        Button { text: "Tümü"; flat: true; onClicked: root.w.go("albums", { type: strip.modelData[0] }) }
                    }
                    ListView {
                        id: list
                        Layout.fillWidth: true; Layout.preferredHeight: 240
                        orientation: ListView.Horizontal
                        interactive: false
                        spacing: 20
                        clip: true
                        leftMargin: 6; rightMargin: 6
                        model: strip.albums
                        delegate: AlbumCard { required property var modelData; album: modelData; y: 6 }
                        function page(dir) { contentX = Math.max(-leftMargin, Math.min(contentWidth - width + rightMargin, contentX + dir * (width - 100))) }
                        property bool ready: false // no animation while restoring
                        Behavior on contentX { enabled: list.ready; NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }
                        onContentXChanged: root.stripX[strip.modelData[0]] = contentX
                        Component.onCompleted: Qt.callLater(() => {
                            const x = root.saved ? root.saved.x[strip.modelData[0]] : undefined
                            if (x !== undefined) contentX = x
                            ready = true
                        })
                    }
                }
            }
        }
    }
    Loading { loading: root.pending > 0 && root.empty; failed: root.failed && root.empty; onRetry: root.load() }
    Empty { visible: root.pending === 0 && !root.failed && root.empty; text: "Kütüphane boş" }
}
