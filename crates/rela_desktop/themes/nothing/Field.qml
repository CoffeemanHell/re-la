// Text field: mono type over a single bottom line.
import QtQuick
import QtQuick.Controls.Basic
import Rela

TextField {
    id: fld
    color: App.theme.fg
    placeholderTextColor: App.theme.muted
    selectionColor: App.theme.accent
    font.family: App.theme.font; font.pixelSize: 12; font.letterSpacing: 1.2
    leftPadding: 0
    background: Item {
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: fld.activeFocus ? App.theme.fg : App.theme.line }
    }
}
