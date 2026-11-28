// Around the theme: the engine loads this first. It makes the theme's window (App.load gives its
// Main.qml), swaps it when Player.theme changes (the new one is up before the old one goes), keeps the tray (when it is
// switched on), shows the
// window for a second launch or MPRIS Raise, and quits or hides to the tray when the window closes.
import QtQuick
import Qt.labs.platform as Platform
import Rela

QtObject {
    id: shell

    property Window win: null
    property WheelBoost wheel: WheelBoost {} // KDE button-scrolling, on the whole app

    function load() {
        const url = App.load(Player.theme)
        let w = make(url)
        if (!w) {
            Player.setStatus(qsTr("Tema yüklenemedi: %1").arg(url))
            // a missing module (Kirigami not installed) must not end the app: Default needs only Qt. The saved choice stays.
            if (!win && Player.theme !== "default")
                w = make(App.load("default"))
            if (!w) {
                if (!win)
                    Qt.exit(1)
                return
            }
        }
        const old = win
        win = w
        if (old)
            old.destroy() // the new window is up first
    }

    function make(url) {
        const c = Qt.createComponent(url)
        const w = c.status === Component.Ready ? c.createObject(null) : null
        if (!w) // a fixed start (the zip's smoke test looks for it), then why: errorString can be empty when createObject fails
            console.warn("Theme failed to load: " + url + ": " + (c.errorString() || "the window could not be created"))
        return w
    }

    function show() {
        if (!win)
            return
        win.show()
        win.raise()
        win.requestActivate()
    }

    readonly property string nowPlaying: Player.station.name ? Player.station.name // a station plays: the queue song waits
                                       : Player.current.title === undefined ? "re-la"
                                       : Player.current.artist ? Player.current.title + " · " + Player.current.artist : Player.current.title

    property Platform.SystemTrayIcon tray: Platform.SystemTrayIcon {
        visible: available && Player.options.closeToTray === true
        icon.source: "qrc:/re-la.svg"
        tooltip: shell.nowPlaying
        menu: Platform.Menu {
            Platform.MenuItem { text: shell.nowPlaying; enabled: false }
            Platform.MenuSeparator {}
            Platform.MenuItem { text: Player.playing ? qsTr("Duraklat") : qsTr("Oynat"); onTriggered: Player.toggle() }
            Platform.MenuItem { text: qsTr("Sonraki"); onTriggered: Player.next() }
            Platform.MenuItem { text: qsTr("Önceki"); onTriggered: Player.prev() }
            Platform.MenuSeparator {}
            Platform.MenuItem { text: qsTr("Pencereyi göster"); onTriggered: shell.show() }
            Platform.MenuItem { text: qsTr("Çıkış"); onTriggered: Qt.quit() }
        }
        onActivated: reason => {
            if (reason !== Platform.SystemTrayIcon.Trigger && reason !== Platform.SystemTrayIcon.DoubleClick)
                return
            if (shell.win && shell.win.visible && shell.win.active)
                shell.win.hide()
            else
                shell.show()
        }
    }

    property Connections player: Connections {
        target: Player
        function onThemeChanged() { Qt.callLater(shell.load) } // not inside a window's own setup (a theme may switch there)
        function onRaiseRequested() { shell.show() }
    }

    // closing the window can leave it playing in the tray
    property Connections window: Connections {
        target: shell.win
        function onClosing() {
            if (!shell.tray.available || !Player.options.closeToTray)
                Qt.quit()
            else if (Player.trayHint())
                shell.tray.showMessage("re-la", qsTr("Arka planda çalmaya devam ediyor. Çıkmak için tepsi simgesine sağ tıkla → Çıkış."))
        }
    }

    Component.onCompleted: {
        App.addCoverProvider()
        App.trayAvailable = tray.available
        load()
    }
}
