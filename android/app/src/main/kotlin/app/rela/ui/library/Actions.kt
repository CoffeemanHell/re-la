package app.rela.ui.library

import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.heightIn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.foundation.clickable
import app.rela.R
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.listOf
import app.rela.core.obj
import app.rela.core.text
import kotlinx.coroutines.CoroutineScope
import androidx.compose.runtime.currentCompositeKeyHash
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject

/** What a screen fetched. [value] stays while a refresh runs; [failed] when the last fetch failed. */
class Loaded<T>(val value: T?, val loading: Boolean, val failed: Boolean, val refresh: () -> Unit)

/** What a screen keeps while it is on the back stack, one per nav entry; cleared when the entry is popped. */
class Kept : ViewModel() {
    val values = mutableMapOf<String, Any>()
}

/** [make] once per call site and [keys] in this nav entry: back to a screen finds what it had, no reload. */
@Composable
private fun <T : Any> kept(vararg keys: Any?, make: (CoroutineScope) -> T): T {
    val vm = viewModel { Kept() }
    val k = "$currentCompositeKeyHash:${keys.toList()}" // the call site, as rememberSaveable keys it
    @Suppress("UNCHECKED_CAST")
    return remember(k) { vm.values.getOrPut(k) { make(vm.viewModelScope) } as T }
}

private class LoadState<T> {
    var value by mutableStateOf<T?>(null)
    var loading by mutableStateOf(true)
    var failed by mutableStateOf(false)
    var gen by mutableIntStateOf(0)
    var done = -1 // the gen last fetched to the end; a screen gone mid-fetch fetches again when back
}

/** Runs [fetch] when the screen first shows (again for new [keys]) and on refresh. A gone screen drops its reply. */
@Composable
fun <T : Any> load(vararg keys: Any?, fetch: suspend () -> T?): Loaded<T> {
    val st = kept(*keys) { LoadState<T>() }
    LaunchedEffect(st, st.gen) {
        if (st.done == st.gen) return@LaunchedEffect
        st.loading = true
        val v = fetch()
        st.failed = v == null
        if (v != null) st.value = v
        st.loading = false
        st.done = st.gen
    }
    return Loaded(st.value, st.loading, st.failed) { st.gen++ }
}

/** A list fetched [size] at a time while scrolling (albums, tracks). */
class Pager<T>(private val scope: CoroutineScope, private val size: Int, private val page: suspend (offset: Int) -> List<T>?) {
    val items = mutableStateListOf<T>()
    var loading by mutableStateOf(false)
        private set
    var failed by mutableStateOf(false)
        private set
    private var hasMore = true
    private var job: Job? = null

    fun next() {
        if (loading || !hasMore) return
        loading = true
        failed = false
        job = scope.launch {
            val l = page(items.size)
            if (l == null) failed = true else {
                items += l
                hasMore = l.size >= size
            }
            loading = false
        }
    }

    fun reset() {
        job?.cancel()
        items.clear()
        hasMore = true
        loading = false
        next()
    }
}

@Composable
fun <T> pager(vararg keys: Any?, size: Int, page: suspend (offset: Int) -> List<T>?): Pager<T> {
    val p = kept(*keys) { scope -> Pager(scope, size, page) }
    LaunchedEffect(p) { if (p.items.isEmpty()) p.next() }
    return p
}

/** Not following, or the leader lets joiners control: what plays may change from here. */
@Composable
fun canControl(): Boolean = Rela.sync.collectAsStateWithLifecycle().value?.canControl ?: true

private fun songsJson(s: List<Song>) = JsonArray(s.map { it.json }).toString()

private fun allowed() = Rela.sync.value?.canControl ?: true

fun play(songs: List<Song>, start: Int = 0, shuffle: Boolean = false) {
    if (songs.isEmpty() || !allowed()) return
    if (shuffle) Rela.core?.playJson(songsJson(songs.shuffled()), 0)
    else Rela.core?.playJson(songsJson(songs), start.toLong())
}

/** where: 0 play now, 1 next, 2 end (Player.add). */
fun add(songs: List<Song>, where: Int) {
    if (songs.isNotEmpty() && allowed()) Rela.core?.addJson(songsJson(songs), where)
}

fun Context.say(text: String) = Toast.makeText(this, text, Toast.LENGTH_SHORT).show()

/** `reply[outer][inner]` as a list (getAlbumList2 → albumList2.album); null when the call failed. */
suspend fun getList(method: String, params: Map<String, Any?>, outer: String, inner: String): List<JsonObject>? =
    Rela.get(method, params)?.let { it[outer].obj().listOf(inner) }

/** `reply[outer]`; null when the call failed. */
suspend fun getObj(method: String, params: Map<String, Any?>, outer: String): JsonObject? =
    Rela.get(method, params)?.let { it[outer].obj() }

/** The songs behind a song, album or playlist; null when the call failed. */
suspend fun songsOf(kind: String, item: JsonObject): List<Song>? = when (kind) {
    "album" -> getList("getAlbum", mapOf("id" to item["id"].text()), "album", "song")?.map(::Song)
    "playlist" -> getList("getPlaylist", mapOf("id" to item["id"].text()), "playlist", "entry")?.map(::Song)
    else -> listOf(Song(item))
}

/** star/unstar; true when the server took it. */
suspend fun star(kind: String, id: String, on: Boolean): Boolean {
    val key = when (kind) {
        "album" -> "albumId"
        "artist" -> "artistId"
        else -> "id"
    }
    return Rela.set(if (on) "star" else "unstar", mapOf(key to id)) != null
}

/** createShare, then the system share sheet with the link (a public share link, no login in it). */
suspend fun share(ctx: Context, id: String) {
    val url = Rela.set("createShare", mapOf("id" to id))?.get("shares").obj().listOf("share").firstOrNull()?.get("url").text()
    if (url.isNullOrEmpty()) return ctx.say("İşlem başarısız")
    ctx.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, url), null))
}

/** Favorite state of an item: flips at once, flips back and says so when the server refuses. */
class Fav(private val kind: String, private val id: String, initial: Boolean, private val scope: CoroutineScope, private val ctx: Context) {
    var on by mutableStateOf(initial)
        private set

    fun toggle(quiet: Boolean = false) {
        val was = on
        on = !was
        scope.launch {
            if (!star(kind, id, !was)) {
                on = was
                ctx.say("İşlem başarısız")
            } else if (!quiet) ctx.say(if (was) "Favorilerden çıkarıldı" else "Favorilere eklendi")
        }
    }
}

@Composable
fun rememberFav(kind: String, item: JsonObject): Fav {
    val scope = rememberCoroutineScope()
    val ctx = LocalContext.current
    return remember(item) { Fav(kind, item["id"].text(), item["starred"] != null, scope, ctx) }
}

/**
 * The ⋮ menu of a song, album, artist or playlist (the desktop's ItemMenu). Playing follows [canControl];
 * writes (favorite, rating, playlists, share) are left to the server. [remove]: a playlist row's "Playlist'ten çıkar".
 * [trigger] draws the button; [extra] items come first (Now Playing's own: crossfade, DJ).
 */
@Composable
fun ItemMenu(
    kind: String, item: JsonObject, remove: (() -> Unit)? = null, fav: Fav = rememberFav(kind, item),
    trigger: @Composable (open: () -> Unit) -> Unit = { IconButton(it) { Icon(painterResource(R.drawable.ic_more_vert), "Diğer") } },
    extra: @Composable ColumnScope.(close: () -> Unit) -> Unit = {},
) {
    var open by remember { mutableStateOf(false) }
    var dialog by remember { mutableStateOf("") }
    val scope = rememberCoroutineScope()
    val ctx = LocalContext.current
    val ctl = canControl()
    val isSong = kind == "song"
    val id = item["id"].text()

    fun withSongs(f: (List<Song>) -> Unit) = scope.launch {
        val s = songsOf(kind, item)
        if (s == null) ctx.say("İşlem başarısız") else if (s.isNotEmpty()) f(s)
    }

    @Composable
    fun Act(text: String, icon: Int, enabled: Boolean = true, onClick: () -> Unit) =
        DropdownMenuItem(text = { Text(text) }, leadingIcon = { Icon(painterResource(icon), null) }, enabled = enabled, onClick = { open = false; onClick() })

    Box {
        trigger { open = true }
        DropdownMenu(open, { open = false }) {
            extra { open = false }
            Act("Çal", R.drawable.ic_play_arrow_fill, ctl) {
                when (kind) {
                    "song" -> add(listOf(Song(item)), 0)
                    "artist" -> scope.launch { topSongs(item["name"].text())?.let { play(it) } }
                    else -> withSongs { play(it) }
                }
            }
            if (kind == "album" || kind == "playlist") Act("Karıştır", R.drawable.ic_shuffle, ctl) { withSongs { play(it, shuffle = true) } }
            if (isSong || kind == "album") Act("Sıradaki yap", R.drawable.ic_subdirectory_arrow_right, ctl) { withSongs { add(it, 1) } }
            if (kind != "artist") Act("Sıraya ekle", R.drawable.ic_queue_music, ctl) { withSongs { add(it, 2) } }
            if (isSong) Act("Şarkı radyosu", R.drawable.ic_radio, ctl) {
                scope.launch {
                    val r = Rela.get("getSimilarSongs", mapOf("id" to id, "count" to 50))
                    if (r == null) ctx.say("İşlem başarısız") else play(listOf(Song(item)) + r["similarSongs"].obj().listOf("song").map(::Song))
                }
            }
            if (kind == "artist") Act("Sanatçı radyosu", R.drawable.ic_radio, ctl) { scope.launch { artistRadio(ctx, id) } }
            if (isSong || kind == "album") {
                HorizontalDivider()
                Act("Playlist'e ekle…", R.drawable.ic_playlist_add) { dialog = "playlist" }
                Act("Paylaş…", R.drawable.ic_share_fill) { scope.launch { share(ctx, id) } }
            }
            if (kind != "playlist") {
                HorizontalDivider()
                Act(if (fav.on) "Favorilerden çıkar" else "Favorilere ekle", if (fav.on) R.drawable.ic_star_fill else R.drawable.ic_star) { fav.toggle() }
            }
            if (isSong) Act("Derecelendir…", R.drawable.ic_star) { dialog = "rating" }
            if (remove != null) Act("Playlist'ten çıkar", R.drawable.ic_playlist_remove) { remove() }
        }
    }
    when (dialog) {
        "playlist" -> PlaylistPicker({ dialog = "" }) { pick -> withSongs { s -> scope.launch { pick(s) } } }
        "rating" -> RatingDialog(Song(item).rating, { dialog = "" }) { n ->
            scope.launch { if (Rela.set("setRating", mapOf("id" to id, "rating" to n)) == null) ctx.say("İşlem başarısız") }
        }
    }
}

suspend fun topSongs(artist: String): List<Song>? =
    getList("getTopSongs", mapOf("artist" to artist, "count" to 50), "topSongs", "song")?.map(::Song)

suspend fun artistRadio(ctx: Context, id: String) {
    val r = Rela.get("getSimilarSongs2", mapOf("id" to id, "count" to 50))
    if (r == null) ctx.say("İşlem başarısız") else play(r["similarSongs2"].obj().listOf("song").map(::Song))
}

/**
 * "Playlist'e ekle": the user's own playlists and a new one. [onPick] gets the add for the chosen list; it runs it
 * on the songs once they are known.
 */
@Composable
private fun PlaylistPicker(onDismiss: () -> Unit, onPick: (suspend (List<Song>) -> Unit) -> Unit) {
    val ctx = LocalContext.current
    val user = Rela.player.collectAsStateWithLifecycle().value?.user ?: ""
    val lists = load { getList("getPlaylists", emptyMap(), "playlists", "playlist")?.filter { it["owner"].text() == user } }
    var name by remember { mutableStateOf("") }
    fun done(ok: Boolean, what: String) = ctx.say(if (ok) what else "İşlem başarısız")
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Playlist'e ekle") },
        text = {
            Column {
                Row {
                    OutlinedTextField(name, { name = it }, Modifier.weight(1f), placeholder = { Text("Yeni playlist") }, singleLine = true)
                    IconButton({
                        val n = name.trim()
                        onDismiss()
                        onPick { s -> done(Rela.set("createPlaylist", mapOf("name" to n, "songId" to s.map { it.id })) != null, "Playlist oluşturuldu: $n") }
                    }, enabled = name.isNotBlank()) { Icon(painterResource(R.drawable.ic_add), "Oluştur") }
                }
                when {
                    lists.value == null && lists.loading -> Text("Yükleniyor…")
                    lists.value == null -> Text("Yüklenemedi")
                }
                LazyColumn(Modifier.heightIn(max = 320.dp)) {
                    items(lists.value ?: emptyList()) { p ->
                        val pn = p["name"].text()
                        ListItem(
                            headlineContent = { Text(pn) },
                            modifier = Modifier.clickable {
                                onDismiss()
                                onPick { s ->
                                    val ok = Rela.set("updatePlaylist", mapOf("playlistId" to p["id"].text(), "songIdToAdd" to s.map { it.id })) != null
                                    done(ok, "$pn listesine eklendi")
                                }
                            },
                        )
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onDismiss) { Text("Vazgeç") } },
    )
}

/** 0..5 stars; the current star again clears it. */
@Composable
private fun RatingDialog(current: Int, onDismiss: () -> Unit, onRate: (Int) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Derecelendir") },
        text = {
            Row {
                for (n in 1..5) IconButton({ onDismiss(); onRate(if (n == current) 0 else n) }) {
                    Icon(painterResource(if (n <= current) R.drawable.ic_star_fill else R.drawable.ic_star), "$n yıldız")
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onDismiss) { Text("Vazgeç") } },
    )
}
