// Pages of the default theme against FakePlayer: clicks open pages, buttons call the Player.
import QtQuick
import QtQuick.Controls
import QtTest
import "../../themes/default"
import Rela
import shared

Item {
    ApplicationWindow {
        id: win
        width: 1200; height: 800; visible: true
        property var opened: null
        function go(page, args) { opened = [page, args] }
        function fmt(t) { return String(t) }
        function tag(s) { return "" }
        function up(s) { return (s || "").toUpperCase() }
        function notify(t) { notice = t }
        property string notice: ""
        property var asked: null    // [title, text, callback] of the last askName
        property var confirmed: null
        function askName(title, text, cb) { asked = [title, text, cb] }
        function confirm(text, cb) { confirmed = [text, cb] }
        property var menued: null      // [kind, item, opts] of the last menu()
        function menu(k, it, o) { menued = [k, it, o || {}] }
        property var infoed: null
        function info(id) { infoed = id }
        property var downloaded: null
        function download(it, k) { downloaded = [it, k] }

        Item { id: host; anchors.fill: parent }
    }
    Component { id: albumCard; AlbumCard { album: ({ id: "al1", name: "X" }) } }
    Component { id: searchPage; SearchPage {} }
    Component { id: artistCard; ArtistCard { artist: ({ id: "ar1", name: "A", albumCount: 2 }) } }
    Component { id: genresPage; GenresPage {} }
    Component { id: albumsPage; AlbumsPage {} }
    Component { id: albumPage; AlbumPage {} }
    Component { id: trackHeader; TrackHeader {} }
    Component { id: radioPage; RadioPage {} }
    Component { id: favoritesPage; FavoritesPage {} }
    Component { id: songRow; SongRow { index: 0; song: ({ id: "s1", title: "Song" }) } }
    Component { id: playlistPage; PlaylistPage {} }
    Component { id: mainWindow; Main {} }
    Component { id: stars; Stars {} }
    Component { id: heart; Heart {} }
    Component { id: share; Share {} }
    Component { id: lyrics; Lyrics {} }
    Component { id: itemMenu; ItemMenu { w: win } }
    Component { id: trackRow; TrackRow { index: 0; song: ({ id: "s1", title: "T", album: "A", year: 1979, bitRate: 900 }); list: [] } }
    Component { id: nowPlaying; NowPlayingPage {} }
    Component { id: infoDialog; InfoDialog {} }
    Component { id: scrolled; ListView {
        model: 50; ScrollBar.vertical: Scroll {}
        delegate: SongRow { required property int modelData; index: modelData; song: ({ id: "s" + modelData, title: "Song" }) }
    } }

    TestCase {
        name: "default"; when: win.visible
        property var page: null
        function init() { Player.reset(); Subwave.reset(); Sync.canControl = true; Sync.available = false; Sync.users = []; Sync.listeners = []; Sync.following = ""; win.opened = null; win.notice = ""; win.asked = null; win.confirmed = null; win.menued = null; win.infoed = null; win.downloaded = null }
        function mi(m, t) { for (let i = 0; i < m.count; i++) if (m.itemAt(i) && m.itemAt(i).text === t) return m.itemAt(i); return null }
        function cleanup() { if (page) page.destroy(); page = null }
        function make(comp, props) { page = createTemporaryObject(comp, host, props); page.width = host.width; page.height = host.height; if (page instanceof Item) waitForRendering(page); return page } // popups (menus, dialogs) are not Items
        function hd(s) { return "default" === "nothing" ? s.toUpperCase() : s }

        function test_rows_stay_clear_of_the_scroll_bar() {
            const l = createTemporaryObject(scrolled, host, { width: 400, height: 300 })
            waitForRendering(l)
            const bar = l.ScrollBar.vertical
            verify(bar.width > 0)
            compare(l.itemAtIndex(0).width, l.width - bar.width)
        }

        function test_album_card_opens_album() {
            const c = make(albumCard, {})
            c.width = 172; c.height = 232; waitForRendering(c) // make() stretches pages to the window
            mouseClick(F.text(c, hd("X"))) // the title: the cover holds the play button
            compare(win.opened, ["album", { id: "al1" }])
        }
        function test_search_groups() {
            Player.replies = { search3: { searchResult3: { artist: [{ id: "ar1", name: "A" }], album: [{ id: "al1", name: "B" }], song: [{ id: "s1", title: "C" }] } } }
            make(searchPage, { args: { query: "x" } })
            tryVerify(() => F.text(page, hd("Sanatçılar")) && F.text(page, hd("Albümler")) && F.text(page, hd("Şarkılar")))
            const c = Player.called("get", "search3")
            compare(c.length, 1)
            compare(c[0][2], { query: "x", artistCount: 6, albumCount: 12, songCount: 30 })
        }
        function test_search_no_results_only_after_reply() {
            Player.replies = { search3: { searchResult3: {} } }
            Player.hold = true
            make(searchPage, { args: { query: "zz" } })
            verify(!F.text(page, hd("Sonuç yok"))) // the request is still running
            Player.release()
            tryVerify(() => F.text(page, hd("Sonuç yok")))
        }
        function test_artist_card_opens_artist() {
            mouseClick(make(artistCard, {}))
            compare(win.opened, ["artist", { id: "ar1" }])
        }
        function test_genre_row_opens_genre() {
            Player.replies = { getGenres: { genres: { genre: [{ value: "Rock", albumCount: 2, songCount: 9 }] } } }
            make(genresPage, { args: {} })
            tryVerify(() => F.text(page, hd("Rock")))
            mouseClick(F.text(page, hd("Rock")))
            compare(win.opened, ["genre", { name: "Rock" }])
        }
        function test_radio_click_plays_station() {
            Player.replies = { getInternetRadioStations: { internetRadioStations: { internetRadioStation: [{ id: "1", name: "Wave", streamUrl: "x" }] } } }
            make(radioPage, { args: {} })
            tryVerify(() => F.text(page, hd("Wave")))
            mouseClick(F.text(page, hd("Wave")))
            compare(Player.called("playStation"), [["playStation", "1"]])
        }
        function live(extra) { // a SUB/WAVE station is playing
            Player.station = { id: "1", name: "SUB/WAVE" }
            Subwave.active = true
            Subwave.now = Object.assign({ id: "abc", title: "The News", artist: "Dead Confederate", album: "W", dj: "wave", show: "", listeners: 1, upcoming: [] }, extra || {})
            Subwave.coverUrl = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
        }
        function test_subwave_footer() {
            live()
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.text(m.footer, "The News") && F.text(m.footer, "Dead Confederate") && F.text(m.footer, hd("● Canlı")))
        }
        function test_subwave_nowplaying() {
            live({ show: "Gece" })
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "The News") && F.text(page, "Dead Confederate") && F.text(page, "DJ wave · Gece · 1 dinleyici"))
            verify(F.all(page, it => it instanceof Image && it.source.toString() === Subwave.coverUrl).length > 0)
            verify(!F.text(page, "Bağlantı yok"))
        }
        function test_subwave_offline_hint() {
            live()
            Subwave.online = false
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "Bağlantı yok"))
        }
        function test_plain_station_has_no_subwave_line() {
            Player.station = { id: "1", name: "Plain" }
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "Plain"))
            verify(!F.all(page, it => it.visible && typeof it.text === "string" && it.text.startsWith("DJ ")).length)
        }
        function test_radio_probes_stream_origin_without_home_page() {
            Player.replies = { getInternetRadioStations: { internetRadioStations: { internetRadioStation: [
                { id: "1", name: "Flac", streamUrl: "x", originUrl: "https://radio.example.com" },
                { id: "2", name: "Nowhere", streamUrl: "y", originUrl: "" }] } } }
            make(radioPage, { args: {} })
            tryVerify(() => F.text(page, "Flac"))
            compare(Subwave.called("probe").map(c => c[1]), ["https://radio.example.com"])
            Subwave.peeked = { "https://radio.example.com": { title: "T", artist: "A", dj: "wave" } }
            tryVerify(() => F.text(page, "T — A · wave"))
        }
        function test_radio_subtitle() {
            Player.replies = { getInternetRadioStations: { internetRadioStations: { internetRadioStation: [
                { id: "1", name: "Wave", streamUrl: "x", homePageUrl: "https://radio.example.com" },
                { id: "2", name: "Plain", streamUrl: "y", homePageUrl: "https://plain.example.com" }] } } }
            make(radioPage, { args: {} })
            tryVerify(() => F.text(page, "Wave"))
            compare(Subwave.called("probe").map(c => c[1]), ["https://radio.example.com", "https://plain.example.com"])
            Subwave.peeked = { "https://radio.example.com": { title: "T", artist: "A", dj: "wave" } }
            tryVerify(() => F.text(page, "T — A · wave"))
            verify(F.text(page, "https://plain.example.com"))
        }
        function liveLyrics(state, lyr) {
            Subwave.now = { id: "abc" }
            Subwave.lyrics = lyr || {}
            Subwave.lyricsState = state
        }
        function test_live_lyrics_follow_clock() {
            liveLyrics("ok", { synced: true, offset: 0, lines: [{ start: 1000, value: "a" }, { start: 5000, value: "b" }] })
            Subwave.elapsedMs = 5200
            const l = make(lyrics, { live: true })
            tryCompare(l, "line", 1)
            Subwave.elapsedMs = 500
            tryCompare(l, "line", -1)
            compare(Player.called("get", "getLyricsBySongId").length, 0) // the Navidrome lyrics are not asked for
        }
        function test_navidrome_word_cues() {
            Player.replies = { getLyricsBySongId: { lyricsList: { structuredLyrics: [
                { kind: "translation", synced: true, line: [{ start: 0, value: "One two" }] },
                { kind: "main", synced: true, offset: 0, line: [{ start: 1000, value: "Bir iki" }, { start: 3000, value: "üç" }],
                  cueLine: [{ index: 0, start: 1000, value: "Bir iki", cue: [{ start: 1000, value: "Bir" }, { start: 1400, value: "iki" }] }] }] } } }
            Player.position = 1.5
            const l = make(lyrics, { song: { id: "s1" } })
            tryCompare(l, "line", 0)
            compare(l.lines[0].words.map(w => w.value), ["Bir ", "iki"])
            compare(l.word, 1)
            verify(!l.lines[1].words)
        }
        function test_navidrome_syllable_cues() { // the OpenSubsonic example: one cue per syllable, spaces as cues
            Player.replies = { getLyricsBySongId: { lyricsList: { structuredLyrics: [{ kind: "main", synced: true,
                line: [{ start: 2747, value: "눈을 뜬 순간" }],
                cueLine: [{ index: 0, start: 2747, value: "눈을 뜬 순간", cue: [
                    { start: 2747, value: "눈" }, { start: 3018, value: "을" }, { start: 3179, value: " " },
                    { start: 3582, value: "뜬" }, { start: 4100, value: " " }, { start: 4500, value: "순" }, { start: 5200, value: "간" }] }] }] } } }
            Player.position = 3.1
            const l = make(lyrics, { song: { id: "s1" } })
            tryCompare(l, "word", 1)
            compare(l.lines[0].words.map(w => w.value).join("|"), "눈|을| |뜬| |순|간")
            verify(l.sung(l.lines[0], 1, "#fff", "#000").indexOf(">눈을</font><font color='#000'> 뜬 순간<") > 0)
        }
        function test_cues_follow_cue_line_text() { // byte offsets and cues belong to cueLine.value, which may differ from line.value
            Player.replies = { getLyricsBySongId: { lyricsList: { structuredLyrics: [{ synced: true,
                line: [{ start: 0, value: "beautiful day" }],
                cueLine: [{ index: 0, start: 0, value: "beau-ti-ful day", cue: [
                    { start: 0, value: "beau-" }, { start: 200, value: "ti-" }, { start: 400, value: "ful" }, { start: 800, value: "day" }] }] }] } } }
            Player.position = 0.3
            const l = make(lyrics, { song: { id: "s1" } })
            tryCompare(l, "word", 1)
            compare(l.lines[0].value, "beau-ti-ful day")
        }
        function test_word_timed_lyrics() {
            liveLyrics("ok", { synced: true, offset: 0, lines: [{ start: 1000, value: "<00:01.00>Bir <00:01.50>iki & <00:02.25>üç" }, { start: 5000, value: "düz" }] })
            Subwave.elapsedMs = 1600
            const l = make(lyrics, { live: true })
            tryCompare(l, "line", 0)
            compare(l.lines[0].value, "Bir iki & üç")
            compare(l.lines[0].words.map(w => w.start), [1000, 1500, 2250])
            compare(l.word, 1)
            verify(l.sung(l.lines[0], 1, "#fff", "#000").indexOf(">Bir iki &amp; </font><font color='#000'>üç<") > 0)
            tryVerify(() => F.all(l, it => it.visible && it.textFormat === Text.StyledText && /iki &amp; <\/font>.*üç/.test(it.text)).length === 1) // drawn by the theme
            Subwave.elapsedMs = 5200
            tryCompare(l, "word", -1) // a plain line has no words
            verify(!l.lines[1].words)
        }
        function test_live_lyrics_states() {
            liveLyrics("auth")
            const l = make(lyrics, { live: true })
            tryVerify(() => F.text(l, "Parola gerekli"))
            Subwave.lyricsState = "off"
            tryVerify(() => F.text(l, "Sözler kapalı"))
            Subwave.lyricsState = "none"
            tryVerify(() => F.text(l, "Söz bulunamadı"))
            Subwave.lyricsState = "retry"
            tryCompare(l, "loading", true)
            verify(!F.text(l, "Söz bulunamadı"))
        }
        function test_live_lyrics_unsynced_and_tap() {
            liveLyrics("ok", { synced: false, offset: 0, lines: [{ start: 0, value: "a" }, { start: 0, value: "b" }] })
            const l = make(lyrics, { live: true })
            tryVerify(() => F.text(l, "a") && F.text(l, "b"))
            compare(l.line, -1)
            tryCompare(F.text(l, "a"), "opacity", 1)
            tryCompare(F.text(l, "b"), "opacity", 1)
            liveLyrics("ok", { synced: true, offset: 0, lines: [{ start: 0, value: "a" }, { start: 10, value: "b" }] })
            tryVerify(() => l.synced)
            mouseClick(F.text(l, "b"))
            compare(Player.called("seek").length, 0) // a live stream has no seek
        }
        function test_nowplaying_lyrics_tab_is_live() {
            live()
            liveLyrics("auth")
            make(nowPlaying, { args: {} })
            mouseClick(tab("Sözler"))
            tryVerify(() => F.text(page, "Parola gerekli"))
        }
        function visibleTabs() { return F.all(page, it => it instanceof TabButton && it.visible).map(b => b.text) }
        function tab(name) { return F.all(page, it => it instanceof TabButton && it.visible && it.text === name)[0] }
        function swHeart() { return F.all(page, it => it.objectName === "swHeart" && it.visible)[0] }
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
        function test_nowplaying_page_has_no_heart_in_radio_mode() {
            live()
            Subwave.likeEnabled = true
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "The News"))
            verify(!swHeart())
        }
        function test_live_tabs() {
            live()
            make(nowPlaying, { args: {} })
            tryVerify(() => visibleTabs().length === 2)
            compare(visibleTabs(), ["Sözler", "İstek"])
        }
        function xOf(it) { return it.mapToItem(null, 0, 0).x }
        function test_nowplaying_texts_start_at_the_cover_edge() {
            live({ show: "Gece" })
            make(nowPlaying, { args: {} })
            const cover = F.all(page, it => it.objectName === "cover")[0]
            for (const t of ["The News", "Dead Confederate", "W", "DJ wave · Gece · 1 dinleyici", "● Canlı"]) {
                const l = F.text(page, t)
                verify(l, t + " missing")
                compare(xOf(l), xOf(cover), t) // left aligned, not centred
            }
            cleanup()
            Subwave.active = false
            Player.station = {}
            Player.current = { id: "1", title: "Song", artist: "Artist", album: "Alb", artistId: "a", albumId: "b" }
            make(nowPlaying, { args: {} })
            const cover2 = F.all(page, it => it.objectName === "cover")[0]
            for (const t of ["Song", "Artist", "Alb"]) {
                const l = F.text(page, t)
                verify(l, t + " missing")
                compare(xOf(l), xOf(cover2), t)
            }
        }
        function test_nowplaying_has_no_live_strip_for_stations() {
            live()
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "The News"))
            verify(!F.all(page, it => it.visible && typeof it.text === "string" && it.text.startsWith("Canlı ·")).length) // SUB/WAVE
            cleanup()
            Subwave.active = false
            Player.station = { id: "2", name: "Example Radio" }
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "Example Radio"))
            verify(!F.all(page, it => it.visible && typeof it.text === "string" && it.text.startsWith("Canlı ·")).length) // plain station
        }
        function strip() { return F.all(page, it => it.objectName === "strip" && it.visible)[0] }
        function test_station_strip_shows_the_stream_quality() {
            live()
            Player.streamInfo = "FLAC · 44,1 kHz · stereo · 890 kbps"
            make(nowPlaying, { args: {} })
            tryVerify(() => strip() && strip().text === "FLAC · 44,1 kHz · stereo · 890 kbps") // SUB/WAVE
            cleanup()
            Subwave.active = false
            Player.station = { id: "2", name: "Example Radio" }
            Player.streamInfo = "MP3 · 48 kHz · stereo"
            make(nowPlaying, { args: {} })
            tryVerify(() => strip() && strip().text === "MP3 · 48 kHz · stereo") // plain station
            Player.streamInfo = "" // nothing known yet: an empty line, not a made-up one
            tryVerify(() => !strip() || strip().text === "")
        }
        function test_song_strip_ignores_stream_info() {
            Player.streamInfo = "FLAC · 44,1 kHz · stereo"
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", suffix: "flac", samplingRate: 44100 }
            make(nowPlaying, { args: {} })
            tryVerify(() => strip() && strip().text.startsWith("Duraklatıldı"))
            verify(strip().text.indexOf("stereo") < 0)
        }
        function test_normal_tabs_for_songs_and_plain_stations() {
            make(nowPlaying, { args: {} })
            compare(visibleTabs(), ["Sıra  0", "Benzer", "Sözler"])
            cleanup()
            Player.station = { id: "1", name: "Plain" } // not SUB/WAVE: subwave is inactive
            make(nowPlaying, { args: {} })
            compare(visibleTabs(), ["Sıra  0", "Benzer", "Sözler"])
        }
        function reqField() { return F.all(page, it => it.objectName === "reqText" && it.visible)[0] }
        function test_live_request_form() {
            live()
            make(nowPlaying, { args: {} })
            mouseClick(tab("İstek"))
            tryVerify(() => reqField())
            const send = F.all(page, it => it.objectName === "reqSend" && it.visible)[0]
            mouseClick(send) // empty text: nothing goes out
            compare(Subwave.called("sendRequest").length, 0)
            reqField().text = "bir şeyler"
            mouseClick(send)
            compare(Subwave.called("sendRequest"), [["sendRequest", "bir şeyler", "me"]])
            Subwave.request = { state: "done", title: "T", artist: "A", queuePosition: 2 }
            tryVerify(() => F.text(page, "Çalınacak") && F.text(page, "T — A") && F.text(page, "Sırada: 2"))
            Subwave.request = { state: "rejected", message: "no" }
            tryVerify(() => F.text(page, "Reddedildi"))
        }
        function test_request_enter_does_not_resend_while_pending() {
            live()
            Subwave.request = { state: "pending" }
            make(nowPlaying, { args: {} })
            mouseClick(tab("İstek"))
            tryVerify(() => reqField())
            reqField().text = "bir şeyler"
            reqField().accepted() // Enter
            compare(Subwave.called("sendRequest").length, 0)
            Subwave.request = {}
            reqField().accepted()
            compare(Subwave.called("sendRequest").length, 1)
        }
        function test_live_upcoming() {
            live({ upcoming: [{ title: "U1", artist: "V1", requestedBy: "" }, { title: "U2", artist: "V2", requestedBy: "ayşe" }] })
            make(nowPlaying, { args: {} })
            mouseClick(tab("İstek"))
            tryVerify(() => F.text(page, "U1") && F.text(page, "U2"))
        }
        function test_live_toggle_keeps_page_usable() {
            live()
            make(nowPlaying, { args: {} })
            mouseClick(tab("İstek"))
            tryVerify(() => reqField())
            Subwave.active = false // the station stops being SUB/WAVE while the request tab is open
            tryVerify(() => visibleTabs().length === 3)
            mouseClick(tab("Benzer"))
            tryVerify(() => F.text(page, "Benzer şarkı bulunamadı."))
        }
        function test_not_live_hides_subwave_controls() {
            Player.station = { id: "1", name: "Plain" }
            Subwave.likeEnabled = true // a stale value must not show anything
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "Plain"))
            verify(!swHeart() && !tab("İstek"))
        }
        function test_theme_names_come_from_theme_json() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Genel")
            const box = F.all(m.settingsDialog.contentItem, it => it.objectName === "themeBox")[0]
            compare(box.model, ["Default", "Nothing"])
            compare(box.currentText, "Default")
            box.activated(1)
            compare(Player.theme, "nothing")
        }
        function test_plain_items_take_the_style_palette() { // Basic: window #ffffff, plain items' system palette #efefef
            const m = createTemporaryObject(mainWindow, null)
            const r = Qt.createQmlObject("import QtQuick; Rectangle {}", m.contentItem)
            compare(r.palette.window, m.palette.window)
            compare(r.palette.disabled.text, m.palette.disabled.text)
            const f = Qt.createQmlObject("import QtQuick; Rectangle {}", m.footer)
            compare(f.palette.windowText, m.palette.windowText)
        }
        function test_settings_show_the_build_date() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Hakkında")
            tryVerify(() => F.text(m.settingsDialog.contentItem, "Yapı tarihi: 01.02.2026"))
        }
        function test_favorites_empty() {
            Player.replies = { getStarred2: { starred2: {} } }
            make(favoritesPage, { args: {} })
            tryVerify(() => F.text(page, hd("Henüz favori yok")))
        }
        function test_heart_stars_song() {
            Player.replies = { star: {} }
            const r = make(songRow, {})
            mouseMove(r, r.width / 2, r.height / 2)
            tryVerify(() => F.text(r, "♡"))
            mouseClick(F.text(r, "♡"))
            compare(Player.called("set"), [["set", "star", { id: "s1" }]])
            verify(F.text(r, "♥"))
            wait(50)
            verify(F.text(r, "♥")) // the reply kept it
        }
        function test_heart_reverts_on_failure() {
            const r = make(songRow, {})
            mouseMove(r, r.width / 2, r.height / 2)
            tryVerify(() => F.text(r, "♡"))
            mouseClick(F.text(r, "♡"))
            tryVerify(() => F.text(r, "♡") && win.notice === "İşlem başarısız")
        }
        function test_heart_follows_a_new_item() {
            Player.replies = { star: {} }
            const h = make(heart, { item: { id: "s1" } })
            mouseClick(h)
            tryCompare(h, "on", true)
            h.item = { id: "s2" } // the next song is not a favorite: the heart must not stay lit
            compare(h.on, false)
            h.item = { id: "s3", starred: "2026-01-01" }
            compare(h.on, true)
        }
        function plReply(owner) {
            return { getPlaylist: { playlist: { id: "p1", name: "Mix", owner: owner, songCount: 2, duration: 300,
                                                entry: [{ id: "s1", title: "One" }, { id: "s2", title: "Two" }] } }, updatePlaylist: {} }
        }
        function rightClick(t) { mouseClick(F.text(page, hd(t)), 1, 1, Qt.RightButton) }
        function test_playlist_remove_song() {
            Player.replies = plReply("me")
            make(playlistPage, { args: { id: "p1" } })
            tryVerify(() => F.text(page, hd("Two")))
            rightClick("Two")
            compare(win.menued[0], "song")
            win.menued[2].onRemove()
            compare(Player.called("set"), [["set", "updatePlaylist", { playlistId: "p1", songIndexToRemove: [1] }]])
            tryVerify(() => !F.text(page, hd("Two")))
        }
        function test_playlist_not_editable_for_others() {
            Player.replies = plReply("x")
            make(playlistPage, { args: { id: "p1" } })
            tryVerify(() => F.text(page, hd("Two")))
            rightClick("Two")
            verify(!win.menued[2].onRemove)
            verify(!F.text(page, hd("Yeniden adlandır")) && !F.text(page, hd("Sil")))
        }
        function test_station_footer() {
            Player.station = { id: "1", name: "SUB/WAVE" }
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.text(m.footer, hd("SUB/WAVE")) && F.text(m.footer, hd("● Canlı")))
            verify(!F.all(m.footer, it => it.visible && it instanceof Slider && it.to !== 100).length) // seek bar hidden (the volume slider stays)
        }
        function volumeX(m) { return F.all(m.footer, it => it.objectName === "volume")[0].mapToItem(null, 0, 0).x }
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
        function test_volume_x_is_steady() {
            Player.current = { id: "1", title: "Song", artist: "A", album: "B", duration: 200 }
            const m = createTemporaryObject(mainWindow, null)
            tryVerify(() => F.all(m.footer, it => it.objectName === "volume").length > 0)
            for (const width of [900, 1400]) { // narrow and wide window
                m.width = width
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
        function test_volume_wheel_step() {
            Player.volume = 100
            const m = createTemporaryObject(mainWindow, null)
            const vol = F.all(m.footer, it => it instanceof Slider && it.to === 100 && it.value === 100)[0]
            waitForRendering(vol)
            mouseWheel(vol, vol.width / 2, vol.height / 2, 0, -120) // one notch down
            compare(Player.volume, 95)
        }
        function test_playlist_removals_one_at_a_time() {
            Player.replies = plReply("me")
            make(playlistPage, { args: { id: "p1" } })
            tryVerify(() => F.text(page, hd("Two")))
            Player.hold = true
            rightClick("Two")
            win.menued[2].onRemove()
            rightClick("One")
            verify(!win.menued[2].onRemove) // indexes would shift under a second request
            Player.release()
            tryVerify(() => { rightClick("One"); return !!win.menued[2].onRemove })
        }
        function test_album_card_play_button() {
            Player.replies = { getAlbum: { album: { id: "al1", song: [{ id: "a" }] } } }
            const c = make(albumCard, {})
            mouseMove(c, c.width / 2, c.width / 2)
            const b = F.all(c, it => it.objectName === "cardPlay")[0]
            tryVerify(() => b.visible)
            mouseClick(b)
            tryVerify(() => Player.called("play").length === 1)
            compare(win.opened, null)
        }
        function test_album_card_right_click_menu() {
            const c = make(albumCard, {})
            mouseClick(c, 10, 10, Qt.RightButton)
            compare(win.menued[0], "album")
            compare(win.opened, null)
        }
        function test_track_columns_drop_when_narrow() {
            const r = make(trackRow, {})
            const col = k => F.all(r, it => it.objectName === "col_" + k && it.visible).length > 0
            r.width = 1200; verify(col("bitrate"))
            r.width = 900; verify(!col("bitrate") && !col("year"))
            verify(col("album"))
            r.hideAlbum = true; verify(!col("album"))
        }
        function test_track_right_click_menu() {
            const r = make(trackRow, {})
            mouseClick(r, 50, r.height / 2, Qt.RightButton)
            compare(win.menued[0], "song"); compare(win.menued[1].id, "s1")
        }
        function test_now_playing_strip() {
            Player.queue = [{ id: "a", duration: 100 }, { id: "b", duration: 357 }, { id: "c", duration: 3000 }]
            Player.index = 1
            Player.current = { id: "b", duration: 357, suffix: "flac", samplingRate: 44100 }
            Player.position = 21; Player.duration = 357; Player.bitrate = 891; Player.playing = true
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, hd("Çalıyor (2/3) 0:21/5:57 · Toplam süre: 57:37 | FLAC 44.1 kHz, 891 kbps")) !== null)
            verify(!F.text(page, hd("Paylaş")))
        }
        function test_stars_rate_and_clear() {
            Player.replies = { setRating: {} }
            const s = make(stars, { item: { id: "s1", userRating: 2 } })
            mouseClick(F.all(s, it => it.objectName === "star4")[0])
            compare(Player.called("set"), [["set", "setRating", { id: "s1", rating: 4 }]])
            compare(s.rating, 4)
            mouseClick(F.all(s, it => it.objectName === "star4")[0])
            compare(Player.called("set")[1], ["set", "setRating", { id: "s1", rating: 0 }])
        }
        function test_stars_revert_on_failure() {
            const s = make(stars, { item: { id: "s1", userRating: 2 } })
            mouseClick(F.all(s, it => it.objectName === "star5")[0])
            tryVerify(() => s.rating === 2 && win.notice === "İşlem başarısız")
        }
        function test_share_copies_link() {
            Player.replies = { createShare: { shares: { share: [{ url: "https://m/share/abc" }] } } }
            const b = make(share, { itemId: "s1", label: hd("Paylaş") })
            mouseClick(b)
            tryVerify(() => Player.called("copy").length === 1)
            compare(Player.called("set")[0], ["set", "createShare", { id: "s1" }])
            compare(Player.called("copy")[0][1], "https://m/share/abc")
            compare(win.notice, "Link kopyalandı")
        }
        function test_repeat_button_cycles() {
            const m = createTemporaryObject(mainWindow, null)
            const b = F.all(m.footer, it => it.objectName === "repeat")[0]
            mouseClick(b); compare(Player.repeat, 1)
            mouseClick(b); compare(Player.repeat, 2)
            mouseClick(b); compare(Player.repeat, 0)
            mouseClick(F.all(m.footer, it => it.objectName === "shuffle")[0]); compare(Player.shuffle, true)
        }
        function test_synced_lyrics_follow_position() {
            Player.replies = { getLyricsBySongId: { lyricsList: { structuredLyrics: [{ synced: true, offset: 0,
                line: [{ start: 0, value: "A" }, { start: 5000, value: "B" }, { start: 10000, value: "C" }] }] } } }
            Player.position = 6
            const l = make(lyrics, { song: { id: "s1" } })
            tryVerify(() => l.line === 1)
            compare(Player.called("get", "getLyricsBySongId")[0][2], { id: "s1", enhanced: true })
        }
        function test_song_menu_actions() {
            const m = make(itemMenu, { kind: "song", item: { id: "s1", title: "S" } })
            m.popup()
            mi(m, "Sıradaki yap").triggered(); compare(Player.called("add")[0], ["add", [{ id: "s1", title: "S" }], 1])
            m.popup(); mi(m, "Bilgileri göster…").triggered(); compare(win.infoed, "s1")
            m.popup(); mi(m, "İndir…").triggered(); compare(win.downloaded[1], "song")
        }
        function test_song_menu_no_control() {
            Sync.canControl = false
            const m = make(itemMenu, { kind: "song", item: { id: "s1" } })
            m.popup()
            verify(!mi(m, "Çal").enabled && !mi(m, "Şarkı radyosu").enabled)
            verify(mi(m, "Favorilere ekle").enabled && mi(m, "Paylaş…").enabled)
        }
        function test_song_menu_rating() {
            Player.replies = { setRating: {} }
            const m = make(itemMenu, { kind: "song", item: { id: "s1" } })
            m.popup()
            const sub = mi(m, "Derecelendirme").subMenu
            mi(sub, "★★★").triggered()
            compare(Player.called("set")[0], ["set", "setRating", { id: "s1", rating: 3 }])
        }
        function test_queue_menu_removes() {
            const m = make(itemMenu, { kind: "song", item: { id: "s1" }, queueIndex: 4 })
            m.popup(); mi(m, "Kuyruktan çıkar").triggered()
            compare(Player.called("remove")[0], ["remove", 4])
        }
        function test_album_menu_plays_album() {
            Player.replies = { getAlbum: { album: { id: "al1", song: [{ id: "a" }, { id: "b" }] } } }
            const m = make(itemMenu, { kind: "album", item: { id: "al1", name: "X" } })
            m.popup(); mi(m, "Çal").triggered()
            tryVerify(() => Player.called("play").length === 1)
            compare(Player.called("play")[0], ["play", [{ id: "a" }, { id: "b" }], 0])
        }
        function test_shuffle_goes_through_the_core() { // the fake Player's shuffle reverses
            Player.replies = { getAlbum: { album: { id: "al1", name: "X", artist: "A", song: [{ id: "a" }, { id: "b" }] } } }
            const m = make(itemMenu, { kind: "album", item: { id: "al1", name: "X" } })
            m.popup(); mi(m, "Karıştır").triggered()
            tryVerify(() => Player.called("play").length === 1)
            compare(Player.called("play")[0], ["play", [{ id: "b" }, { id: "a" }], 0])
            compare(make(albumPage, { args: { id: "al1" } }).shuffled([1, 2, 3]), [3, 2, 1])
            compare(make(playlistPage, { args: { id: "p1" } }).shuffled([1, 2, 3]), [3, 2, 1])
        }
        function test_download_name_comes_from_the_core() {
            compare(Util.fileName({ name: "Mix" }, "playlist"), "file")
            compare(Player.called("downloadNameJson")[0], ["downloadNameJson", { name: "Mix" }, "playlist"])
        }
        function test_track_cell_text() {
            const cell = (k, s) => Columns.cell(Columns.byKey(k), s)
            compare(cell("size", { size: 1572864 }), "1.5 MB")
            compare(cell("genre", { genres: [{ name: "Folk" }, { name: "Rock" }] }), "Folk, Rock")
            compare(cell("year", { year: 0 }), "")
            compare(cell("created", { created: "2026-11-12T10:00:00Z" }), new Date("2026-11-12T10:00:00Z").toLocaleDateString(Qt.locale(), Locale.ShortFormat))
        }
        function test_info_dialog_rows() {
            Player.replies = { getSong: { song: { id: "s1", title: "Gone", path: "a/b.flac", size: 47185920, samplingRate: 44100, composer: "" } } }
            const d = make(infoDialog, {})
            d.show("s1")
            tryVerify(() => F.text(d.contentItem, "a/b.flac") !== null)
            verify(F.text(d.contentItem, "45.0 MB") && F.text(d.contentItem, "44.1 kHz"))
            verify(!F.text(d.contentItem, "Besteciler")) // empty field dropped
        }
        function test_settings_categories() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Scrobble")
            const s = m.settingsDialog
            tryVerify(() => F.text(s.contentItem, "Dinlenenleri gönder") !== null)
            m.openSettings("DJ")
            tryVerify(() => F.text(s.contentItem, "DJ modu") !== null)
        }
        function test_top_nav_centered() {
            const m = createTemporaryObject(mainWindow, null)
            const nav = F.all(m.header, it => it.objectName === "nav")[0]
            const c = nav.mapToItem(null, nav.width / 2, 0).x // scene = window coordinates
            verify(Math.abs(c - m.width / 2) <= 1)
        }
        function test_login_cancel_while_connecting() {
            Player.online = false; Player.signedIn = true; Player.connecting = true
            const m = createTemporaryObject(mainWindow, null)
            const b = F.text(m.contentItem, hd("Vazgeç"))
            verify(b !== null) // a hanging connect can still be abandoned
            mouseClick(b)
            compare(Player.called("logout").length, 1)
        }
        function test_login_error_is_red() {
            Player.online = false; Player.signedIn = false; Player.status = "olmadı"
            const m = createTemporaryObject(mainWindow, null)
            const line = F.text(m.contentItem, hd("olmadı"))
            verify(line && !Qt.colorEqual(line.color, "#e5484d"))
            Player.loginFailed = true
            verify(Qt.colorEqual(line.color, "#e5484d"))
        }
        function test_footer_has_no_format() {

            Player.current = { id: "s1", title: "T", suffix: "flac", bitDepth: 16, samplingRate: 44100 }
            Player.bitrate = 900
            const m = createTemporaryObject(mainWindow, null)
            verify(!F.all(m.footer, it => it.visible && typeof it.text === "string" && it.text.indexOf("kbps") >= 0).length)
            verify(F.text(m.footer, "DJ"))
        }
        function test_listener_toast_and_button() {
            Sync.available = true; Sync.users = [{ user: "A", following: "me" }]; Sync.listeners = ["A"]
            const m = createTemporaryObject(mainWindow, null)
            verify(F.text(m.header, "1 kişi seni dinliyor"))
            Sync.listenerJoined("A")
            compare(m.notice, "A seni dinliyor")
            Sync.listenerLeft("A")
            compare(m.notice, "A ayrıldı")
        }
        function test_outside_text_is_never_html() { // a tag in a name would otherwise fetch pictures from anywhere
            const name = "<img src='http://127.0.0.1:9/x.png'>"
            Sync.available = true; Sync.users = [{ user: name, following: null, station: null, title: name, artist: name, playing: true }]
            const m = createTemporaryObject(mainWindow, null)
            m.openTogether()
            const texts = () => F.all(m.togetherDialog.contentItem, it => it.visible && typeof it.text === "string" && it.text.indexOf("127.0.0.1") >= 0)
            tryVerify(() => texts().length >= 2)
            texts().forEach(it => compare(it.textFormat, Text.PlainText))
        }
        function test_together_tabs() {
            Sync.available = true; Sync.listeners = ["A"]
            Sync.users = [{ user: "C", following: "B", title: "T", artist: "R", playing: true }, { user: "A", following: "me" }]
            const m = createTemporaryObject(mainWindow, null)
            m.openTogether()
            const t = m.togetherDialog
            tryVerify(() => t.opened)
            const row = n => F.all(t.contentItem, it => it.objectName === "user_" + n && it.visible)[0]
            verify(row("A") && row("C"))
            verify(row("A").mapToItem(null, 0, 0).y < row("C").mapToItem(null, 0, 0).y) // who listens to me comes first
            verify(F.all(row("C"), it => it.visible && typeof it.text === "string" && it.text.indexOf("→") >= 0).length)
            mouseClick(F.text(t.contentItem, "Ayarlar"))
            tryVerify(() => F.text(t.contentItem, "Gizli") !== null)
            verify(!F.all(m.settingsDialog.contentItem, it => it.objectName === "cat_Birlikte dinle" && it.visible).length)
        }
        function togetherRow(u) { // the Together window with one online user; returns that user's row
            Sync.available = true; Sync.users = [u]
            const m = createTemporaryObject(mainWindow, null)
            m.openTogether()
            const t = m.togetherDialog
            tryVerify(() => t.opened)
            return F.all(t.contentItem, it => it.objectName === "user_" + u.user && it.visible)[0]
        }
        function hint(row) { return F.all(row, it => it.visible && typeof it.text === "string" && (it.text.startsWith("📻") || it.text.startsWith("▶") || it.text.startsWith("❚")))[0] }
        function test_together_row_shows_station() {
            const row = togetherRow({ user: "ali", following: null, station: "SUB/WAVE", title: "T", artist: "A", playing: true })
            verify(row)
            tryVerify(() => hint(row) && hint(row).text === "📻 SUB/WAVE · T · A")
        }
        function test_together_row_station_without_track() {
            const row = togetherRow({ user: "ali", following: null, station: "SUB/WAVE", title: null, artist: null, playing: true })
            tryVerify(() => hint(row) && hint(row).text === "📻 SUB/WAVE")
        }
        function test_together_row_song_unchanged() {
            const row = togetherRow({ user: "ali", following: null, station: null, title: "Song", artist: "Artist", playing: true })
            tryVerify(() => hint(row) && hint(row).text === "▶  Song  ·  Artist")
        }
        function test_grid_size_option() {
            Player.options = { gridSize: 220 }
            compare(createTemporaryObject(albumCard, host).width, 220) // not make(): it stretches the item to the window
            compare(createTemporaryObject(artistCard, host).width, Math.round(220 * 150 / 172))
            make(albumsPage, { args: {} })
            compare(F.all(page, it => it.cellWidth !== undefined)[0].cellWidth, 220 + 24)
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Genel")
            tryVerify(() => F.text(m.settingsDialog.contentItem, "Izgara boyutu") !== null)
        }
        function test_track_artists_link() {
            const r = make(trackRow, { song: { id: "s1", title: "T", artists: [{ id: "a1", name: "X" }, { id: "a2", name: "Y" }] } })
            mouseClick(F.text(r, "Y"))
            compare(win.opened[0], "artist"); compare(win.opened[1].id, "a2")
            win.opened = null
            r.song = { id: "s1", title: "T", artist: "Z", artistId: "a9" } // no artists list: the one artist
            mouseClick(F.text(r, "Z"))
            compare(win.opened[1].id, "a9")
            win.opened = null
            r.song = { id: "s1", title: "T", artist: "Q", artists: [{ name: "Q" }] } // no id: plain text
            mouseClick(F.text(r, "Q"))
            compare(win.opened, null)
        }
        function test_album_genre_link() {
            Player.replies = { getAlbum: { album: { id: "al1", name: "A", genres: [{ name: "Folk" }, { name: "Rock" }], song: [] } } }
            make(albumPage, { args: { id: "al1" } })
            tryVerify(() => F.text(page, "Folk") !== null)
            mouseClick(F.text(page, "Folk"))
            compare(win.opened[0], "genre"); compare(win.opened[1].name, "Folk")
        }
        function test_column_menu() {
            const h = make(trackHeader, {})
            mouseClick(h, 50, h.height / 2, Qt.RightButton)
            const m = h.columnMenu
            tryVerify(() => m.opened)
            compare(m.count, 18)
            const fav = mi(m, "Fav.")
            verify(fav.checked)
            mouseClick(fav)
            compare(Player.called("setOption").length, 1)
            verify(Player.options.columns.split(",").indexOf("fav") < 0 && Player.options.columns.indexOf("artist") >= 0)
            const r = createTemporaryObject(trackRow, host, { width: 1200, song: { id: "s1", title: "T", size: 1048576, path: "a/b.flac" } })
            const cols = () => F.all(r, it => it.visible && String(it.objectName).startsWith("col_")).map(it => it.objectName).sort().join(" ")
            verify(cols().indexOf("col_fav") < 0)
            Player.options = { columns: "size" }
            compare(cols(), "col_num col_size col_title")
            verify(F.text(r, "1.0 MB"))
            Player.options = { columns: "bogus,,path" } // unknown keys are skipped
            compare(cols(), "col_num col_path col_title")
        }
        function test_columns_wait_for_width() {
            // a new row has width 0 first: building the narrow set then the wide one doubles the work while scrolling
            const r = createTemporaryObject(trackRow, host, { width: 0 })
            const tc = F.all(r, it => it.keys !== undefined && it.columns !== undefined)[0]
            compare(tc.keys, "")
            r.width = 1200
            verify(tc.keys.indexOf("bitrate") >= 0)
        }
        function test_no_lyrics() {

            Player.replies = { getLyricsBySongId: { lyricsList: {} } }
            make(lyrics, { song: { id: "s1" } })
            tryVerify(() => F.text(page, hd("Söz bulunamadı")))
        }
    }
}
