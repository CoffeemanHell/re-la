// Outlined pill button; `on` fills it (white, or red when `danger`).
import QtQuick
import QtQuick.Controls.Basic
import Rela

AbstractButton {
    id: pill
    property bool on: false
    property bool danger: false
    hoverEnabled: true
    implicitHeight: 30
    implicitWidth: lbl.implicitWidth + 28
    contentItem: Text {
        id: lbl
        text: pill.text; textFormat: Text.PlainText
        color: pill.on ? (pill.danger ? App.theme.fg : App.theme.bg) : (pill.enabled ? App.theme.fg : App.theme.muted)
        font.family: App.theme.font; font.pixelSize: 11; font.letterSpacing: 1.5
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: height / 2
        color: pill.on ? (pill.danger ? App.theme.accent : App.theme.fg) : (pill.hovered ? App.theme.surface2 : "transparent")
        border.width: 1
        border.color: pill.on ? "transparent" : pill.hovered ? App.theme.fg : App.theme.line
    }
}
