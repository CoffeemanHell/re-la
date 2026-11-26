package app.rela.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyGridScope
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.OutlinedIconToggleButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.R
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.length
import app.rela.core.listOf
import app.rela.core.songs
import app.rela.core.text
import app.rela.plural
import app.rela.str
import app.rela.ui.Cover
import app.rela.ui.CoverTheme
import app.rela.ui.LocalLook
import app.rela.ui.LocalNav
import app.rela.ui.Screen
import app.rela.ui.open
import app.rela.ui.sharedCover
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject

/** The cover in the middle, the name, [sub] under it (tappable when [onSub]), a line of facts, a row of buttons. */
@Composable
private fun Hero(
    cover: String, shape: Shape, title: String, info: String, sub: String = "", onSub: (() -> Unit)? = null, share: String = "",
    buttons: @Composable RowScope.() -> Unit,
) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Cover(cover, 320.dp, Modifier.fillMaxWidth(0.65f).aspectRatio(1f).sharedCover(share), shape)
        Text(title, Modifier.padding(top = 16.dp), style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center, maxLines = 2, overflow = TextOverflow.Ellipsis)
        if (sub.isNotEmpty()) Text(
            sub, Modifier.clickable(enabled = onSub != null) { onSub?.invoke() }.padding(top = 2.dp),
            style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary, textAlign = TextAlign.Center,
        )
        if (info.isNotEmpty()) Text(info, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        Row(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically, content = buttons)
    }
}

/** A round outlined button at the ends of the hero's row. */
@Composable
private fun Round(onClick: () -> Unit, enabled: Boolean = true, content: @Composable () -> Unit) =
    OutlinedIconButton(onClick, Modifier.size(56.dp), enabled = enabled, content = content)

/** The wide Oynat in the middle of the hero's row. */
@Composable
private fun RowScope.Wide(onClick: () -> Unit, enabled: Boolean) =
    Button(onClick, Modifier.weight(1f).height(56.dp), enabled = enabled) {
        Icon(painterResource(R.drawable.ic_play_arrow_fill), null, Modifier.size(20.dp))
        Text(str(R.string.play), Modifier.padding(start = 8.dp), style = MaterialTheme.typography.titleMedium)
    }

/** Karıştır, Oynat and, for an album, the favourite star. */
@Composable
private fun RowScope.PlayButtons(songs: List<Song>, fav: Fav? = null) {
    val ctl = canControl() && songs.isNotEmpty()
    Round({ play(songs, shuffle = true) }, ctl) { Icon(painterResource(R.drawable.ic_shuffle), str(R.string.shuffle)) }
    Wide({ play(songs) }, ctl)
    if (fav != null) FavButton(fav)
}

@Composable
private fun FavButton(fav: Fav) = OutlinedIconToggleButton(fav.on, { fav.toggle(quiet = true) }, Modifier.size(56.dp)) {
    Icon(painterResource(if (fav.on) R.drawable.ic_star_fill else R.drawable.ic_star), str(if (fav.on) R.string.remove_favorite else R.string.add_favorite))
}

/** getAlbum: cover, play / shuffle / favorite, songs with disc headers on multi-disc albums. */
@Composable
fun Album(id: String, from: String = "") {
    val nav = LocalNav.current
    val l = load(id) { getObj("getAlbum", mapOf("id" to id), "album") }
    val a = l.value ?: remember(id) { peeks["album:$id"] } // the card's copy until the album is here
    val songs = remember(a) { a?.songs("song") ?: emptyList() }
    val multi = songs.map { it.disc }.distinct().size > 1
    val fav = a?.let { rememberFav("album", it) }
    CoverTheme(a?.get("coverArt").text()) { Page("", refreshing = l.loading && l.value != null, onRefresh = l.refresh, compact = true, actions = {
        if (a != null) ItemMenu("album", a, fav = fav!!)
    }) { pad ->
        Listing(if (l.value == null && (a == null || l.failed)) 0 else 1, l.loading, l.failed, "", l.refresh) {
            if (a != null) LazyColumn(contentPadding = pad) {
                item {
                    val facts = listOf(a["genre"].text(), a["year"].text()).filter { it.isNotEmpty() }
                    val artistId = a["artistId"].text()
                    Hero(
                        a["coverArt"].text(), coverShape(LocalLook.current.coverShape), a["name"].text(), facts.joinToString(" • "),
                        sub = a["artist"].text(), onSub = if (artistId.isEmpty()) null else ({ nav.open(Screen.Artist(artistId)) }),
                        share = albumCover(id, from),
                    ) { PlayButtons(songs, fav) }
                }
                songs.forEachIndexed { i, s ->
                    if (multi && (i == 0 || songs[i - 1].disc != s.disc)) item { Header("Disk ${s.disc}") }
                    item { SongRow(s, { play(songs, i) }, number = if (s.track > 0) "${s.track}" else "${i + 1}", card = true, albumArtist = a["artist"].text()) }
                }
            }
        }
    } }
}

/** getArtist + getTopSongs: play the popular songs, artist radio, the popular five, the albums newest first. */
@Composable
fun Artist(id: String, from: String = "") {
    val ctx = LocalContext.current
    val scope = rememberCoroutineScope()
    val l = load(id) { getObj("getArtist", mapOf("id" to id), "artist")?.let { it to (topSongs(it["name"].text()) ?: emptyList()) } }
    val (loaded, top) = l.value ?: (null to emptyList())
    val a = loaded ?: remember(id) { peeks["artist:$id"] } // the row's copy until the artist is here
    val albums = remember(a) { a?.listOf("album")?.sortedByDescending { it["year"].text().toIntOrNull() ?: 0 } ?: emptyList() }
    val fav = a?.let { rememberFav("artist", it) }
    val look = LocalLook.current
    CoverTheme(a?.get("coverArt").text()) { Page("", refreshing = l.loading && loaded != null, onRefresh = l.refresh, compact = true, actions = {
        if (a != null) ItemMenu("artist", a, fav = fav!!)
    }) { pad ->
        Listing(if (loaded == null && (a == null || l.failed)) 0 else 1, l.loading, l.failed, "", l.refresh) {
            if (a != null) LazyVerticalGrid(
                GridCells.Fixed(look.gridColumns),
                contentPadding = PaddingValues(top = pad.calculateTopPadding(), bottom = pad.calculateBottomPadding() + 8.dp),
            ) {
                full {
                    Hero(a["coverArt"].text(), coverShape(look.artistShape), a["name"].text(), plural(R.plurals.n_albums, a["albumCount"].text().toIntOrNull() ?: albums.size), share = artistCover(id, from)) {
                        Round({ scope.launch { artistRadio(ctx, id) } }, canControl()) { Icon(painterResource(R.drawable.ic_radio), str(R.string.artist_radio)) }
                        Wide({ play(top) }, canControl() && top.isNotEmpty())
                        FavButton(fav!!)
                    }
                }
                if (top.isNotEmpty()) {
                    full { Header(str(R.string.popular)) }
                    top.take(5).forEachIndexed { i, s -> full { SongRow(s, { play(top, i) }, card = true) } }
                }
                if (albums.isNotEmpty()) full { Header(str(R.string.albums)) }
                items(albums) { AlbumCard(it, Modifier.padding(horizontal = 4.dp)) }
            }
        }
    } }
}

private fun LazyGridScope.full(content: @Composable () -> Unit) = item(span = { GridItemSpan(maxLineSpan) }) { content() }

/** getAlbumList2 byGenre, 60 at a time; "Karışık çal" plays 50 random songs of it. */
@Composable
fun Genre(name: String) {
    val p = pager(name, size = 60) { off ->
        getList("getAlbumList2", mapOf("type" to "byGenre", "genre" to name, "size" to 60, "offset" to off), "albumList2", "album")
    }
    val ctl = canControl()
    val scope = rememberCoroutineScope()
    Page(name, onRefresh = p::reset, actions = {
        IconButton({ scope.launch { getList("getRandomSongs", mapOf("genre" to name, "size" to 50), "randomSongs", "song")?.let { play(it.map(::Song)) } } }, enabled = ctl) {
            Icon(painterResource(R.drawable.ic_shuffle), str(R.string.shuffle_play))
        }
    }) { pad -> AlbumGrid(p, pad, str(R.string.no_albums_in_genre)) }
}

/** getPlaylist: play, shuffle; its owner can remove songs and delete it. */
@Composable
fun Playlist(id: String) {
    val nav = LocalNav.current
    val ctx = LocalContext.current
    val scope = rememberCoroutineScope()
    val l = load(id) { getObj("getPlaylist", mapOf("id" to id), "playlist") }
    val pl = l.value
    val songs = remember(pl) { mutableStateListOf<Song>().apply { addAll(pl?.songs("entry") ?: emptyList()) } }
    val user = Rela.player.collectAsStateWithLifecycle().value?.user ?: ""
    val mine = pl != null && pl["owner"].text() == user
    var busy by remember { mutableStateOf(false) } // one removal at a time: indexes shift after each
    var confirm by remember { mutableStateOf(false) }

    fun removeAt(i: Int) {
        if (busy) return
        busy = true
        val s = songs.removeAt(i)
        scope.launch {
            val ok = Rela.set("updatePlaylist", mapOf("playlistId" to id, "songIndexToRemove" to listOf(i))) != null
            busy = false
            if (!ok) {
                songs.add(i, s)
                ctx.say(str(R.string.action_failed))
            }
        }
    }

    Page("", refreshing = l.loading && pl != null, onRefresh = l.refresh, compact = true, actions = {
        if (mine) IconButton({ confirm = true }) { Icon(painterResource(R.drawable.ic_delete), str(R.string.delete)) }
    }) { pad ->
        Listing(if (pl == null) 0 else 1, l.loading, l.failed, "", l.refresh) {
            if (pl != null) LazyColumn(contentPadding = pad) {
                item {
                    Hero(
                        pl["coverArt"].text(), coverShape(LocalLook.current.coverShape), pl["name"].text(), pl["comment"].text(),
                        sub = plural(R.plurals.n_songs, songs.size) + " · " + length(songs.sumOf { it.duration }),
                    ) { PlayButtons(songs.toList()) }
                }
                itemsIndexed(songs) { i, s -> SongRow(s, { play(songs.toList(), i) }, remove = if (mine) ({ removeAt(i) }) else null, card = true) }
            }
        }
    }
    if (confirm) AlertDialog(
        onDismissRequest = { confirm = false },
        title = { Text(str(R.string.delete_playlist)) },
        confirmButton = {
            TextButton({
                confirm = false
                scope.launch {
                    if (Rela.set("deletePlaylist", mapOf("id" to id)) != null) nav.removeAt(nav.lastIndex) else ctx.say(str(R.string.action_failed))
                }
            }) { Text(str(R.string.delete)) }
        },
        dismissButton = { TextButton({ confirm = false }) { Text(str(R.string.cancel)) } },
    )
}
