// Shell.qml (around the theme) against the fake App: which theme window it makes, and what happens when one fails.
import QtQuick
import QtTest
import "../../qml"
import Rela

Item {
    Component { id: shellComp; Shell {} }

    TestCase {
        name: "shell"
        function init() { Player.reset(); Player.theme = "default"; App.broken = []; App.loaded = [] }

        function test_broken_theme_falls_back_to_default() {
            Player.theme = "broken"; App.broken = ["broken"]
            ignoreWarning(/^Theme failed to load: .*no-such-theme.*: \S/) // a fixed marker for the zip's smoke test, then why
            const s = createTemporaryObject(shellComp, null)
            verify(s.win !== null) // the app keeps running
            compare(App.loaded, ["broken", "default"])
            verify(String(s.win.title).length > 0)
            compare(Player.called("setStatus").length, 1) // the failure is told
            verify(Player.called("setStatus")[0][1].indexOf("no-such-theme") >= 0)
            compare(Player.theme, "broken") // the saved choice is not changed
            s.win.destroy()
        }
        function test_switch_theme_and_back() {
            const s = createTemporaryObject(shellComp, null)
            const first = s.win
            verify(first !== null)
            s.load(); compare(App.loaded, ["default", "default"])
            Player.theme = "nothing"; s.load()
            verify(s.win !== null && s.win !== first)
            Player.theme = "kirigami"; s.load()
            verify(s.win !== null && s.win.globalDrawer !== undefined) // the Kirigami window
            Player.theme = "default"; s.load()
            verify(s.win !== null)
            compare(App.loaded, ["default", "default", "nothing", "kirigami", "default"])
            s.win.destroy()
        }
    }
}
