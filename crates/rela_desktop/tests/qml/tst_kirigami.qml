// The Kirigami theme's window against FakePlayer: sidebar, header, player bar, login. Its pages are tested below too as
// they are converted (until then the window loads Default's page files).
import QtQuick
import QtQuick.Controls
import QtTest
import QtCore
import org.kde.kirigami as Kirigami
import "../../themes/kirigami"
import Rela
import shared

Item {
    width: 1200; height: 800
    Component { id: mainWindow; Main {} }

    TestCase {
        name: "kirigami"
        function init() { Player.reset(); Subwave.reset(); Sync.canControl = true; Sync.available = false; Sync.users = []; Sync.listeners = []; Sync.following = "" }
        function hd(s) { return s }
        Settings { id: geo; category: "window" }
        function cleanup() { geo.setValue("w", 1200); geo.setValue("h", 760); geo.sync() } // resized windows save their size: the next file's windows start as usual
        function read(file) {
            const x = new XMLHttpRequest()
            x.open("GET", Qt.resolvedUrl("../../themes/kirigami/" + file), false)
            x.send()
            return x.responseText
        }
        function nav(m, target) { return m.globalDrawer.actions.filter(a => a.objectName === "nav_" + target)[0] }

        function test_sidebar_entries_go() {
            const m = createTemporaryObject(mainWindow, null)
            tryCompare(m, "page", "home")
            for (const t of ["nowplaying", "favorites", "albums", "artists", "genres", "playlists", "tracks", "radio", "home"]) {
                const a = nav(m, t)
                verify(a, t)
                a.trigger()
                compare(m.page, t)
                verify(a.checked, t + " checked")
            }
            verify(m.globalDrawer.actions.filter(a => a.objectName === "settings").length === 1)
            verify(m.globalDrawer.actions.filter(a => a.objectName === "together").length === 1)
        }
        function test_sidebar_collapses_when_narrow() {
            const m = createTemporaryObject(mainWindow, null)
            m.width = 900; tryCompare(m.globalDrawer, "collapsed", true)
            m.width = 1400; tryCompare(m.globalDrawer, "collapsed", false)
            verify(!m.globalDrawer.modal)
        }
        function test_bars_span_the_window_and_pages_sit_beside_the_sidebar() {
            const m = createTemporaryObject(mainWindow, null)
            for (const width of [900, 1400]) {
                m.width = width
                tryCompare(m.globalDrawer, "collapsed", width < 1100)
                wait(Kirigami.Units.longDuration + 100)
                for (const bar of [m.header, m.footer]) {
                    compare(bar.mapToItem(null, 0, 0).x, 0, "bar starts at the window's edge, width " + width)
                    compare(bar.width, m.width)
                }
                fuzzyCompare(m.pageStack.mapToItem(null, 0, 0).x, m.globalDrawer.width, 1) // pages start where the sidebar ends
                fuzzyCompare(m.pageStack.width, m.width - m.globalDrawer.width, 1)
            }
        }
        function test_sidebar_hidden_when_not_usable() {
            Player.online = false; Player.signedIn = false
            const m = createTemporaryObject(mainWindow, null)
            verify(!m.globalDrawer.visible || !m.globalDrawer.enabled)
        }
        function test_back_keeps_page_state() {
            const m = createTemporaryObject(mainWindow, null)
            tryCompare(m, "page", "home")
            m.go("albums"); m.go("artists")
            verify(m.history[m.at - 1].state !== undefined) // albums left with its state saved
            m.back()
            compare(m.page, "albums")
            compare(m.pageStack.currentItem.saved, m.history[m.at].state)
            compare(m.pageStack.depth, 1) // one page area
            m.forward(); compare(m.page, "artists")
        }
        function test_search_field_waits_300ms() {
            const m = createTemporaryObject(mainWindow, null)
            tryCompare(m, "page", "home")
            const q = named(m.header, "search")
            q.forceActiveFocus()
            tryVerify(() => q.activeFocus) // the test window becomes active first
            const type = c => { q.insert(q.length, c); q.textEdited() } // key events go to the test's own window
            type("a")
            verify(m.page !== "search") // not before 300 ms
            tryCompare(m, "page", "search", 1000)
            verify(q.activeFocus) // the new page does not take the field's focus: typing goes on
            type("b")
            tryVerify(() => m.history[m.at].args.query === "ab", 1000)
        }
        function test_header_back_forward_refresh() {
            const m = createTemporaryObject(mainWindow, null)
            tryCompare(m, "page", "home")
            m.go("albums")
            mouseClick(named(m.header, "back")); compare(m.page, "home")
            mouseClick(named(m.header, "forward")); compare(m.page, "albums")
            verify(named(m.header, "refresh").enabled)
        }
        function test_player_bar_fits_at_min_width() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            const m = createTemporaryObject(mainWindow, null)
            m.width = 900
            tryVerify(() => F.all(m.footer, it => it.objectName === "volume").length > 0)
            wait(50)
            for (const n of ["shuffle", "repeat", "volume"]) {
                const it = named(m.footer, n)
                const x = it.mapToItem(m.footer, 0, 0).x
                verify(x >= 0 && x + it.width <= m.footer.width, n + " inside the bar")
            }
        }
        function test_volume_has_ticks() {
            const m = createTemporaryObject(mainWindow, null)
            const v = named(m.footer, "volume")
            compare(v.stepSize, 5) // KDE's Slider draws a tick per stepSize
        }
        function test_login_error_is_negative() {
            Player.online = false; Player.signedIn = false; Player.status = "olmadı"
            const m = createTemporaryObject(mainWindow, null)
            const line = F.text(m.contentItem, "olmadı")
            verify(line && !Qt.colorEqual(line.color, line.Kirigami.Theme.negativeTextColor)) // the color set the line itself sees
            Player.loginFailed = true
            verify(Qt.colorEqual(line.color, line.Kirigami.Theme.negativeTextColor))
        }
        function test_login_connects() {
            Player.online = false; Player.signedIn = false
            const m = createTemporaryObject(mainWindow, null)
            named(m.contentItem, "loginUrl").text = "https://x"
            named(m.contentItem, "loginUser").text = "u"
            named(m.contentItem, "loginPass").text = "p"
            mouseClick(F.text(m.contentItem, "Bağlan"))
            compare(Player.called("login"), [["login", "https://x", "u", "p"]])
        }
        function test_no_hex_colors() {
            for (const f of ["Main.qml", "Sidebar.qml", "PlayerBar.qml", "LoginPage.qml"])
                verify(!/#[0-9a-fA-F]{3,8}\b/.test(read(f).replace(/\/\/.*$/gm, "")), f + " has a hex color")
        }
        function test_offline_opens_the_queue_not_the_login() {
            Player.online = false; Player.offline = true
            const m = createTemporaryObject(mainWindow, null)
            compare(m.page, "nowplaying")
            verify(m.footer.visible && !F.text(m.contentItem, hd("Vazgeç")))
            Player.offline = false; Player.online = true
            Player.status = "x"
            compare(m.page, "nowplaying", "stays there when the server is back")
        }
        function test_login_cancel_while_connecting() {
            Player.online = false; Player.signedIn = true; Player.connecting = true
            const m = createTemporaryObject(mainWindow, null)
            const b = F.text(m.contentItem, hd("Vazgeç"))
            verify(b !== null) // a hanging connect can still be abandoned
            mouseClick(b)
            compare(Player.called("logout").length, 1)
        }
        function test_skip_undo_notice() {
            const m = createTemporaryObject(mainWindow, null)
            verify(!F.text(m.footer, "Geri al"))
            const p = named(m.contentItem, "skipUndoPopup")
            verify(!p.visible)
            Player.skipUndo = { id: "1", title: "X" }
            tryVerify(() => p.visible)
            verify(F.text(p, "Atlama eksi sayılacak") !== null)
            const tc = named(m.footer, "transport").mapToItem(m.contentItem, named(m.footer, "transport").width / 2, 0).x
            fuzzyCompare(p.mapToItem(m.contentItem, p.width / 2, 0).x, tc, 1) // centred on the transport
            verify(p.mapToItem(m.contentItem, 0, p.height).y <= m.footer.mapToItem(m.contentItem, 0, 0).y) // above the bar
            mouseClick(F.text(p, "Geri al")) // a real click: KDE's ToolBar gives none to what sits outside it
            compare(Player.called("undoSkip").length, 1)
            verify(named(m.footer, "status").visible) // the status line no longer hides behind the notice
            const tc2 = m.ToolTip.toolTip.contentItem // KDE style: the Label is its first child
            compare((tc2.textFormat !== undefined ? tc2 : tc2.children[0]).textFormat, Text.PlainText) // song names in tips: never markup
        }
        function test_listener_toast_and_button() {
            Sync.available = true; Sync.users = [{ user: "A", following: "me" }]; Sync.listeners = ["A"]
            const m = createTemporaryObject(mainWindow, null)
            verify(F.text(m.globalDrawer, "1 kişi seni dinliyor") || F.text(m.header, "1 kişi seni dinliyor"))
            Sync.listenerJoined("A")
            compare(m.notice, "A seni dinliyor")
            Sync.listenerLeft("A")
            compare(m.notice, "A ayrıldı")
        }
        function test_footer_progress() {
            live({ duration: 271 })
            Subwave.elapsedMs = 42000
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.text(m.footer, "0:42") && F.text(m.footer, "4:31"))
            const bar = F.all(m.footer, it => it instanceof ProgressBar && it.visible)[0]
            verify(bar && Math.abs(bar.value / bar.to - 42 / 271) < 0.001)
        }
        function test_footer_progress_clamps_and_needs_duration() {
            live({ duration: 271 })
            Subwave.elapsedMs = 400000
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.all(m.footer, it => it.visible && it.text === "4:31").length === 2) // elapsed stops at the duration
            Subwave.now = Object.assign({}, Subwave.now, { duration: 0 })
            Subwave.elapsedMs = 42000
            tryVerify(() => F.text(m.footer, "0:42"))
            verify(!F.text(m.footer, "0:00")) // no total when the duration is unknown
        }
        function test_footer_progress_only_for_live_subwave() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            let m = createTemporaryObject(mainWindow, null)
            verify(!F.all(m.footer, it => it instanceof ProgressBar && it.visible).length) // a song has the seek slider
            Player.station = { id: "2", name: "Plain" }
            wait(50)
            verify(!F.all(m.footer, it => it instanceof ProgressBar && it.visible).length)
            verify(!F.all(m.footer, it => it.visible && typeof it.text === "string" && /^\d+:\d\d$/.test(it.text)).length)
        }
        function test_footer_title_not_cut_short() {
            Player.current = { id: "1", title: "Beef Rapp Long Title With Many Words Featuring Somebody", artist: "MF DOOM", album: "MM..FOOD" }
            const m = createTemporaryObject(mainWindow, null)
            const t = F.all(m.footer, it => it instanceof Label && it.text.startsWith("Beef Rapp"))[0]
            m.width = 1000; waitForRendering(t)
            verify(t.width > 250, "wide window: title cut at " + t.width + "px")
            m.width = 640; waitForRendering(t)
            verify(t.width > 140, "narrow window: title cut at " + t.width + "px")
        }
        function test_footer_short_title_not_elided() {
            const m = createTemporaryObject(mainWindow, null)
            m.width = 1000
            for (const title of ["Hoe Cakes", "Beef Rapp", "Rhymes Like Dimes", "Kon Karne", "Deep Fried Frenz", "Poo-Putt Platter", "One Beer", "Kookies", "Gas Drawls", "Fig Leaf Bi-Carbonate"]) {
                Player.current = { id: "1", title: title, artist: "MF DOOM", album: "MM..FOOD" }
                const t = F.all(m.footer, it => it instanceof Label && it.text === title)[0]
                waitForRendering(t)
                verify(!t.truncated, title + ": " + t.width + " < " + t.implicitWidth)
            }
        }
        function test_footer_has_no_format() {

            Player.current = { id: "s1", title: "T", suffix: "flac", bitDepth: 16, samplingRate: 44100 }
            Player.bitrate = 900
            const m = createTemporaryObject(mainWindow, null)
            verify(!F.all(m.footer, it => it.visible && typeof it.text === "string" && it.text.indexOf("kbps") >= 0).length)
            verify(F.text(m.footer, "DJ"))
        }
        function test_volume_wheel_step() {
            Player.volume = 100
            const m = createTemporaryObject(mainWindow, null)
            const vol = F.all(m.footer, it => it instanceof Slider && it.to === 100 && it.value === 100)[0]
            waitForRendering(vol)
            mouseWheel(vol, vol.width / 2, vol.height / 2, 0, -120) // one notch down
            compare(Player.volume, 95)
        }
        function test_repeat_button_cycles() {
            const m = createTemporaryObject(mainWindow, null)
            const b = F.all(m.footer, it => it.objectName === "repeat")[0]
            mouseClick(b); compare(Player.repeat, 1)
            mouseClick(b); compare(Player.repeat, 2)
            mouseClick(b); compare(Player.repeat, 0)
            mouseClick(F.all(m.footer, it => it.objectName === "shuffle")[0]); compare(Player.shuffle, true)
        }
        function test_transport_is_centered() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.all(m.footer, it => it.objectName === "shuffle").length > 0)
            const row = F.all(m.footer, it => it.objectName === "shuffle")[0].parent
            for (const width of [900, 1400]) { // narrow and wide window
                m.width = width
                tryVerify(() => Math.abs(row.mapToItem(m.footer, row.width / 2, 0).x - m.footer.width / 2) <= 1,
                          5000, "transport off center at width " + width)
            }
        }
        function test_station_footer() {
            Player.station = { id: "1", name: "SUB/WAVE" }
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.text(m.footer, hd("SUB/WAVE")) && F.text(m.footer, hd("● Canlı")))
            verify(!F.all(m.footer, it => it.visible && it instanceof Slider && it.to !== 100).length) // seek bar hidden (the volume slider stays)
        }
        function volumeX(m) { return F.all(m.footer, it => it.objectName === "volume")[0].mapToItem(null, 0, 0).x }
        function test_subwave_footer() {
            live()
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.text(m.footer, "The News") && F.text(m.footer, "Dead Confederate") && F.text(m.footer, hd("● Canlı")))
        }
        function footerHeart(m) { return F.all(m.footer, it => it.objectName === "swHeart" && it.visible)[0] }
        function test_live_heart_is_in_the_footer() {
            live()
            Subwave.likeEnabled = true
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => footerHeart(m))
            mouseClick(footerHeart(m))
            compare(Subwave.called("like").length, 1)
            Subwave.liked = true
            tryCompare(footerHeart(m), "text", "♥")
            Subwave.liked = false
            Subwave.likeBlockedMs = 3000
            tryCompare(footerHeart(m), "enabled", false)
            const withHeart = volumeX(m)
            Subwave.likeEnabled = false
            tryVerify(() => !footerHeart(m))
            compare(volumeX(m), withHeart) // the volume slider does not move when the heart comes and goes
        }
        function test_heart_only_for_live_subwave_in_the_footer() {
            Subwave.likeEnabled = true
            Player.station = { id: "2", name: "Plain" } // a plain station: subwave is inactive
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.all(m.footer, it => it.objectName === "volume").length > 0)
            wait(50)
            verify(!footerHeart(m))
        }
        function test_volume_x_is_steady() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.all(m.footer, it => it.objectName === "volume").length > 0)
            for (const width of [900, 1400]) { // narrow and wide window
                m.width = width
                tryCompare(m.globalDrawer, "collapsed", width < 1100)
                wait(Kirigami.Units.longDuration + 100) // the sidebar's width settles (the bar sits beside it)
                Player.station = {}; Subwave.active = false
                wait(50)
                const song = volumeX(m)
                live()
                wait(50)
                const sw = volumeX(m)
                Subwave.active = false
                Player.station = { id: "2", name: "Plain" }
                wait(50)
                const plain = volumeX(m)
                compare([sw, plain], [song, song], "width " + width) // [SUB/WAVE, plain station] against a song
            }
        }
        function test_transport_buttons_fit_their_column_in_a_narrow_window() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            const m = createTemporaryObject(mainWindow, null)
            m.width = 900 // the smallest the window goes
            tryVerify(() => F.all(m.footer, it => it.objectName === "transport").length > 0)
            const bar = () => F.all(m.footer, it => it.objectName === "transport")[0]
            // the test style draws no icons; with a desktop icon theme each button is about this wide
            for (const b of F.all(bar(), it => it instanceof ToolButton)) b.implicitWidth = 44
            const fits = what => verify(bar().width <= bar().parent.width, what + ": buttons " + bar().width + " px in a column of " + bar().parent.width)
            wait(50)
            fits("song")
            live()
            wait(50)
            fits("SUB/WAVE")
            Subwave.active = false
            Player.station = { id: "2", name: "Plain" }
            wait(50)
            fits("plain station")
        }
        function live(extra) { // a SUB/WAVE station is playing
            Player.station = { id: "1", name: "SUB/WAVE" }
            Subwave.active = true
            Subwave.now = Object.assign({ id: "abc", title: "The News", artist: "Dead Confederate", album: "W", dj: "wave", show: "", listeners: 1, upcoming: [] }, extra || {})
            Subwave.coverUrl = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
        }
        function named(root, n) { return F.all(root, it => it.objectName === n)[0] }
    }
}
