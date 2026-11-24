// One song in a compact list (queue, similar, popular, search). Hover: heart and ⋮; in the queue also ↑ ↓ ×.
// Right click or ⋮ opens the song menu. list: the songs this row belongs to: double click plays the list from this row.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Item {
    id: row
    required property var song
    required property int index
    property bool inQueue: false
    property var list: null
    property int number: index + 1
    readonly property bool isCurrent: inQueue && index === Player.index
    readonly property var w: ApplicationWindow.window
    width: ListView.view ? ListView.view.width : parent ? parent.width : 0
    height: 44
    HoverHandler { id: hov }
    function openMenu() { w.menu("song", song, { queueIndex: inQueue ? index : -1 }) }
    TapHandler {
        enabled: Sync.canControl
        onDoubleTapped: row.inQueue ? Player.jump(row.index) : row.list ? Player.play(row.list, row.index) : Player.add([row.song], 0)
    }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: row.openMenu() }
    Rectangle { anchors.fill: parent; color: hov.hovered ? App.theme.surface : "transparent" }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
    RowLayout {
        anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 6
        spacing: 12
        Mono {
            text: row.isCurrent ? "●" : String(row.number).padStart(2, "0")
            color: row.isCurrent ? App.theme.accent : App.theme.muted
            Layout.preferredWidth: 22
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Mono { text: row.w ? row.w.up(row.song.title) : ""; Layout.fillWidth: true; font.bold: row.isCurrent; color: row.inQueue && row.index < Player.index ? App.theme.muted : App.theme.fg }
            Mono { text: row.w ? row.w.up(row.song.artist) : ""; color: App.theme.muted; font.pixelSize: 9; Layout.fillWidth: true }
        }
        Mono { text: (row.song.suffix || "").toUpperCase(); color: App.theme.muted; font.pixelSize: 9; visible: !hov.hovered }
        Mono { text: row.w ? row.w.fmt(row.song.duration) : ""; color: App.theme.muted; font.pixelSize: 10; visible: !hov.hovered }
        Row {
            visible: hov.hovered
            spacing: 4
            Heart { implicitHeight: 22; item: row.song }
            Pill { implicitHeight: 22; visible: row.inQueue; enabled: Sync.canControl && row.index > 0; text: "↑"; onClicked: Player.move(row.index, row.index - 1) }
            Pill { implicitHeight: 22; visible: row.inQueue; enabled: Sync.canControl && row.index < Player.queueLength - 1; text: "↓"; onClicked: Player.move(row.index, row.index + 1) }
            Pill { implicitHeight: 22; visible: row.inQueue; enabled: Sync.canControl; text: "×"; onClicked: Player.remove(row.index) }
            Pill { implicitHeight: 22; text: "⋮"; onClicked: row.openMenu() }

        }
    }
}
