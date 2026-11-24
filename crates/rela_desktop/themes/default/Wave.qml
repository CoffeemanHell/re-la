// Waveform bars for the seek row: one bar per 2 px, the song's peaks as a light outer bar and its
// loudness (RMS) as a solid inner one. The played part is in the highlight color, the rest dimmed.
// Both parts are drawn once per song and size; playing only moves the cut between them (no repaint per tick).
import QtQuick

Item {
    id: wv
    property var levels: [] // peaks 0..1
    property var rms: []    // same length, each <= its peak
    property real value: 0  // 0..1 played
    readonly property real cut: Math.round(width * (value > 0 ? Math.min(1, value) : 0)) // NaN -> 0

    function draw(canvas, played) {
        const c = canvas.getContext("2d")
        c.reset()
        const n = levels.length
        if (!n || width < 1) return
        const bars = Math.max(1, Math.floor(width / 2)), step = width / bars
        const top = (a, from, to) => { let v = 0; for (let k = from; k < to; k++) v = Math.max(v, a[k] || 0); return v }
        c.fillStyle = String(played ? palette.highlight : palette.placeholderText)
        const w = Math.max(1, step - 0.5)
        for (let i = 0; i < bars; i++) {
            const from = Math.floor(i * n / bars), to = Math.max(Math.floor((i + 1) * n / bars), from + 1)
            const peak = Math.max(1, top(levels, from, to) * height), body = Math.max(1, top(rms, from, to) * height)
            c.globalAlpha = played ? 0.45 : 0.3
            c.fillRect(i * step, (height - peak) / 2, w, peak)
            c.globalAlpha = played ? 1 : 0.75
            c.fillRect(i * step, (height - body) / 2, w, body)
        }
    }
    onLevelsChanged: { lit.requestPaint(); dim.requestPaint() }
    onRmsChanged: { lit.requestPaint(); dim.requestPaint() }
    onPaletteChanged: { lit.requestPaint(); dim.requestPaint() }

    Item { // left of the cut: the played canvas
        width: wv.cut; height: wv.height; clip: true
        Canvas { id: lit; width: wv.width; height: wv.height; onPaint: wv.draw(this, true); onWidthChanged: requestPaint(); onHeightChanged: requestPaint() }
    }
    Item { // right of the cut: the rest, shifted so its bars line up
        x: wv.cut; width: wv.width - wv.cut; height: wv.height; clip: true
        Canvas { id: dim; x: -wv.cut; width: wv.width; height: wv.height; onPaint: wv.draw(this, false); onWidthChanged: requestPaint(); onHeightChanged: requestPaint() }
    }
}
