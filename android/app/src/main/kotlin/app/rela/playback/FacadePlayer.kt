package app.rela.playback

import android.os.Looper
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.json
import app.rela.core.text
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import uniffi.rela.PlayerState
import uniffi.rela.SubwaveState
import uniffi.rela.SyncState

/** What the media session shows: the core's state, read through [facadeView] (tested on the JVM). */
data class FacadeView(
    val playing: Boolean,
    /** prev?, current, next? — the notification draws prev/next only when there are items there. */
    val items: List<FacadeItem>,
    val current: Int,
    val positionMs: Long,
    val seekable: Boolean,
)

/** The queue's song that plays, or null while a station plays (the core keeps the queue's index meanwhile). */
fun queueSong(s: PlayerState?, queue: List<Song>): Song? =
    s?.takeIf { json(it.stationJson).isEmpty() }?.let { queue.getOrNull(it.index.toInt()) }

/** The cover that goes with what plays: the song's, or SUB/WAVE's for a station ("sw:" ids). */
fun coverId(s: PlayerState?, sw: SubwaveState?): String {
    if (s == null) return ""
    if (json(s.stationJson).isEmpty()) return json(s.currentJson)["coverArt"].text()
    val id = sw?.takeIf { it.active }?.let { json(it.nowJson)["id"].text() } ?: ""
    return if (id.isEmpty()) "" else "sw:$id"
}

/** Listening together outlives the UI: the service stays in the foreground while following or followed, paused too. */
fun keepAlive(sync: SyncState?) = sync != null && (sync.following.isNotEmpty() || sync.listeners.isNotEmpty())

data class FacadeItem(val uid: String, val title: String, val artist: String, val album: String, val durationMs: Long)

fun facadeView(s: PlayerState?, queue: List<Song>, subwaveTitle: String = ""): FacadeView {
    if (s == null) return FacadeView(false, emptyList(), 0, 0, false)
    val station = json(s.stationJson)
    if (station.isNotEmpty()) {
        val name = station["name"].text()
        val item = FacadeItem("station:" + station["id"].text(), subwaveTitle.ifEmpty { name }, name, "", 0)
        return FacadeView(s.playing, listOf(item), 0, 0, false)
    }
    val i = s.index.toInt()
    if (i !in queue.indices) return FacadeView(false, emptyList(), 0, 0, false)
    fun item(role: String, k: Int) = queue[k].let {
        FacadeItem("$role:$k:${it.id}", it.title, it.artist, it.album, (it.duration * 1000).toLong())
    }
    val items = buildList {
        if (queue.size > 1) add(item("p", if (i > 0) i - 1 else queue.lastIndex)) // prev at 0 restarts; the core decides
        add(item("c", i).let { if (s.duration > 0) it.copy(durationMs = (s.duration * 1000).toLong()) else it })
        if (queue.size > 1) add(item("n", if (i < queue.lastIndex) i + 1 else 0))
    }
    return FacadeView(s.playing, items, if (queue.size > 1) 1 else 0, (s.position.coerceAtLeast(0.0) * 1000).toLong(), true)
}

/**
 * The one Player Media3's session needs (notification, lock screen, headset, Bluetooth). It plays nothing: its
 * state is the core's, its commands go to the core, which keeps deciding what plays on the two decks.
 */
@OptIn(UnstableApi::class)
class FacadePlayer : SimpleBasePlayer(Looper.getMainLooper()) {
    private var view = FacadeView(false, emptyList(), 0, 0, false)
    private var artwork: ByteArray? = null

    /** On the main thread, whenever the core said something. */
    fun update(next: FacadeView, art: ByteArray?) {
        view = next
        artwork = art
        invalidateState()
    }

    override fun getState(): State {
        val v = view
        val commands = Player.Commands.Builder().addAll(
            COMMAND_PLAY_PAUSE, COMMAND_STOP, COMMAND_GET_CURRENT_MEDIA_ITEM, COMMAND_GET_METADATA,
            COMMAND_GET_TIMELINE, COMMAND_SEEK_TO_NEXT, COMMAND_SEEK_TO_NEXT_MEDIA_ITEM, COMMAND_SEEK_TO_PREVIOUS,
            COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM, COMMAND_SEEK_TO_MEDIA_ITEM,
        ).apply { if (v.seekable) add(COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM) }.build()
        val b = State.Builder()
            .setAvailableCommands(commands)
            .setPlayWhenReady(v.playing, PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
            .setPlaybackState(if (v.items.isEmpty()) STATE_IDLE else STATE_READY)
        if (v.items.isNotEmpty()) {
            b.setPlaylist(v.items.mapIndexed { k, it ->
                val md = MediaMetadata.Builder().setTitle(it.title).setArtist(it.artist).setAlbumTitle(it.album)
                if (k == v.current) artwork?.let { md.setArtworkData(it, MediaMetadata.PICTURE_TYPE_FRONT_COVER) }
                MediaItemData.Builder(it.uid)
                    .setMediaItem(MediaItem.Builder().setMediaId(it.uid).build())
                    .setMediaMetadata(md.build())
                    .setDurationUs(if (it.durationMs > 0) it.durationMs * 1000 else androidx.media3.common.C.TIME_UNSET)
                    .setIsSeekable(v.seekable)
                    .build()
            })
            b.setCurrentMediaItemIndex(v.current)
            b.setContentPositionMs(PositionSupplier.getExtrapolating(v.positionMs, if (v.playing) 1f else 0f))
        }
        return b.build()
    }

    private fun done(): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        if (playWhenReady != view.playing) Rela.core?.toggle()
        return done()
    }

    override fun handleStop(): ListenableFuture<*> {
        if (view.playing) Rela.core?.toggle()
        return done()
    }

    override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> {
        val core = Rela.core ?: return done()
        when {
            seekCommand == COMMAND_SEEK_TO_NEXT || seekCommand == COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> core.next()
            seekCommand == COMMAND_SEEK_TO_PREVIOUS || seekCommand == COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> core.prev()
            mediaItemIndex > view.current -> core.next()
            mediaItemIndex < view.current -> core.prev()
            else -> core.seek(positionMs / 1000.0)
        }
        return done()
    }

    override fun handlePrepare(): ListenableFuture<*> = done()

    override fun handleRelease(): ListenableFuture<*> = done()
}
