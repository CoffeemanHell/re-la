package app.rela.playback

import kotlin.math.abs
import kotlin.math.pow
import uniffi.rela.DeckEnd
import uniffi.rela.DeckNote

/** The core speaks mpv: volume is 0..100 on mpv's cubic scale, ExoPlayer's is linear 0..1. */
fun exoVolume(v: Double): Float = (v / 100.0).coerceIn(0.0, 1.0).pow(3).toFloat()

/** A codec as mpv names it in audio-params/format ("audio/flac" → "flac"). */
fun codecName(mime: String?): String = when (mime) {
    null -> ""
    "audio/mpeg" -> "mp3"
    "audio/mp4a-latm" -> "aac"
    "audio/raw" -> "pcm"
    else -> mime.substringAfter('/').removePrefix("x-")
}

/** An AudioFormat encoding as mpv's sample format names ("s16", "float", …). */
fun pcmName(encoding: Int): String = when (encoding) {
    2 -> "s16" // ENCODING_PCM_16BIT
    3 -> "u8" // ENCODING_PCM_8BIT
    4 -> "float" // ENCODING_PCM_FLOAT
    21 -> "s24" // ENCODING_PCM_24BIT_PACKED
    22 -> "s32" // ENCODING_PCM_32BIT
    else -> "enc$encoding"
}

/** What the track is, as mpv's properties; null for what is not known (no track: all of it), so the last one's goes. */
fun trackNotes(mime: String?, sampleRate: Int, channels: Int, bitrate: Int): List<DeckNote> {
    val codec = codecName(mime).ifEmpty { null }
    fun known(n: Int) = n.takeIf { it > 0 }
    return listOf(
        DeckNote.Prop("audio-params/format", codec),
        DeckNote.Prop("audio-codec-name", codec),
        DeckNote.Prop("audio-params/samplerate", known(sampleRate)?.toString()),
        DeckNote.Prop("audio-params/channel-count", known(channels)?.toString()),
        DeckNote.Bitrate(known(bitrate)?.toDouble()),
    )
}

/**
 * What one ExoPlayer saw, told the way libmpv tells it (crates/mpv), since the core's gapless and crossfade logic
 * reads mpv's event order: StartFile on load, FileLoaded + Duration once it can play, a gapless move as
 * EndFile(Eof) then StartFile, positions only when they moved.
 */
class DeckLogic(private val report: (DeckNote) -> Unit) {
    private var loaded = false
    private var duration = 0.0
    private var position: Double? = null

    /** load(): a new file replaces the deck's. */
    fun started() {
        loaded = false
        duration = 0.0
        position = null
        report(DeckNote.StartFile)
    }

    /** The player can play the current item (state READY, or right after a gapless move into a ready item). */
    fun ready(durationMs: Long) {
        durationChanged(durationMs)
        if (!loaded) {
            loaded = true
            report(DeckNote.FileLoaded)
        }
    }

    fun durationChanged(durationMs: Long) {
        val d = if (durationMs > 0) durationMs / 1000.0 else 0.0
        if (d > 0 && d != duration) {
            duration = d
            report(DeckNote.Duration(d))
        }
    }

    /** ExoPlayer moved to the appended item by itself. */
    fun gaplessMove(readyNow: Boolean, durationMs: Long) {
        report(DeckNote.EndFile(DeckEnd.EOF, ""))
        started()
        if (readyNow) ready(durationMs)
    }

    fun ended() = report(DeckNote.EndFile(DeckEnd.EOF, ""))

    /** The current item failed with another one appended: mpv goes on with that one, so ExoPlayer is told to. */
    fun failedIntoNext(message: String) {
        failed(message)
        started()
    }

    fun failed(message: String) = report(DeckNote.EndFile(DeckEnd.ERROR, message))

    /** The appended item failed before ExoPlayer moved to it; mpv moves first, then fails, and the core skips it. */
    fun failedAhead(message: String) {
        gaplessMove(readyNow = false, durationMs = 0)
        failed(message)
    }

    fun stopped() {
        loaded = false
        position = null
        report(DeckNote.Position(null))
    }

    /** Polled while the deck holds a file; mpv sends time-pos every frame, a 10 ms change is enough here. */
    fun position(ms: Long) {
        val p = ms.coerceAtLeast(0) / 1000.0
        val last = position
        if (last == null || abs(p - last) >= 0.01) {
            position = p
            report(DeckNote.Position(p))
        }
    }
}
