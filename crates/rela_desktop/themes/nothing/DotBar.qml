// Row of dots used for progress, volume and setting values; the last lit dot is red.
// levels (0..1, any length): each dot grows into a bar of that level, for the waveform.
pragma ComponentBehavior: Bound
import QtQuick
import Rela

Item {
    id: bar
    property real value: 0
    property int dots: 64
    property bool live: false
    property var levels: []
    signal picked(real v)
    implicitHeight: levels.length ? 22 : 16
    Row {
        anchors.verticalCenter: parent.verticalCenter
        spacing: Math.max(1, (bar.width - bar.dots * 6) / Math.max(1, bar.dots - 1))
        Repeater {
            model: bar.dots
            Rectangle {
                required property int index
                readonly property int lit: Math.round(bar.value * bar.dots)
                readonly property real level: { // the loudest peak under this dot, so short hits are not skipped
                    const n = bar.levels.length, from = Math.floor(index * n / bar.dots), to = Math.max(Math.floor((index + 1) * n / bar.dots), from + 1)
                    let v = 0
                    for (let k = from; k < Math.min(to, n); k++) v = Math.max(v, bar.levels[k])
                    return v
                }
                width: 6; height: bar.levels.length ? Math.max(6, level * bar.height) : 6; radius: 3
                anchors.verticalCenter: parent.verticalCenter
                color: index < lit ? (index === lit - 1 ? App.theme.accent : App.theme.fg) : App.theme.line
                Behavior on color { ColorAnimation { duration: 120 } }
            }
        }
    }
    MouseArea {
        anchors.fill: parent
        enabled: bar.enabled
        cursorShape: bar.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        function at(x) { return Math.max(0, Math.min(1, x / width)) }
        onClicked: m => bar.picked(at(m.x))
        onPositionChanged: m => { if (pressed && bar.live) bar.picked(at(m.x)) }
    }
}
