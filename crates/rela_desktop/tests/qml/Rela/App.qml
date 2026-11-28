// Stands in for App in QML tests: `theme` is the nothing theme's tokens over the defaults, read from the theme folders
// (run with QML_XHR_ALLOW_FILE_READ=1, see docs/development.md).
pragma Singleton
import QtQuick

QtObject {
    function read(file) {
        const x = new XMLHttpRequest()
        x.open("GET", Qt.resolvedUrl("../../../themes/" + file), false)
        x.send()
        return JSON.parse(x.responseText)
    }
    property var theme: Object.assign(read("default/theme.json"), read("nothing/theme.json"))
    property var themeNames: ["default", "nothing"]
    property var themeTitles: ({ default: "Default", nothing: "Nothing" })
    property bool trayAvailable: false
    property string buildDate: "01.02.2026"
    // Shell.qml: a theme's Main.qml by folder; "broken" (and anything in `broken`) gives a file that does not exist
    property var broken: []
    property var loaded: []
    function load(name) {
        loaded = loaded.concat([name])
        return Qt.resolvedUrl("../../../themes/" + (broken.indexOf(name) >= 0 ? "no-such-theme" : name) + "/Main.qml")
    }
    function addCoverProvider() {}

    // Main.qml's Settings (window size) need an app identity; the test command keeps them in a scratch config dir
    Component.onCompleted: {
        Qt.application.organization = "re-la-test"
        Qt.application.name = "qml_test"
    }
}
