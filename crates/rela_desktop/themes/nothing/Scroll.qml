// Thin scroll bar.
import QtQuick
import QtQuick.Controls.Basic
import Rela

ScrollBar {
    width: 3
    contentItem: Rectangle { radius: 1; color: App.theme.muted }
    background: Item {}
}
