package app.rela.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.unit.dp
import uniffi.rela.Wave

/** n bars from any number of values: each bar is the loudest value of its range (as Wave.qml draws). */
fun bars(values: List<Float>, n: Int): FloatArray {
    if (n <= 0 || values.isEmpty()) return FloatArray(0)
    return FloatArray(n) { i ->
        val lo = i * values.size / n
        val hi = maxOf(lo + 1, (i + 1) * values.size / n)
        var v = 0f
        for (k in lo until hi) v = maxOf(v, values[k])
        v
    }
}

/** A wave's bars for the width last drawn: the seek bar redraws many times a second, its width does not change. */
private class Bars(private val wave: Wave) {
    var peaks = FloatArray(0)
    var rms = FloatArray(0)

    fun fit(n: Int) {
        if (peaks.size == n) return
        peaks = bars(wave.peaks, n)
        rms = bars(wave.rms, n)
    }
}

/**
 * The seek bar's track as the song's waveform: per 3 dp a bar, the peak light, the loudness (RMS) solid; the played
 * part in the primary color. Drawn from the relay's levels; nothing is decoded here.
 */
@Composable
fun WaveTrack(wave: Wave, progress: () -> Float, modifier: Modifier = Modifier) {
    val played = MaterialTheme.colorScheme.primary
    val rest = MaterialTheme.colorScheme.onSurfaceVariant
    val b = remember(wave) { Bars(wave) }
    Canvas(modifier.fillMaxWidth().height(32.dp)) {
        val step = 3.dp.toPx()
        val n = (size.width / step).toInt()
        b.fit(n)
        val cut = size.width * progress().coerceIn(0f, 1f)
        val mid = size.height / 2
        val w = 2.dp.toPx()
        for (i in 0 until n) {
            val x = i * step
            val c = if (x + w / 2 <= cut) played else rest
            val p = maxOf(1f, b.peaks[i] * size.height)
            val r = maxOf(1f, b.rms[i] * size.height)
            drawRect(c.copy(alpha = c.alpha * 0.4f), Offset(x, mid - p / 2), Size(w, p))
            drawRect(c, Offset(x, mid - r / 2), Size(w, r))
        }
    }
}
