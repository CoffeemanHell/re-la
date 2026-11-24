// A setting with a switch: label and hint on the left, the switch on the right.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    id: sr
    property string label
    property string hint
    property alias checked: sw.checked
    signal toggled(bool on)
    Layout.fillWidth: true
    ColumnLayout {
        Layout.fillWidth: true; spacing: 2
        Plain { text: sr.label }
        Hint { text: sr.hint }
    }
    Switch { id: sw; onToggled: sr.toggled(checked) }
}
