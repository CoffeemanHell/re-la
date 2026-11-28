// The Kirigami theme's own window tests: sidebar, header, page area, player bar geometry, login. Everything Default's
// tests check runs on this theme too, in tst_kirigami_pages.qml.
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
        function cleanupTestCase() { wait(0); geo.setValue("w", 1200); geo.setValue("h", 760); geo.sync() } // resized windows save their size (also when destroyed): the next file's windows start as usual
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
        // the files this theme has drawn itself so far (the rest still come from Default)
        readonly property var converted: ["HomePage.qml", "AlbumsPage.qml", "ArtistsPage.qml", "GenresPage.qml", "PlaylistsPage.qml",
                                          "AlbumCard.qml", "ArtistCard.qml", "Cover.qml", "Empty.qml", "Loading.qml",
                                          "AlbumPage.qml", "ArtistPage.qml", "GenrePage.qml", "PlaylistPage.qml", "TracksPage.qml",
                                          "FavoritesPage.qml", "SearchPage.qml", "TrackRow.qml", "TrackHeader.qml", "TrackColumns.qml",
                                          "SongRow.qml", "Stars.qml", "Link.qml", "Heart.qml", "PageTitle.qml",
                                          "NowPlayingPage.qml", "Lyrics.qml", "RadioPage.qml", "RequestForm.qml", "Wave.qml", "Bars.qml",
                                          "Hint.qml", "Share.qml", "SettingsDialog.qml", "SwitchRow.qml", "TogetherDialog.qml",
                                          "DjStateDialog.qml", "InfoDialog.qml", "ItemMenu.qml", "PlaylistMenu.qml"]
        function test_no_hex_colors() {
            for (const f of ["Main.qml", "Sidebar.qml", "PlayerBar.qml", "LoginPage.qml"].concat(converted))
                verify(!/#[0-9a-fA-F]{3,8}\b/.test(read(f).replace(/\/\/.*$/gm, "")), f + " has a hex color")
        }
        function test_converted_files_are_kirigami() {
            for (const f of converted) {
                const t = read(f)
                verify(t.length > 0, f + " exists")
                verify(t.indexOf("../default") < 0, f + " uses no Default file")
                if (f !== "ItemMenu.qml" && f !== "PlaylistMenu.qml") // menus are QtQuick Controls menus, as in Kirigami apps
                    verify(/import org\.kde\.kirigami/.test(t), f + " uses Kirigami")
            }
        }
        function test_no_default_files_loaded() {
            for (const f of ["Main.qml", "Sidebar.qml", "PlayerBar.qml", "LoginPage.qml"].concat(converted))
                verify(read(f).indexOf("../default") < 0, f + " loads a Default file")
        }
        function test_pages_load_from_this_theme() {
            const m = createTemporaryObject(mainWindow, null)
            tryCompare(m, "page", "home")
            for (const [p, a] of [["home"], ["albums"], ["artists"], ["genres"], ["playlists"], ["album", { id: "1" }], ["artist", { id: "1" }],
                                  ["genre", { name: "Rap" }], ["playlist", { id: "1" }], ["tracks"], ["favorites"], ["search", { query: "x" }],
                                  ["nowplaying"], ["radio"]]) {
                m.go(p, a)
                const url = String(m.lastPageUrl)
                verify(url.indexOf("/themes/kirigami/") >= 0, p + ": " + url)
            }
        }
        function live(extra) { // a SUB/WAVE station is playing
            Player.station = { id: "1", name: "SUB/WAVE" }
            Subwave.active = true
            Subwave.now = Object.assign({ id: "abc", title: "The News", artist: "Dead Confederate", album: "W", dj: "wave", show: "", listeners: 1, upcoming: [] }, extra || {})
        }
        function volumeX(m) { return F.all(m.footer, it => it.objectName === "volume")[0].mapToItem(null, 0, 0).x }
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
        function named(root, n) { return F.all(root, it => it.objectName === n)[0] }
    }
}
