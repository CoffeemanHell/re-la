// Lyrics of a song (getLyricsBySongId). Synced lyrics follow the playback: the current line, and with
// word timing the current word. The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: ly
    property var song
    property bool active: true
    property bool live: false
    readonly property string songId: live ? Subwave.now.id || "" : song && song.id ? song.id : ""
    property var raw: []        // lines as they come; `lines` is them with word timing split out
    readonly property var lines: split(raw)
    property bool synced: false
    property int offset: 0      // ms; positive shows lines sooner (OpenSubsonic)
    property bool loading: false
    property bool failed: false
    property string forId: ""
    readonly property real ms: live ? Subwave.elapsedMs : Player.position * 1000
    readonly property int line: synced ? lineAt(ms) : -1
    // index of the sung word in the current line (enhanced LRC); -1 when the line has no word timing
    readonly property int word: {
        const w = line >= 0 && lines[line].words
        if (!w) return -1
        let k = -1
        for (let i = 0; i < w.length && w[i].start - offset <= ms; i++) k = i
        return k
    }
    readonly property string lyState: live ? Subwave.lyricsState : ""
    function lineAt(ms) {
        let k = -1
        for (let i = 0; i < lines.length && lines[i].start - offset <= ms; i++) k = i
        return k
    }
    // Enhanced LRC: "<mm:ss.xx>" before each word. value loses the tags; words = [{start, value}].
    function split(list) {
        const tag = /<(\d+):(\d+(?:\.\d+)?)>/g
        return list.map(l => {
            const v = l.value || ""
            if (!v.match(tag)) return l
            const words = []
            let start = l.start, at = 0, m
            tag.lastIndex = 0
            while ((m = tag.exec(v)) !== null) {
                const t = v.slice(at, m.index)
                if (t.trim() !== "") words.push({ start: start, value: t })
                else if (words.length) words[words.length - 1].value += t
                start = Math.round((Number(m[1]) * 60 + Number(m[2])) * 1000)
                at = tag.lastIndex
            }
            const t = v.slice(at)
            if (t.trim() !== "") words.push({ start: start, value: t })
            else if (words.length) words[words.length - 1].value += t
            const out = Object.assign({}, l, { value: words.map(w => w.value).join("").trim() })
            if (words.length) out.words = words
            return out
        })
    }
    // cueLine[i].cue = timed words or syllables of line[index]; each keeps the text up to the next one (its spaces).
    // Cues belong to cueLine.value (it may differ from the line's), so that becomes the line's text.
    function withCues(list, cueLines) {
        const out = list.slice()
        for (const c of cueLines) {
            const l = out[c.index], cues = c.cue || []
            if (!l || !cues.length) continue
            const v = c.value || l.value || ""
            let at = 0
            const pos = cues.map(q => { const i = v.indexOf(q.value, at); if (i >= 0) at = i + q.value.length; return i })
            if (pos.some(i => i < 0)) continue // cues do not match the line text: keep it a plain line
            const words = cues.map((q, k) => ({ start: q.start, value: v.slice(k ? pos[k] : 0, k + 1 < cues.length ? pos[k + 1] : v.length) }))
            out[c.index] = Object.assign({}, l, { value: v, words: words })
        }
        return out
    }
    // line i lights up word by word: it is the current line and has word timing
    function karaoke(i) { return synced && i === line && !!lines[i] && !!lines[i].words }
    // what a theme shows for line i (StyledText when karaoke(i)); `on`/`off`: colors of sung/unsung words
    function lineText(i, on, off) { return karaoke(i) ? sung(lines[i], word, on, off) : lines[i].value || " " }
    // the line as StyledText: sung words in `on`, the rest in `off`
    function sung(l, upto, on, off) {
        const esc = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
        const a = l.words.slice(0, upto + 1).map(w => w.value).join("").replace(/^\s+/, "")
        let b = l.words.slice(upto + 1).map(w => w.value).join("")
        if (!a) b = b.replace(/^\s+/, "")
        return "<font color='" + on + "'>" + esc(a) + "</font><font color='" + off + "'>" + esc(b) + "</font>"
    }
    function load() {
        const id = songId
        if (!active || live || id === forId) return
        forId = id
        raw = []; synced = false; offset = 0; failed = false
        if (!id) return
        loading = true
        // enhanced: word timing as cueLine[].cue[] (OpenSubsonic songLyrics v2); older servers ignore it
        Player.get(ly, "getLyricsBySongId", { id: id, enhanced: true }, r => {
            if (id !== forId) return
            loading = false
            if (!r) return failed = true
            // v2 also lists translations and pronunciations; show the main track (else whatever there is)
            const list = (r.lyricsList && r.lyricsList.structuredLyrics) || []
            const main = list.filter(l => !l.kind || l.kind === "main")
            const all = main.length ? main : list
            const pick = all.find(l => l.synced) || all[0]
            if (!pick) return
            synced = !!pick.synced
            offset = pick.offset || 0
            raw = withCues(pick.line || [], pick.cueLine || [])
        })
    }
    // later, not inside the binding update that triggered it (an immediate reply would loop the bindings)
    onSongIdChanged: Qt.callLater(load)
    onActiveChanged: Qt.callLater(load)
    Binding { target: ly; property: "raw"; value: Subwave.lyrics.lines || []; when: ly.live }
    Binding { target: ly; property: "synced"; value: !!Subwave.lyrics.synced; when: ly.live }
    Binding { target: ly; property: "offset"; value: Subwave.lyrics.offset || 0; when: ly.live }
    Binding { target: ly; property: "loading"; value: ly.lyState === "loading" || ly.lyState === "retry"; when: ly.live }
    Binding { target: ly; property: "failed"; value: false; when: ly.live }
    Component.onCompleted: Qt.callLater(load)
}
