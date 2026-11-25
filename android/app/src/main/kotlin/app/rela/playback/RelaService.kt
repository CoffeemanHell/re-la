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
    private var art: Pair<String, ByteArray>? = null
    private var focusLostWhilePlaying = false

    private val focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build(),
        )
        .setOnAudioFocusChangeListener { change ->
            val playing = Rela.player.value?.playing == true
            when (change) {
                AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> if (playing) {
                    focusLostWhilePlaying = change == AudioManager.AUDIOFOCUS_LOSS_TRANSIENT
                    Rela.core?.toggle()
                }
                AudioManager.AUDIOFOCUS_GAIN -> if (focusLostWhilePlaying && !playing) {
                    focusLostWhilePlaying = false
                    Rela.core?.toggle()
                }
            }
        }
        .build()

    /** Headphones out: pause, as every player does. */
    private val noisy = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (Rela.player.value?.playing == true) Rela.core?.toggle()
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
            combine(Rela.player, Rela.queue, Rela.subwave) { s, q, sw ->
                facadeView(s, q, sw?.takeIf { it.active }?.let { json(it.nowJson)["title"].text() } ?: "")
            }.collect { facade.update(it, art) }
        }
        scope.launch {
            Rela.player.map { it?.playing == true }.distinctUntilChanged().collect(::playingChanged)
        }
        scope.launch { Rela.sync.map(::keepAlive).distinctUntilChanged().collect { triggerNotificationUpdate() } }
        scope.launch {
            // the notification's cover: fetched through the core's cache when the song changes
            Rela.player.map { it?.let { s -> json(s.currentJson)["coverArt"].text() } ?: "" }.distinctUntilChanged().collect { id ->
                art = if (id.isEmpty()) null else Rela.core?.cover(id, 512u)?.let { id to it }
                facade.update(facadeView(Rela.player.value, Rela.queue.value), art)
            }
        }
    }

    private fun playingChanged(playing: Boolean) {
        if (playing) {
            focusLostWhilePlaying = false // playing again, by hand or by us: no resume is owed any more
            audio.requestAudioFocus(focus)
            if (!noisyOn) {
                ContextCompat.registerReceiver(this, noisy, IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY), ContextCompat.RECEIVER_NOT_EXPORTED)
                noisyOn = true
            }
        } else if (noisyOn) {
            unregisterReceiver(noisy)
            noisyOn = false
            if (!focusLostWhilePlaying) audio.abandonAudioFocusRequest(focus)
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

    /** Media3 keeps the foreground while playing; listening together keeps it paused too (the UI may be gone). */
    override fun onUpdateNotification(session: MediaSession, startInForegroundRequired: Boolean) =
        super.onUpdateNotification(session, startInForegroundRequired || keepAlive(Rela.sync.value))

    /** Swiped away from recents: the app closes, music and listening together too. */
    override fun onTaskRemoved(rootIntent: Intent?) {
        if (Rela.player.value?.playing == true) Rela.core?.toggle()
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
