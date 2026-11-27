package app.rela.core

import android.content.Context
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import uniffi.rela.Core
import uniffi.rela.CoreListener
import uniffi.rela.DeckFactory
import uniffi.rela.PlayerChanges
import uniffi.rela.PlayerState
import uniffi.rela.SubwaveState
import uniffi.rela.SyncNote
import uniffi.rela.SyncState
import uniffi.rela.setLanguage

/**
 * The Rust core of this process and what it last said. RelaService starts it and stops it; screens read the flows
 * and call [core] (or [get]/[set] for Subsonic calls through the core's guards).
 */
object Rela {
    private var running: Core? = null

    private val _player = MutableStateFlow<PlayerState?>(null)
    val player = _player.asStateFlow()
    private val _queue = MutableStateFlow<List<Song>>(emptyList())
    val queue = _queue.asStateFlow()
    private val _sync = MutableStateFlow<SyncState?>(null)
    val sync = _sync.asStateFlow()
    private val _syncNotes = MutableSharedFlow<SyncNote>(extraBufferCapacity = 16)
    val syncNotes = _syncNotes.asSharedFlow()
    private val _subwave = MutableStateFlow<SubwaveState?>(null)
    val subwave = _subwave.asStateFlow()

    private val replies = ConcurrentHashMap<ULong, CompletableDeferred<String?>>()
    private val ids = AtomicLong(0)

    /** Null while the service isn't running. */
    val core: Core? get() = running

    /** Certificates are Android's to judge: the core needs the app's context for it, before it first connects. */
    private external fun initTls(context: Context)

    @Synchronized
    fun start(context: Context, decks: DeckFactory): Core = running ?: run {
        System.loadLibrary("rela") // for initTls: JNA, which makes the other calls, loads it for itself only
        initTls(context.applicationContext)
        setLanguage(context.resources.configuration.locales[0].language) // the app's language (per-app or the system's)
        Core(context.filesDir.path, context.cacheDir.path, decks, Listener).also { running = it; it.refresh() }
    }

    /** Saves the queue, says bye to followers, frees the core. */
    @Synchronized
    fun stop() {
        val c = running ?: return
        running = null
        c.persistNow()
        c.stopSync(true)
        c.destroy()
        replies.values.forEach { it.cancel() }
        replies.clear()
    }

    suspend fun get(method: String, params: Map<String, Any?> = emptyMap()) = call(method, params, false)
    suspend fun set(method: String, params: Map<String, Any?> = emptyMap()) = call(method, params, true)

    /** The core's own answers (djState, djFeedback): any JSON, offline too. */
    suspend fun local(method: String): JsonElement? = raw(method, emptyMap(), false)?.let { runCatching { Json.parseToJsonElement(it) }.getOrNull() }

    /** The `subsonic-response` object, or null when the call failed (the core shows why in status). */
    private suspend fun call(method: String, params: Map<String, Any?>, write: Boolean): JsonObject? =
        raw(method, params, write)?.let { Json.parseToJsonElement(it) as? JsonObject }

    private suspend fun raw(method: String, params: Map<String, Any?>, write: Boolean): String? {
        val c = running ?: return null
        val id = ids.incrementAndGet().toULong()
        val reply = CompletableDeferred<String?>()
        replies[id] = reply
        try {
            if (!c.request(id, method, paramsJson(params), write)) return null
            return reply.await()
        } finally {
            replies.remove(id) // a screen that went away: its reply is dropped
        }
    }

    private object Listener : CoreListener {
        override fun onPlayer(changes: PlayerChanges, state: PlayerState, queueJson: String?) {
            if (queueJson != null) _queue.value = Song.list(Json.parseToJsonElement(queueJson))
            _player.value = state
        }

        override fun onReply(id: ULong, json: String, call: Boolean) {
            val r = replies.remove(id) ?: return
            if (call) r.complete(json.takeIf { it != "null" }) else r.cancel()
        }

        override fun onSync(note: SyncNote, state: SyncState) {
            _sync.value = state
            _syncNotes.tryEmit(note)
        }

        override fun onSubwave(state: SubwaveState) {
            _subwave.value = state
        }

        override fun onSubwaveTick(elapsedMs: Long) {
            _subwave.value = _subwave.value?.copy(elapsedMs = elapsedMs) // same thread as onSubwave
        }
    }
}

/** Kotlin values (String, Number, Boolean, List of those) as the JSON object the core's request takes. */
fun paramsJson(params: Map<String, Any?>): String = JsonObject(params.mapValues { (_, v) -> toJson(v) }).toString()

private fun toJson(v: Any?): JsonElement = when (v) {
    null -> JsonNull
    is JsonElement -> v
    is String -> kotlinx.serialization.json.JsonPrimitive(v)
    is Number -> kotlinx.serialization.json.JsonPrimitive(v)
    is Boolean -> kotlinx.serialization.json.JsonPrimitive(v)
    is List<*> -> kotlinx.serialization.json.JsonArray(v.map(::toJson))
    else -> kotlinx.serialization.json.JsonPrimitive(v.toString())
}

fun json(text: String): JsonObject = (runCatching { Json.parseToJsonElement(text) }.getOrNull() as? JsonObject) ?: JsonObject(emptyMap())
