// Home: four strips of albums from getAlbumList2. "TÜMÜ" opens the Albums page with that order.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

HomeData {
    id: root
    scroller: flick

    strips: [["recent", "SON DİNLENENLER"], ["newest", "YENİ EKLENENLER"],
             ["frequent", "ÇOK DİNLENENLER"], ["random", "RASTGELE"]]

    Flickable {
        id: flick
        anchors.fill: parent
        visible: !root.empty
        contentHeight: col.implicitHeight + 72
        clip: true
        ScrollBar.vertical: Scroll {}
        ColumnLayout {
            id: col
            x: 36; y: 36; width: parent.width - 72
            spacing: 36
            Repeater {
                model: root.strips
                ColumnLayout {
                    id: strip
                    required property var modelData
                    required property int index
                    readonly property var albums: root.lists[modelData[0]] || []
                    Layout.fillWidth: true
                    visible: albums.length > 0
                    spacing: 14
                    RowLayout {
                        spacing: 8
                        Mono { text: String(strip.index + 1).padStart(2, "0") + " · " + strip.modelData[1]; color: App.theme.muted; Layout.fillWidth: true }
                        // the strip doesn't take the wheel (the page scrolls); these move it a page at a time
                        Pill { text: "←"; enabled: list.contentX > 0; onClicked: list.page(-1) }
                        Pill { text: "→"; enabled: list.contentX < list.contentWidth - list.width - 1; onClicked: list.page(1) }
                        Pill { text: "TÜMÜ"; onClicked: root.w.go("albums", { type: strip.modelData[0] }) }
                    }
                    ListView {
                        id: list
                        Layout.fillWidth: true; Layout.preferredHeight: 226
                        orientation: ListView.Horizontal
                        interactive: false
                        spacing: 20
                        clip: true
                        model: strip.albums
                        delegate: AlbumCard { required property var modelData; album: modelData }
                        function page(dir) { contentX = Math.max(0, Math.min(contentWidth - width, contentX + dir * (width - 100))) }
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
    Empty { visible: root.pending === 0 && !root.failed && root.empty; text: "KÜTÜPHANE BOŞ" }
}
