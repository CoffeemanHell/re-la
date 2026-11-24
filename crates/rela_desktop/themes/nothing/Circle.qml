// Round transport button.
import QtQuick
import QtQuick.Controls.Basic
import Rela

AbstractButton {
    id: circ
    property int size: 52
    property bool filled: false
    hoverEnabled: true
    implicitWidth: size; implicitHeight: size
    contentItem: Text {
        text: circ.text
        color: circ.filled ? App.theme.bg : (circ.enabled ? App.theme.fg : App.theme.muted)
        font.family: "Segoe UI Symbol"; font.pixelSize: circ.size * 0.32
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: width / 2
        color: circ.filled ? (circ.down ? App.theme.muted : App.theme.fg) : (circ.down ? App.theme.surface2 : "transparent")
        border.width: 1
        border.color: circ.filled || circ.hovered ? App.theme.fg : App.theme.line
    }
}
