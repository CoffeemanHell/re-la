// A setting with an on/off pill: label and hint on the left, the pill on the right.
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: sr
    property string label
    property string hint
    property bool checked: false
    property bool danger: false
    signal toggled(bool on)
    Layout.fillWidth: true
    spacing: 16
    ColumnLayout {
        Layout.fillWidth: true; spacing: 4
        Mono { text: sr.label; Layout.fillWidth: true }
        Hint { text: sr.hint }
    }
    Pill { text: sr.checked ? "AÇIK" : "KAPALI"; on: sr.checked; danger: sr.danger; onClicked: sr.toggled(!sr.checked) }
}
