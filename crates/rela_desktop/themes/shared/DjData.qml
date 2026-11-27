// What the DJ knows, for its settings and state page: its last pick (djState) and the listener's "more / less like
// this" list (djFeedback). Both are answered by the core itself, offline too.
import QtQuick
import Rela

QtObject {
    id: root
    property var state: ({})    // {searching, seed: {title, artist}, session: [{tag, weight}], rows: [...]}; {} before a pick
    property var feedback: []   // [{id, title, artist, dir, day}], newest first
    readonly property bool picked: !!(state.rows && state.rows.length)
    function refresh() { Player.get(root, "djState", {}, r => root.state = r || {}) }
    function refreshFeedback() { Player.get(root, "djFeedback", {}, r => root.feedback = r || []) }
    // the listener's choice for one song: 1, -1 or 0
    function dirOf(id) { const f = feedback.find(x => x.id === id); return f ? f.dir : 0 }
    // dir again takes the choice back
    function toggle(song, dir) {
        const d = dirOf(song.id) === dir ? 0 : dir
        Player.djFeedback(song, d)
        const rest = feedback.filter(x => x.id !== song.id)
        feedback = d ? [{ id: song.id, title: song.title || song.id, artist: song.artist || "", dir: d, day: 0 }].concat(rest) : rest
        return d
    }
}
