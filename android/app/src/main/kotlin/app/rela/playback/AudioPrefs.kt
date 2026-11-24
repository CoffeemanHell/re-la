package app.rela.playback

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager

/**
 * The experimental audio switches. They live on the device (they depend on its DAC and offload support), not in the
 * core's options, and are read when a deck builds its player: the decks exist before the core hands options to Kotlin.
 */
object AudioPrefs {
    private fun prefs(ctx: Context) = ctx.getSharedPreferences("audio", Context.MODE_PRIVATE)
    fun floatOutput(ctx: Context) = prefs(ctx).getBoolean("floatOutput", false)
    fun offload(ctx: Context) = prefs(ctx).getBoolean("audioOffload", false) && offloadSupported()

    /** Whether the platform offloads any codec Media3 can hand it (it never offloads FLAC); many phones say no. */
    fun offloadSupported(): Boolean {
        val attrs = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build()
        val codecs = listOf(AudioFormat.ENCODING_MP3, AudioFormat.ENCODING_AAC_LC, AudioFormat.ENCODING_OPUS)
        return codecs.any {
            val f = AudioFormat.Builder().setEncoding(it).setSampleRate(48000).setChannelMask(AudioFormat.CHANNEL_OUT_STEREO).build()
            AudioManager.isOffloadedPlaybackSupported(f, attrs)
        }
    }
    fun set(ctx: Context, key: String, on: Boolean) = prefs(ctx).edit().putBoolean(key, on).apply()
}
