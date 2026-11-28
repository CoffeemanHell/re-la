// Vertical scroll bar that keeps rows out from under it: a list with no rightMargin of its own gets one as wide
// as the bar, and rows size themselves to ListView.view.width - rightMargin.
import QtQuick
import QtQuick.Controls

ScrollBar {
    onParentChanged: { const f = parent as Flickable; if (f && f.rightMargin === 0) f.rightMargin = Qt.binding(() => width) }
}
