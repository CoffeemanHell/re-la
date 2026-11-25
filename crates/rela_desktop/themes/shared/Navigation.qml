// Which page is shown and the back / forward history. The window keeps one and forwards go(), back() ... to it.
import QtQuick
import Rela

QtObject {
    property var loader      // the Loader the pages are shown in
    property var searchField // the search field: going back to a search puts its text there
    signal load(string file, var props) // show this page file with these properties (the theme loads it: the file is its own)

    readonly property var pages: ({ home: "HomePage.qml", albums: "AlbumsPage.qml", album: "AlbumPage.qml",
                                    artist: "ArtistPage.qml", nowplaying: "NowPlayingPage.qml", search: "SearchPage.qml",
                                    favorites: "FavoritesPage.qml", artists: "ArtistsPage.qml", genres: "GenresPage.qml",
                                    genre: "GenrePage.qml", tracks: "TracksPage.qml", radio: "RadioPage.qml",
                                    playlists: "PlaylistsPage.qml", playlist: "PlaylistPage.qml" })
    property var history: []   // [{page, args}]
    property int at: -1        // shown entry
    readonly property bool canBack: at > 0
    readonly property bool canForward: at < history.length - 1
    readonly property string page: at >= 0 ? history[at].page : ""

    // leaving a page: keep what it loaded and where it was scrolled, so back/forward shows it as it was
    function leave() { if (at >= 0 && loader.item && loader.item.saveState) history[at].state = loader.item.saveState() }
    function show() {
        const e = history[at]
        const props = { args: e.args }
        if (e.state) props.saved = e.state
        if (e.page === "search" && searchField.text !== e.args.query) searchField.text = e.args.query // back/forward: the field shows that search
        load(pages[e.page], props)
    }
    function go(page, args) {
        args = args || {}
        const cur = history[at]
        if (cur && cur.page === page && JSON.stringify(cur.args) === JSON.stringify(args)) return
        leave()
        const h = history.slice(0, at + 1).concat([{ page: page, args: args }])
        history = h.slice(-50) // saved pages hold album lists: keep the last 50 entries
        at = history.length - 1
        show()
    }
    function back() { if (canBack) { leave(); at--; show() } }
    function forward() { if (canForward) { leave(); at++; show() } }
    function reload() { if (at >= 0) { history[at].state = undefined; show() } } // fresh from the server
    // typing in the search field: a new entry the first time, then the same entry follows the text
    function search(q) {
        if (!q.trim()) return
        if (page !== "search") return go("search", { query: q })
        history[at].args = { query: q }
        loader.item.args = history[at].args // the page re-queries
    }
    property bool offline: false // the pages were opened without the server
    // online: open Home once; offline with songs on disk: the queue; logged out: drop the pages so nothing keeps calling
    // the server
    function start() {
        if (Player.offline && at < 0) go("nowplaying")
        if (Player.online && at < 0) go("home")
        if (!Player.online && !Player.offline && at >= 0) { loader.source = ""; at = -1; history = [] }
        if (Player.online && offline) reload() // what could not be loaded
        offline = Player.offline
    }
}
