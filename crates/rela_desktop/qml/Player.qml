// `Player` for themes (import Rela): the Rust PlayerCore (src/player.rs) with what only JS can
// hold. get/set keep each `done` callback here until the core answers; calls that take JS values hand them over as
// JSON. The API themes use is in docs/themes.md.
pragma Singleton
import QtQuick
import Rela // its own module: the file sits in qml/, so PlayerCore is not seen without it

PlayerCore {
    id: core

    property int _next: 1
    property var _pending: ({}) // request id -> {owner, done}

    function _call(owner, method, params, done, write) {
        const id = _next++
        if (!request(id, method, JSON.stringify(params || {}), write)) {
            done(null) // not a method themes may call
            return
        }
        _pending[id] = { owner: owner, done: done }
    }
    // Read-only Subsonic call for pages: done(response) or done(null). Skipped once owner is destroyed.
    function get(owner, method, params, done) { _call(owner, method, params, done, false) }
    // Writes (favorites, ratings, playlists): the same, for the few methods allowed.
    function set(owner, method, params, done) { _call(owner, method, params, done, true) }

    onReplied: (id, json, call) => {
        const p = _pending[id]
        delete _pending[id]
        // A destroyed owner reads as an object without a parent; a page the Loader just replaced is detached at once
        // but deleted later: its parent is already gone. Either way the page is not there to take the answer.
        if (p && call && p.owner && p.owner.parent)
            p.done(JSON.parse(json))
    }

    function play(songs, start) { playJson(JSON.stringify(songs), start) } // replace the queue, play from start
    function add(songs, where) { addJson(JSON.stringify(songs), where) }   // 0 play now, 1 next, 2 end
    function setOption(key, value) { setOptionJson(key, JSON.stringify(value)) }
    function download(id, file) { downloadTo(id, file.toString()) }        // song: original file, album / playlist: zip

    // Player.copy(text): the system clipboard, through a TextEdit (Qt's clipboard without any C++)
    property TextEdit _clip: TextEdit { visible: false }
    function copy(text) {
        _clip.text = text
        _clip.selectAll()
        _clip.copy()
        _clip.text = ""
    }}
