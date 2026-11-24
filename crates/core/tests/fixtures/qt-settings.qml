// qml qt-settings.qml -- write <file>   or   qml qt-settings.qml -- read <file>
// write: QSettings (through QtCore Settings) writes the values the Rust tests expect back.
// read: prints what QSettings reads from the file, one `key=<JSON of toString()>` per line on stdout-ish (console.warn).
import QtQuick
import QtCore

QtObject {
    readonly property var values: ({
        "url": "https://music.example.com",
        "user": " ali, veli = x; \"q\" ",
        "token": "abc123",
        "theme": "Şık tema ü",
        "volume": 0.75,
        "crossfade": 5,
        "shuffle": true,
        "weird": "@leading at",
        "path": "C:\\back\\slash",
        "tabs": "a\tb\nc",
        "options/relayUrl": "ws://h:4534/",
        "options/autoContinue": false,
        "sync/hidden": true
    })
    readonly property var args: Qt.application.arguments
    property Settings s: Settings { location: "file://" + args[args.length - 1] }
    Component.onCompleted: {
        const mode = args[args.length - 2]
        const keys = Object.keys(values).sort()
        if (mode === "write") {
            for (const k of keys) s.setValue(k, values[k])
            s.sync()
        } else {
            for (const k of keys) console.warn("KV " + k + "=" + JSON.stringify(String(s.value(k))))
        }
        Qt.exit(0)
    }
}
