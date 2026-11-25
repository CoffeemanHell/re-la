package app.rela.playback

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import androidx.annotation.OptIn
import androidx.core.content.ContextCompat
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import app.rela.MainActivity
import app.rela.core.Rela
import app.rela.core.json
import app.rela.core.text
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

/**
 * Owns the core while the app runs: made when the UI (or a media button) connects, stopped when nothing plays, nobody
 * follows and the UI is gone. Media3 keeps it in the foreground with the notification while the facade plays, and
 * [onUpdateNotification] while listening together.
 */
@OptIn(UnstableApi::class)
class RelaService : MediaSessionService() {
    private val scope = MainScope()
    private lateinit var decks: ExoDecks
    private lateinit var facade: FacadePlayer
    private lateinit var session: MediaSession
    private lateinit var audio: AudioManager
    private val art = MutableStateFlow<Pair<String, ByteArray>?>(null) // cover id → bytes
    private val sound = SoundLogic(pause = { Rela.core?.pauseHere() }, mute = { decks.mute(it) })

    private val focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build(),
        )
        .setOnAudioFocusChangeListener(::focusChanged)
        .build()

    private val playing get() = Rela.player.value?.playing == true
    private val following get() = !Rela.sync.value?.following.isNullOrEmpty()

    private fun focusChanged(change: Int) {
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS -> lost(passing = false)
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> lost(passing = true)
            AudioManager.AUDIOFOCUS_GAIN -> if (!sound.gained(playing, following)) audio.abandonAudioFocusRequest(focus)
        }
    }

    private fun lost(passing: Boolean) {
        if (!playing) return
        val station = json(Rela.player.value?.stationJson ?: "")["id"].text()
        sound.lost(passing, following) { if (station.isEmpty()) Rela.core?.toggle() else Rela.core?.playStation(station) }
    }

    /** Headphones out: pause, as every player does. */
    private val noisy = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            lost(passing = false)
        }
    }
    private var noisyOn = false

    /** Wi-Fi or mobile data: the transcode setting follows the network. */
    private val network = object : ConnectivityManager.NetworkCallback() {
        override fun onCapabilitiesChanged(n: Network, c: NetworkCapabilities) {
            Rela.core?.setMobile(c.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR))
        }
    }

    override fun onCreate() {
        super.onCreate()
        audio = getSystemService(AudioManager::class.java)
        decks = ExoDecks(this) { k, note -> Rela.core?.deckEvent(k, note) }
        Rela.start(this, decks)
        getSystemService(ConnectivityManager::class.java).registerDefaultNetworkCallback(network)
        facade = FacadePlayer()
        session = MediaSession.Builder(this, facade)
            .setSessionActivity(
                PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE),
            )
            .build()
        scope.launch {
            combine(Rela.player, Rela.queue, Rela.subwave, art) { s, q, sw, a ->
                val view = facadeView(s, q, sw?.takeIf { it.active }?.let { json(it.nowJson)["title"].text() } ?: "")
                view to a?.takeIf { it.first == coverId(s, sw) }?.second // never the last song's
            }.collect { (view, bytes) -> facade.update(view, bytes) }
        }
        scope.launch {
            Rela.player.map { it?.playing == true }.distinctUntilChanged().collect(::playingChanged)
        }
        scope.launch { Rela.sync.map(::keepAlive).distinctUntilChanged().collect { triggerNotificationUpdate() } }
        scope.launch {
            // the notification's cover: fetched through the core's cache when the song changes
            combine(Rela.player, Rela.subwave, ::coverId).distinctUntilChanged().collectLatest { id ->
                art.value = if (id.isEmpty()) null else Rela.core?.cover(id, 512u)?.let { id to it }
            }
        }
    }

    private fun playingChanged(playing: Boolean) {
        if (playing) {
            sound.playing(focus = audio.requestAudioFocus(focus) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
            if (!noisyOn) {
                ContextCompat.registerReceiver(this, noisy, IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY), ContextCompat.RECEIVER_NOT_EXPORTED)
                noisyOn = true
            }
        } else if (noisyOn) {
            unregisterReceiver(noisy)
            noisyOn = false
            if (!sound.waiting) audio.abandonAudioFocusRequest(focus)
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

    /** Media3 keeps the foreground while playing; listening together keeps it paused too (the UI may be gone). */
    override fun onUpdateNotification(session: MediaSession, startInForegroundRequired: Boolean) =
        super.onUpdateNotification(session, startInForegroundRequired || keepAlive(Rela.sync.value))

    /** Swiped away from recents: the app closes, music and listening together too. */
    override fun onTaskRemoved(rootIntent: Intent?) {
        lost(passing = false)
        stopSelf()
    }

    override fun onDestroy() {
        scope.cancel()
        getSystemService(ConnectivityManager::class.java).unregisterNetworkCallback(network)
        if (noisyOn) unregisterReceiver(noisy)
        audio.abandonAudioFocusRequest(focus)
        session.release()
        facade.release()
        Rela.stop()
        decks.release()
        super.onDestroy()
    }
}
