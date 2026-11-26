// Takes the next keys pressed as an action's shortcut. A theme puts it beside its button and calls start(); Esc gives up.
import QtQuick
import Rela

Item {
    id: cap
    property string action // a key of Shortcuts.all
    readonly property bool on: activeFocus
    property string problem: "" // why the last keys were not taken
    function start() { problem = ""; forceActiveFocus() }
    Keys.onShortcutOverride: e => e.accepted = true // while on, no shortcut fires
    Keys.onPressed: e => {
        e.accepted = true
        if (e.key === Qt.Key_Escape) { problem = ""; focus = false; return }
        const keys = Shortcuts.written(e.key, e.modifiers)
        if (!keys) return
        const other = Shortcuts.clash(action, keys)
        if (other) { problem = qsTr("%1 zaten kullanılıyor: %2").arg(keys).arg(other); return }
        problem = ""
        const a = Shortcuts.byKey(action)
        Player.setOption(action, keys === (a.text || a.std) ? "" : keys)
        focus = false
    }
}
