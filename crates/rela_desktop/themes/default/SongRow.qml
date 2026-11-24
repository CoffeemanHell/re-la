// One song in a compact list (queue, similar, popular, search). Hover: heart and ⋮; in the queue also up / down / remove.
// Right click or ⋮ opens the song menu. list: the songs this row belongs to: double click plays the list from this row.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Item {
    id: row
    required property var song
    required property int index
    property bool inQueue: false
    property var list: null
    property int number: index + 1
    readonly property var w: ApplicationWindow.window
    readonly property bool isCurrent: inQueue && index === Player.index
    readonly property bool played: inQueue && index < Player.index
    width: ListView.view ? ListView.view.width - ListView.view.rightMargin : parent ? parent.width : 0
    height: 52
    HoverHandler { id: hov }
    function openMenu() { w.menu("song", song, { queueIndex: inQueue ? index : -1 }) }
    TapHandler {
        enabled: Sync.canControl
        onDoubleTapped: row.inQueue ? Player.jump(row.index) : row.list ? Player.play(row.list, row.index) : Player.add([row.song], 0)
    }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: row.openMenu() }
    Rectangle {
        anchors.fill: parent; radius: 4
        color: row.isCurrent ? Qt.rgba(palette.highlight.r, palette.highlight.g, palette.highlight.b, 0.22)
             : hov.hovered ? palette.alternateBase : "transparent"
    }
    RowLayout {
        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
        spacing: 12
        Item {
            Layout.preferredWidth: 24; Layout.fillHeight: true
            Plain { anchors.centerIn: parent; visible: !row.isCurrent; text: row.number; color: palette.placeholderText }
            Bars { anchors.centerIn: parent; visible: row.isCurrent; active: Player.playing }
        }
        Cover { Layout.preferredWidth: 38; Layout.preferredHeight: 38; art: row.song.coverArt || ""; radius: 4 }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Plain {
                Layout.fillWidth: true; elide: Text.ElideRight
                text: row.song.title || ""
                color: row.played ? palette.placeholderText : palette.windowText
                font.weight: row.isCurrent ? Font.Bold : Font.Medium // not the highlight color: the row is already tinted with it
            }
            Plain {
                Layout.fillWidth: true; elide: Text.ElideRight
                text: (row.song.artist || "") + (row.width > 640 && row.song.album ? "  ·  " + row.song.album : "")
                color: palette.placeholderText
                font.pointSize: Qt.application.font.pointSize * 0.9
            }
        }
        Plain { text: row.w ? row.w.tag(row.song) : ""; color: palette.placeholderText; font.pointSize: Qt.application.font.pointSize * 0.8; visible: !hov.hovered }
        Plain { text: row.w ? row.w.fmt(row.song.duration) : ""; color: palette.placeholderText; visible: !hov.hovered; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignRight }
        Row {
            visible: hov.hovered
            component Act: ToolButton {
                property string tip
                text: tip
                display: AbstractButton.IconOnly
                implicitWidth: 32; implicitHeight: 32
                ToolTip.visible: hovered; ToolTip.text: tip; ToolTip.delay: 500
            }
            Heart { item: row.song }

            Act { visible: row.inQueue; enabled: Sync.canControl && row.index > 0; icon.name: "go-up"; tip: "Yukarı"; onClicked: Player.move(row.index, row.index - 1) }
            Act { visible: row.inQueue; enabled: Sync.canControl && row.index < Player.queueLength - 1; icon.name: "go-down"; tip: "Aşağı"; onClicked: Player.move(row.index, row.index + 1) }
            Act { visible: row.inQueue; enabled: Sync.canControl; icon.name: "list-remove"; tip: "Kuyruktan çıkar"; onClicked: Player.remove(row.index) }
            Act { text: "⋮"; tip: "Diğer"; display: AbstractButton.TextOnly; onClicked: row.openMenu() }

        }
    }
}
