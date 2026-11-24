// Three bouncing bars: marks the playing song in a list.
pragma ComponentBehavior: Bound
import QtQuick

Row {
    id: bars
    property bool active: false
    height: 14; spacing: 2
    Repeater {
        model: 3
        Rectangle {
            required property int index
            id: bar
            width: 3; radius: 1.5; color: palette.highlight
            y: bars.height - height
            height: 5
            SequentialAnimation on height {
                running: bars.active; loops: Animation.Infinite
                NumberAnimation { to: 14; duration: 260 + bar.index * 110; easing.type: Easing.InOutSine }
                NumberAnimation { to: 4; duration: 300 + bar.index * 80; easing.type: Easing.InOutSine }
            }
        }
    }
}
