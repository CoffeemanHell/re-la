// A setting with a switch: label and hint on the left, the switch on the right.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

RowLayout {
    id: sr
    property string label
    property string hint
    property alias checked: sw.checked
    signal toggled(bool on)
    Layout.fillWidth: true
    spacing: Kirigami.Units.largeSpacing
    ColumnLayout {
        Layout.fillWidth: true; spacing: Kirigami.Units.smallSpacing / 2
        Plain { text: sr.label }
        Hint { text: sr.hint }
    }
    Switch { id: sw; Accessible.name: sr.label; onToggled: sr.toggled(checked) }
}
