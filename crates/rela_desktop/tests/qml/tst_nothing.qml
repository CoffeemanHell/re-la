// Pages of the nothing theme against FakePlayer: clicks open pages, buttons call the Player.
import QtQuick
import QtQuick.Controls
import QtTest
import "../../themes/nothing"
import Rela

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

    TestCase {
        name: "nothing"; when: win.visible
        property var page: null
        function init() { Player.reset(); Subwave.reset(); Sync.canControl = true; Sync.available = false; Sync.users = []; Sync.listeners = []; Sync.following = ""; win.opened = null; win.notice = ""; win.asked = null; win.confirmed = null; win.menued = null; win.infoed = null; win.downloaded = null }
        function trUp(t) { return t.replace(/i/g, "İ").toUpperCase() } // the theme writes Turkish capitals (İ) by hand
        function mi(m, t) { for (let i = 0; i < m.count; i++) if (m.itemAt(i) && m.itemAt(i).text === trUp(t)) return m.itemAt(i); return null }
        function cleanup() { if (page) page.destroy(); page = null }
        function make(comp, props) { page = createTemporaryObject(comp, host, props); page.width = host.width; page.height = host.height; if (page instanceof Item) waitForRendering(page); return page } // popups (menus, dialogs) are not Items
        function hd(s) { return "nothing" === "nothing" ? s.toUpperCase() : s }

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
            tryVerify(() => F.text(m.footer, "THE NEWS") && F.text(m.footer, "DEAD CONFEDERATE") && F.text(m.footer, "● CANLI"))
            verify(F.all(m.footer, it => it instanceof Image && it.source.toString() === Subwave.coverUrl).length > 0)
        }
        function test_subwave_nowplaying() {
            live({ show: "Gece" })
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "THE NEWS") && F.text(page, "DEAD CONFEDERATE") && F.text(page, "W") && F.text(page, "DJ WAVE · GECE · 1 DİNLEYİCİ"))
            verify(F.all(page, it => it instanceof Image && it.source.toString() === Subwave.coverUrl).length > 0)
            verify(!F.text(page, "BAĞLANTI YOK"))
            Subwave.online = false
            tryVerify(() => F.text(page, "BAĞLANTI YOK"))
        }
        function test_plain_station_has_no_subwave_line() {
            Player.station = { id: "1", name: "Plain" }
            Subwave.likeEnabled = true // a stale value must not show anything
            make(nowPlaying, { args: {} })
            tryVerify(() => F.text(page, "PLAIN"))
            verify(!F.all(page, it => it.visible && typeof it.text === "string" && it.text.startsWith("DJ ")).length)
            compare(tabs(), ["SIRA 00", "BENZER", "SÖZLER"])
        }
        function test_radio_subtitle() {
            Player.replies = { getInternetRadioStations: { internetRadioStations: { internetRadioStation: [
                { id: "1", name: "Wave", streamUrl: "x", homePageUrl: "https://radio.example.com" },
                { id: "2", name: "Plain", streamUrl: "y", homePageUrl: "https://plain.example.com" },
                { id: "3", name: "Flac", streamUrl: "z", originUrl: "https://flac.example.com" },
                { id: "4", name: "Nowhere", streamUrl: "q", originUrl: "" }] } } }
            make(radioPage, { args: {} })
            tryVerify(() => F.text(page, "WAVE"))
            compare(Subwave.called("probe").map(c => c[1]), ["https://radio.example.com", "https://plain.example.com", "https://flac.example.com"])
            Subwave.peeked = { "https://radio.example.com": { title: "T", artist: "A", dj: "wave" } }
            tryVerify(() => F.text(page, "T — A · WAVE"))
            verify(F.text(page, "HTTPS://PLAIN.EXAMPLE.COM"))
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
            mouseClick(F.text(l, "b"))
            compare(Player.called("seek").length, 0) // a live stream has no seek
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
            tryVerify(() => F.text(l, "PAROLA GEREKLİ"))
            Subwave.lyricsState = "off"
            tryVerify(() => F.text(l, "SÖZLER KAPALI"))
            Subwave.lyricsState = "none"
            tryVerify(() => F.text(l, "SÖZ BULUNAMADI"))
            Subwave.lyricsState = "retry"
            tryCompare(l, "loading", true)
            verify(!F.text(l, "SÖZ BULUNAMADI"))
        }
        function tabs() { return F.all(page, it => it.visible && it.on !== undefined && ["SÖZLER", "İSTEK", "BENZER"].concat(["SIRA 00"]).indexOf(it.text) >= 0).map(b => b.text) }
        function tab(name) { return F.all(page, it => it.visible && it.on !== undefined && it.text === name)[0] }
        function footerHeart(m) { return F.all(m.footer, it => it.objectName === "swHeart" && it.visible)[0] }
        function test_live_tabs() {
            live()
            liveLyrics("auth")
            Subwave.now = Object.assign({ title: "The News" }, Subwave.now)
            make(nowPlaying, { args: {} })
            tryVerify(() => tabs().length === 2)
            compare(tabs(), ["SÖZLER", "İSTEK"])
            tryVerify(() => F.text(page, "PAROLA GEREKLİ")) // the lyrics tab is the live one
            Subwave.active = false // the station stops being SUB/WAVE: the song tabs are back
            tryVerify(() => tabs().length === 3)
            waitForRendering(page) // the pills have just been laid out
            mouseClick(tab("BENZER"))
            tryVerify(() => F.text(page, "BENZER ŞARKI BULUNAMADI"))
        }
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
            Subwave.active = false // a plain station: no heart
            tryVerify(() => !footerHeart(m))
        }
        function test_station_strip_shows_the_stream_quality() {
            live()
            Player.streamInfo = "FLAC · 44,1 kHz · stereo · 890 kbps"
            make(nowPlaying, { args: {} })
            const strip = () => F.all(page, it => it.objectName === "strip" && it.visible)[0]
            tryVerify(() => strip() && strip().text === "FLAC · 44,1 KHZ · STEREO · 890 KBPS")
            verify(!F.all(page, it => it.visible && typeof it.text === "string" && it.text.startsWith("CANLI ·")).length)
        }
        function reqField() { return F.all(page, it => it.objectName === "reqText" && it.visible)[0] }
        function test_live_request_form() {
            live({ upcoming: [{ title: "U1", artist: "V1", requestedBy: "" }, { title: "U2", artist: "V2", requestedBy: "ayşe" }] })
            make(nowPlaying, { args: {} })
            mouseClick(tab("İSTEK"))
            tryVerify(() => reqField())
            tryVerify(() => F.text(page, "U1") && F.text(page, "U2") && F.text(page, "V2 · AYŞE"))
            const send = F.all(page, it => it.objectName === "reqSend" && it.visible)[0]
            mouseClick(send) // empty text: nothing goes out
            compare(Subwave.called("sendRequest").length, 0)
            reqField().text = "bir şeyler"
            mouseClick(send)
            compare(Subwave.called("sendRequest"), [["sendRequest", "bir şeyler", "me"]])
            Subwave.request = { state: "pending" }
            reqField().accepted() // Enter while the answer is awaited: not sent again
            compare(Subwave.called("sendRequest").length, 1)
            Subwave.request = { state: "done", title: "T", artist: "A", queuePosition: 2 }
            tryVerify(() => F.text(page, "ÇALINACAK") && F.text(page, "T — A") && F.text(page, "SIRADA: 2"))
            Subwave.request = { state: "rejected", message: "no" }
            tryVerify(() => F.text(page, "REDDEDİLDİ") && F.text(page, "NO"))
        }
        function test_footer_progress() {
            live({ duration: 271 })
            Subwave.elapsedMs = 42000
            const m = createTemporaryObject(mainWindow, null)
            const bar = () => F.all(m.footer, it => it.objectName === "swProgress" && it.visible)[0]
            tryVerify(() => F.text(m.footer, "00:42") && F.text(m.footer, "04:31") && bar())
            verify(Math.abs(bar().value - 42 / 271) < 0.001)
            Subwave.elapsedMs = 400000
            tryVerify(() => F.all(m.footer, it => it.visible && it.text === "04:31").length === 2) // elapsed stops at the duration
            Subwave.now = Object.assign({}, Subwave.now, { duration: 0 })
            tryVerify(() => !bar()) // no bar and no total when the duration is unknown
            Subwave.active = false
            Player.station = { id: "2", name: "Plain" }
            wait(50)
            verify(!F.all(m.footer, it => it.visible && typeof it.text === "string" && /^\d+:\d\d$/.test(it.text)).length)
        }
        function test_volume_wheel_and_mute() {
            Player.volume = 100
            const m = createTemporaryObject(mainWindow, null)
            const vol = F.all(m.footer, it => it.objectName === "volume")[0]
            waitForRendering(vol)
            mouseWheel(vol, vol.width / 2, vol.height / 2, 0, -120) // one notch down
            compare(Player.volume, 95)
            mouseClick(F.text(m.footer, "SES"))
            compare(Player.volume, 0)
            mouseClick(F.text(m.footer, "SESSİZ"))
            compare(Player.volume, 95)
        }
        function test_seek_needs_a_song() {
            const m = createTemporaryObject(mainWindow, null)
            const seek = F.all(m.footer, it => it.dots !== undefined && it.visible && it.objectName === "")[0]
            verify(seek && !seek.enabled) // nothing queued: nothing to seek
            Player.current = { id: "1", title: "Song", duration: 200 }
            tryCompare(seek, "enabled", true)
        }
        function test_together_row_shows_station() {
            Sync.available = true
            Sync.users = [{ user: "B", following: "", station: "SUB/WAVE", title: "The News", artist: "Dead Confederate", playing: true },
                          { user: "C", following: "", station: "Plain", title: "", artist: "", playing: true },
                          { user: "D", following: "", title: "T", artist: "R", playing: false }]
            const m = createTemporaryObject(mainWindow, null)
            m.openTogether()
            const t = m.togetherDialog
            tryVerify(() => t.opened)
            tryVerify(() => F.text(t.contentItem, "RADYO SUB/WAVE · THE NEWS · DEAD CONFEDERATE"))
            verify(F.text(t.contentItem, "RADYO PLAIN"))
            verify(F.text(t.contentItem, "❚❚  T  ·  R"))
        }
        function test_theme_names_come_from_theme_json() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Genel")
            tryVerify(() => F.text(m.settingsDialog.contentItem, "NOTHING") && F.text(m.settingsDialog.contentItem, "DEFAULT"))
        }
        function test_settings_show_the_build_date() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Hakkında")
            tryVerify(() => F.text(m.settingsDialog.contentItem, "YAPI TARİHİ: 01.02.2026"))
        }
        function test_favorites_empty() {
            Player.replies = { getStarred2: { starred2: {} } }
            make(favoritesPage, { args: {} })
            tryVerify(() => F.text(page, "HENÜZ FAVORİ YOK"))
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
            tryVerify(() => F.text(m.footer, hd("SUB/WAVE")) && F.text(m.footer, "● CANLI"))
            verify(!F.all(m.footer, it => it.visible && it instanceof Slider && it.to !== 100).length) // seek bar hidden (the volume slider stays)
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
        function test_info_dialog_rows() {
            Player.replies = { getSong: { song: { id: "s1", title: "Gone", path: "a/b.flac", size: 47185920, samplingRate: 44100, composer: "" } } }
            const d = make(infoDialog, {})
            d.show("s1")
            tryVerify(() => F.text(d.contentItem, "a/b.flac") !== null)
            verify(F.text(d.contentItem, "45.0 MB") && F.text(d.contentItem, "44.1 kHz"))
            verify(!F.text(d.contentItem, trUp("Besteciler"))) // empty field dropped
        }
        function test_settings_categories() {
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Scrobble")
            const s = m.settingsDialog
            tryVerify(() => F.text(s.contentItem, trUp("Dinlenenleri gönder")) !== null)
            m.openSettings("DJ")
            tryVerify(() => F.text(s.contentItem, trUp("DJ modu")) !== null)
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
            verify(line && !Qt.colorEqual(line.color, App.theme.accent))
            Player.loginFailed = true
            verify(Qt.colorEqual(line.color, App.theme.accent))
        }
        function test_footer_has_no_format() {

            Player.current = { id: "s1", title: "T", suffix: "flac", bitDepth: 16, samplingRate: 44100 }
            Player.bitrate = 900
            const m = createTemporaryObject(mainWindow, null)
            verify(!F.all(m.footer, it => it.visible && typeof it.text === "string" && it.text.toLowerCase().indexOf("kbps") >= 0).length)
            verify(F.text(m.footer, "DJ"))
        }
        function test_listener_toast_and_button() {
            Sync.available = true; Sync.users = [{ user: "A", following: "me" }]; Sync.listeners = ["A"]
            const m = createTemporaryObject(mainWindow, null)
            verify(F.text(m.header, "1 KİŞİ SENİ DİNLİYOR"))
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
            mouseClick(F.text(t.contentItem, "AYARLAR"))
            tryVerify(() => F.text(t.contentItem, "GİZLİ") !== null)
            verify(!F.all(m.settingsDialog.contentItem, it => it.objectName === "cat_Birlikte dinle" && it.visible).length)
        }
        function test_grid_size_option() {
            Player.options = { gridSize: 220 }
            compare(createTemporaryObject(albumCard, host).width, 220) // not make(): it stretches the item to the window
            compare(createTemporaryObject(artistCard, host).width, Math.round(220 * 150 / 172))
            make(albumsPage, { args: {} })
            compare(F.all(page, it => it.cellWidth !== undefined)[0].cellWidth, 220 + 24)
            const m = createTemporaryObject(mainWindow, null)
            m.openSettings("Genel")
            tryVerify(() => F.text(m.settingsDialog.contentItem, "IZGARA BOYUTU") !== null)
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
            tryVerify(() => F.text(page, "FOLK") !== null)
            mouseClick(F.text(page, "FOLK"))
            compare(win.opened[0], "genre"); compare(win.opened[1].name, "Folk")
        }
        function test_column_menu() {
            const h = make(trackHeader, {})
            mouseClick(h, 50, h.height / 2, Qt.RightButton)
            const m = h.columnMenu
            tryVerify(() => m.opened)
            compare(m.count, 18)
            const fav = mi(m, "FAV.")
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
