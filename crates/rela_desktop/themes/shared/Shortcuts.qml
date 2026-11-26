// The keyboard shortcuts: what each action's keys are. The user's own are in Player.options (empty: the standard ones).
// key: the option's name; std: the standard keys; text: how they are written when they are a StandardKey.
pragma Singleton
import QtQuick
import Rela

QtObject {
    id: sc
    readonly property var all: [
        { key: "keyToggle", title: qsTr("Çal / duraklat"), std: "Space" },
        { key: "keyNext", title: qsTr("Sonraki"), std: "Ctrl+Right" },
        { key: "keyPrev", title: qsTr("Önceki"), std: "Ctrl+Left" },
        { key: "keySearch", title: qsTr("Ara"), std: "Ctrl+F" },
        { key: "keyBack", title: qsTr("Geri"), std: StandardKey.Back, text: "Alt+Left" },
        { key: "keyForward", title: qsTr("İleri"), std: StandardKey.Forward, text: "Alt+Right" },
    ]
    function byKey(k) { return all.find(a => a.key === k) }
    // For Shortcut.sequences.
    function sequences(k) { return [Player.options[k] || byKey(k).std] }
    // As written: "Ctrl+F".
    function label(k) { return Player.options[k] || byKey(k).text || byKey(k).std }
    // The other action that has these keys, "" when they are free.
    function clash(k, keys) {
        const other = all.find(a => a.key !== k && label(a.key) === keys)
        return other ? other.title : ""
    }
    // Names QKeySequence reads; letters, digits and signs are their own.
    readonly property var names: {
        const n = {}
        const add = (key, name) => n[key] = name
        add(Qt.Key_Space, "Space"); add(Qt.Key_Left, "Left"); add(Qt.Key_Right, "Right"); add(Qt.Key_Up, "Up"); add(Qt.Key_Down, "Down")
        add(Qt.Key_Return, "Return"); add(Qt.Key_Enter, "Enter"); add(Qt.Key_Tab, "Tab"); add(Qt.Key_Backspace, "Backspace")
        add(Qt.Key_Delete, "Del"); add(Qt.Key_Insert, "Ins"); add(Qt.Key_Home, "Home"); add(Qt.Key_End, "End")
        add(Qt.Key_PageUp, "PgUp"); add(Qt.Key_PageDown, "PgDown"); add(Qt.Key_Back, "Back"); add(Qt.Key_Forward, "Forward")
        for (let i = 0; i < 24; i++) add(Qt.Key_F1 + i, "F" + (i + 1))
        return n
    }
    // The pressed keys as text, "" while only Ctrl, Alt ... are down or the key has no name here.
    function written(key, modifiers) {
        const printable = key > 0x20 && key < 0x180 && key !== 0x7f
        const name = names[key] || (printable ? String.fromCharCode(key) : "")
        if (!name) return ""
        const letter = /^[A-Z0-9]$/.test(name)
        return (modifiers & Qt.ControlModifier ? "Ctrl+" : "") + (modifiers & Qt.AltModifier ? "Alt+" : "")
             + (modifiers & Qt.ShiftModifier && (letter || names[key]) ? "Shift+" : "") // a sign is already the shifted one
             + (modifiers & Qt.MetaModifier ? "Meta+" : "") + name
    }
}
