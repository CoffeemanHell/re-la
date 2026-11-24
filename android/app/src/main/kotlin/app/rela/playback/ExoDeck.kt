package app.rela.playback

import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.annotation.OptIn
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.TrackSelectionParameters.AudioOffloadPreferences
import androidx.media3.common.Tracks
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlaybackException
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.analytics.AnalyticsListener
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.audio.DefaultAudioSink
import uniffi.rela.DeckFactory
import uniffi.rela.DeckNote
import uniffi.rela.DeckOut

/**
 * One of the core's two decks: an ExoPlayer that only plays what it is told. The core calls from its own thread, so
 * every call hops to the main looper, where the player lives. Audio focus is the service's (two decks, one app).
 */
/** The audio renderers; [float]: 32-bit float PCM to the AudioTrack where the device takes it. */
@OptIn(UnstableApi::class)
private fun renderers(context: Context, float: Boolean) = object : DefaultRenderersFactory(context) {
    override fun buildAudioSink(context: Context, enableFloatOutput: Boolean, enableAudioTrackPlaybackParams: Boolean): AudioSink =
        DefaultAudioSink.Builder(context).setEnableFloatOutput(float).setEnableAudioTrackPlaybackParams(enableAudioTrackPlaybackParams).build()
}

@OptIn(UnstableApi::class)
class ExoDeck(context: Context, private val index: UInt, private val report: (UInt, DeckNote) -> Unit) : DeckOut {
    private val main = Handler(Looper.getMainLooper())
    private val logic = DeckLogic { report(index, it) }
    private var released = false
    private val player: ExoPlayer = ExoPlayer.Builder(context, renderers(context, AudioPrefs.floatOutput(context))).build().apply {
        if (AudioPrefs.offload(context)) trackSelectionParameters = trackSelectionParameters.buildUpon().setAudioOffloadPreferences(
            AudioOffloadPreferences.Builder()
                .setAudioOffloadMode(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_ENABLED)
                .setIsGaplessSupportRequired(true)
                .build(),
        ).build()
        setAudioAttributes(
            AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
            false,
        )
        setWakeMode(C.WAKE_MODE_NETWORK) // screen off: keep the CPU and Wi-Fi up between buffer refills
        addListener(Events())
        addAnalyticsListener(Output())
    }

    private val poll = object : Runnable {
        override fun run() {
            if (released || player.mediaItemCount == 0) return
            logic.position(player.currentPosition)
            main.postDelayed(this, 100)
        }
    }

    private fun on(block: ExoPlayer.() -> Unit) {
        main.post { if (!released) player.block() }
    }

    override fun load(url: String, paused: Boolean) = on {
        setMediaItem(MediaItem.fromUri(url))
        logic.started()
        playWhenReady = !paused
        prepare()
        main.removeCallbacks(poll)
        main.post(poll)
    }

    override fun append(url: String) = on { addMediaItem(MediaItem.fromUri(url)) }

    override fun clearAppended() = on {
        if (mediaItemCount > currentMediaItemIndex + 1) removeMediaItems(currentMediaItemIndex + 1, mediaItemCount)
    }

    override fun dropFirst() = on { if (currentMediaItemIndex > 0) removeMediaItems(0, currentMediaItemIndex) }

    override fun setPaused(paused: Boolean) = on { playWhenReady = !paused }

    override fun seek(seconds: Double) = on { seekTo((seconds * 1000).toLong()) }

    override fun setVolume(volume: Double) = on { this.volume = exoVolume(volume) }

    override fun setSpeed(speed: Double) = on { playbackParameters = PlaybackParameters(speed.toFloat()) }

    override fun stop() = on {
        stop()
        clearMediaItems()
        main.removeCallbacks(poll)
        logic.stopped()
    }

    /** On the main thread, after the core is gone. */
    fun release() {
        released = true
        main.removeCallbacks(poll)
        player.release()
    }

    private inner class Events : Player.Listener {
        override fun onPlaybackStateChanged(state: Int) {
            when (state) {
                Player.STATE_READY -> logic.ready(player.duration)
                Player.STATE_ENDED -> logic.ended()
                else -> Unit
            }
        }

        override fun onMediaItemTransition(item: MediaItem?, reason: Int) {
            if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO) {
                logic.gaplessMove(player.playbackState == Player.STATE_READY, player.duration)
            }
        }

        override fun onTimelineChanged(timeline: androidx.media3.common.Timeline, reason: Int) {
            logic.durationChanged(player.duration)
        }

        override fun onPlayerError(error: PlaybackException) {
            val message = error.message ?: error.errorCodeName
            // a prepared-ahead (appended) item can fail while the current one still plays
            val at = (error as? ExoPlaybackException)?.mediaPeriodId?.periodUid
            val t = player.currentTimeline
            val ahead = at != null && !t.isEmpty && at != t.getUidOfPeriod(player.currentPeriodIndex)
            if (ahead) logic.failedAhead(message) else logic.failed(message)
        }

        override fun onTracksChanged(tracks: Tracks) {
            val f = tracks.groups.firstOrNull { it.type == C.TRACK_TYPE_AUDIO && it.isSelected }?.getTrackFormat(0) ?: return
            report(index, DeckNote.Prop("audio-params/format", codecName(f.sampleMimeType)))
            if (f.sampleRate > 0) report(index, DeckNote.Prop("audio-params/samplerate", f.sampleRate.toString()))
            if (f.channelCount > 0) report(index, DeckNote.Prop("audio-params/channel-count", f.channelCount.toString()))
            val bitrate = if (f.averageBitrate > 0) f.averageBitrate else f.bitrate
            report(index, DeckNote.Bitrate(bitrate.takeIf { it > 0 }?.toDouble()))
        }
    }

    /** What reaches AudioTrack: the signal path's right-hand side. */
    private inner class Output : AnalyticsListener {
        override fun onAudioTrackInitialized(eventTime: AnalyticsListener.EventTime, config: AudioSink.AudioTrackConfig) {
            report(index, DeckNote.Prop("audio-out-params/format", pcmName(config.encoding)))
            report(index, DeckNote.Prop("audio-out-params/samplerate", config.sampleRate.toString()))
            report(index, DeckNote.Prop("current-ao", "AudioTrack"))
        }
    }
}

/** The two decks Core asks for; made on the main thread (the service's onCreate). */
class ExoDecks(private val context: Context, private val report: (UInt, DeckNote) -> Unit) : DeckFactory {
    val decks = mutableListOf<ExoDeck>()

    override fun makeDeck(index: UInt): DeckOut = ExoDeck(context, index, report).also { decks += it }

    fun release() = decks.forEach { it.release() }
}
