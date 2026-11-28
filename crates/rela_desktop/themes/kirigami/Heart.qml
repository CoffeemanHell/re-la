// Favorite toggle (star / unstar). Flips at once; a failed write flips back and says so.
// item: a song, album or artist object (its `starred` is the starting state); kind picks the id parameter.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import shared

ToolButton {
    id: h
    property var item
    property string kind: "song"
    readonly property var w: ApplicationWindow.window
    readonly property bool on: fav.on
    readonly property Favorite fav: Favorite { item: h.item; kind: h.kind; w: h.w; owner: h }
    text: on ? "♥" : "♡"
    font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.3
    contentItem: Plain {
        text: h.text; font: h.font
        color: h.on ? h.Kirigami.Theme.highlightColor : h.Kirigami.Theme.textColor
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
    }
    implicitWidth: 32; implicitHeight: 32
    ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay; ToolTip.text: on ? qsTr("Favorilerden çıkar") : qsTr("Favorilere ekle")
    onClicked: fav.toggle()
}
