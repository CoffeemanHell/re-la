package app.rela.ui.library

import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.Role
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.ToggleButton
import androidx.compose.material3.ButtonGroupDefaults
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import kotlinx.coroutines.launch
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.ui.draw.clip
import app.rela.R
import app.rela.ui.setOption
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import app.rela.core.Song
import app.rela.core.length
import app.rela.core.listOf
import app.rela.core.songs
import app.rela.core.text
import app.rela.ui.Cover
import app.rela.ui.LocalNav
import app.rela.ui.LocalLook
import app.rela.ui.Screen
import app.rela.ui.open
import java.text.Collator
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.serialization.json.JsonObject

private val strips = listOf(
    "recent" to "Son dinlenenler", "newest" to "Yeni eklenenler", "frequent" to "Çok dinlenenler", "random" to "Rastgele",
)

private val sections: List<Triple<String, Int, Screen>> = listOf(
    Triple("Türler", R.drawable.ic_genres_fill, Screen.Genres),
    Triple("Favoriler", R.drawable.ic_star_fill, Screen.Favorites),
    Triple("Şarkılar", R.drawable.ic_music_note_2_fill, Screen.Tracks),
)

/** Kitaplık: Türler, Favoriler, Şarkılar, then four strips of albums from getAlbumList2; "Tümü" opens Albums in that order. */
@Composable
fun Library() {
    val nav = LocalNav.current
    val l = load {
        coroutineScope {
            strips.map { (t, _) -> async { getList("getAlbumList2", mapOf("type" to t, "size" to 20), "albumList2", "album") } }.awaitAll()
        }.takeIf { r -> r.any { it != null } }
    }
    val lists = l.value ?: emptyList()
    Page("Kitaplık", refreshing = l.loading && l.value != null, onRefresh = l.refresh, root = true) { pad ->
        Listing(lists.sumOf { it?.size ?: 0 }, l.loading, l.failed, "Kütüphane boş", l.refresh) {
            LazyColumn(contentPadding = pad) {
                item {
                    Row(Modifier.padding(horizontal = 16.dp, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        sections.forEach { (t, icon, s) ->
                            Surface({ nav.open(s) }, Modifier.weight(1f), shape = RoundedCornerShape(16.dp), color = MaterialTheme.colorScheme.surfaceContainer) {
                                Column(Modifier.padding(vertical = 14.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                    Icon(painterResource(icon), null, tint = MaterialTheme.colorScheme.primary)
                                    Text(t, Modifier.padding(top = 6.dp), style = MaterialTheme.typography.labelLarge, maxLines = 1)
                                }
                            }
                        }
                    }
                }
                strips.forEachIndexed { i, (t, title) ->
                    val albums = lists.getOrNull(i) ?: emptyList()
                    if (albums.isNotEmpty()) item(t) {
                        Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(title, Modifier.weight(1f), style = MaterialTheme.typography.titleLarge)
                            TextButton({ nav.open(Screen.Albums(t)) }) { Text("Tümü") }
                        }
                        LazyRow(contentPadding = PaddingValues(horizontal = 10.dp)) { items(albums) { StripCard(it, from = t) } }
                    }
                }
            }
        }
    }
}

/** Albümler: the order from the sort sheet (the tab remembers it; a "Tümü" list does not), grid or list. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Albums(picked: String?) {
    val nav = LocalNav.current
    val look = LocalLook.current
    val type = picked ?: look.albumOrder
    var starred by rememberSaveable { mutableStateOf(false) }
    var descending by rememberSaveable { mutableStateOf(true) }
    val params = albumParams(type, starred, descending)
    val p = pager(params, size = 60) { off -> getList("getAlbumList2", params + mapOf("size" to 60, "offset" to off), "albumList2", "album") }
    var sheet by remember { mutableStateOf(false) }
    val title = if (starred) "Yıldızlılar" else albumOrders.toMap()[type] ?: ""
    Page("Albümler", title, onRefresh = p::reset, root = picked == null, actions = {
        IconButton({ sheet = true }) { Icon(painterResource(R.drawable.ic_sort), "Sırala") }
    }) { pad -> AlbumGrid(p, pad, "Albüm yok") }
    if (sheet) ModalBottomSheet({ sheet = false }) {
        Column(Modifier.padding(horizontal = 16.dp).padding(bottom = 16.dp).verticalScroll(rememberScrollState())) {
            Text("Sırala", Modifier.padding(start = 8.dp, bottom = 8.dp), style = MaterialTheme.typography.headlineSmall)
            albumOrders.forEach { (t, label) ->
                Row(
                    Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).clickable {
                        starred = false
                        if (picked == null) setOption("albumOrder", t) else nav[nav.lastIndex] = Screen.Albums(t) // back returns to this order
                    }.heightIn(min = 48.dp).padding(horizontal = 8.dp), // a full touch target per row
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                    RadioButton(!starred && t == type, null)
                }
            }
            FilterChip(starred, { starred = !starred }, { Text("Yıldızlılar") }, Modifier.padding(vertical = 8.dp),
                leadingIcon = { Icon(painterResource(R.drawable.ic_star_fill), null, Modifier.size(18.dp)) })
            Text("Görünüm", Modifier.padding(top = 8.dp, bottom = 4.dp), style = MaterialTheme.typography.titleMedium)
            Segments(listOf("grid" to "Izgara", "list" to "Liste"), look.albumView) { setOption("albumView", it) }
            if (!starred && type == "byYear") {
                Text("Yön", Modifier.padding(top = 12.dp, bottom = 4.dp), style = MaterialTheme.typography.titleMedium)
                Segments(listOf("asc" to "Artan", "desc" to "Azalan"), if (descending) "desc" else "asc") { descending = it == "desc" }
            }
        }
    }
}

/** A full-width single-choice row of [options] (key → label). */
@Composable
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
fun Segments(options: List<Pair<String, String>>, selected: String, modifier: Modifier = Modifier, onPick: (String) -> Unit) =
    // a connected button group: the picked one turns round, the others keep square inner corners
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(ButtonGroupDefaults.ConnectedSpaceBetween)) {
        options.forEachIndexed { i, (key, label) ->
            ToggleButton(
                checked = selected == key,
                onCheckedChange = { onPick(key) },
                modifier = Modifier.weight(1f).semantics { role = Role.RadioButton },
                shapes = when (i) {
                    0 -> ButtonGroupDefaults.connectedLeadingButtonShapes()
                    options.lastIndex -> ButtonGroupDefaults.connectedTrailingButtonShapes()
                    else -> ButtonGroupDefaults.connectedMiddleButtonShapes()
                },
            ) { Text(label, maxLines = 1) }
        }
    }

/** Albums of a [Pager] as the Look says (grid of N columns, or list); the next page is fetched near the end. */
@Composable
fun AlbumGrid(p: Pager<JsonObject>, pad: PaddingValues, empty: String) {
    val look = LocalLook.current
    val list = look.albumView == "list"
    val grid = rememberLazyGridState()
    LaunchedEffect(p) { snapshotFlow { grid.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }.collect { if (it >= p.items.size - 12) p.next() } }
    Listing(p.items.size, p.loading, p.failed, empty, p::next) {
        LazyVerticalGrid(
            GridCells.Fixed(if (list) 1 else look.gridColumns), state = grid,
            contentPadding = PaddingValues(start = 10.dp, end = 10.dp, top = pad.calculateTopPadding(), bottom = pad.calculateBottomPadding() + 8.dp),
        ) {
            items(p.items) { if (list) AlbumRow(it) else AlbumCard(it) }
        }
    }
}

@Composable
fun Artists() {
    val l = load { getList("getArtists", emptyMap(), "artists", "index") }
    val index = l.value ?: emptyList()
    val list = rememberLazyListState()
    val scope = rememberCoroutineScope()
    // the list position of each letter's header (a header, then its artists)
    val starts = remember(index) { var n = 0; index.map { i -> n.also { n += 1 + i.listOf("artist").size } } }
    Page("Sanatçılar", refreshing = l.loading && l.value != null, onRefresh = l.refresh, root = true) { pad ->
        Listing(index.size, l.loading, l.failed, "Sanatçı yok", l.refresh) {
            Box(Modifier.fillMaxSize()) {
                val bar = LocalLook.current.alphaScroll && index.size > 1
                // the letter bar gets its own strip, so it does not cover the rows' ⋮
                LazyColumn(Modifier.padding(end = if (bar) 28.dp else 0.dp), state = list, contentPadding = pad) {
                    index.forEach { i ->
                        item { Header(i["name"].text()) }
                        items(i.listOf("artist")) { ArtistRow(it) }
                    }
                }
                if (bar) LetterBar(
                    index.map { it["name"].text() },
                    Modifier.align(Alignment.CenterEnd).padding(top = pad.calculateTopPadding() + 8.dp, bottom = pad.calculateBottomPadding() + 8.dp),
                ) { k -> scope.launch { list.scrollToItem(starts[k]) } }
            }
        }
    }
}

/** The index letters down the right edge: tap or drag to jump to one. */
@Composable
private fun LetterBar(letters: List<String>, modifier: Modifier, onPick: (Int) -> Unit) {
    var h by remember { mutableIntStateOf(1) }
    fun pick(y: Float) = onPick((y / h * letters.size).toInt().coerceIn(0, letters.lastIndex))
    Column(
        modifier.width(28.dp).onSizeChanged { h = it.height.coerceAtLeast(1) }
            .pointerInput(letters) { detectTapGestures { pick(it.y) } }
            .pointerInput(letters) { detectVerticalDragGestures { change, _ -> pick(change.position.y) } },
        verticalArrangement = Arrangement.SpaceEvenly,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        letters.forEach { Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary) }
    }
}

@Composable
fun Genres() {
    val nav = LocalNav.current
    val l = load { getList("getGenres", emptyMap(), "genres", "genre")?.sortedWith(compareBy(Collator.getInstance()) { it["value"].text() }) }
    val genres = l.value ?: emptyList()
    Page("Türler", refreshing = l.loading && l.value != null, onRefresh = l.refresh) { pad ->
        Listing(genres.size, l.loading, l.failed, "Tür yok", l.refresh) {
            LazyColumn(contentPadding = pad) {
                items(genres) { g ->
                    val name = g["value"].text()
                    ListItem(
                        headlineContent = { Text(name) },
                        supportingContent = { Text("${g["albumCount"].text().ifEmpty { "0" }} albüm · ${g["songCount"].text().ifEmpty { "0" }} şarkı") },
                        modifier = Modifier.clickable { nav.open(Screen.Genre(name)) },
                    )
                }
            }
        }
    }
}

@Composable
fun Playlists() {
    val nav = LocalNav.current
    val l = load { getList("getPlaylists", emptyMap(), "playlists", "playlist") }
    val lists = l.value ?: emptyList()
    Page("Playlistler", refreshing = l.loading && l.value != null, onRefresh = l.refresh, root = true) { pad ->
        Listing(lists.size, l.loading, l.failed, "Playlist yok", l.refresh) {
            LazyColumn(contentPadding = pad) {
                items(lists) { p ->
                    ListItem(
                        headlineContent = { Text(p["name"].text()) },
                        supportingContent = { Text("${p["songCount"].text().ifEmpty { "0" }} şarkı · ${length(Song(p).duration)}") },
                        leadingContent = { Cover(p["coverArt"].text(), 48.dp, Modifier.size(48.dp)) },
                        trailingContent = { ItemMenu("playlist", p) },
                        modifier = Modifier.clickable { nav.open(Screen.Playlist(p["id"].text())) },
                    )
                }
            }
        }
    }
}

/** Starred songs, albums and artists (getStarred2), a tab each. */
@Composable
fun Favorites() {
    val l = load { getObj("getStarred2", emptyMap(), "starred2") }
    val s = l.value
    var tab by rememberSaveable { mutableIntStateOf(0) }
    Page("Favoriler", refreshing = l.loading && s != null, onRefresh = l.refresh) { pad ->
        Column {
            PrimaryTabRow(tab) {
                listOf("Şarkılar", "Albümler", "Sanatçılar").forEachIndexed { i, t -> Tab(tab == i, { tab = i }, text = { Text(t) }) }
            }
            when (tab) {
                0 -> {
                    val songs = remember(s) { s?.songs("song") ?: emptyList() }
                    Listing(songs.size, l.loading, l.failed, "Favori şarkı yok", l.refresh) {
                        LazyColumn(contentPadding = pad) { itemsIndexed(songs) { i, song -> SongRow(song, { play(songs, i) }) } }
                    }
                }
                1 -> {
                    val albums = s?.listOf("album") ?: emptyList()
                    Listing(albums.size, l.loading, l.failed, "Favori albüm yok", l.refresh) {
                        LazyVerticalGrid(GridCells.Adaptive(150.dp), contentPadding = PaddingValues(start = 10.dp, end = 10.dp, bottom = pad.calculateBottomPadding())) {
                            items(albums) { AlbumCard(it) }
                        }
                    }
                }
                else -> {
                    val artists = s?.listOf("artist") ?: emptyList()
                    Listing(artists.size, l.loading, l.failed, "Favori sanatçı yok", l.refresh) {
                        LazyColumn(contentPadding = pad) { items(artists) { ArtistRow(it) } }
                    }
                }
            }
        }
    }
}

/** Every song (search3 with an empty query), 100 at a time; a tap plays from that row. */
@Composable
fun Tracks() {
    val p = pager("tracks", size = 100) { off ->
        getList("search3", mapOf("query" to "", "songCount" to 100, "songOffset" to off, "artistCount" to 0, "albumCount" to 0), "searchResult3", "song")
            ?.map(::Song)
    }
    val list = rememberLazyListState()
    LaunchedEffect(p) { snapshotFlow { list.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }.collect { if (it >= p.items.size - 20) p.next() } }
    Page("Şarkılar", onRefresh = p::reset) { pad ->
        Listing(p.items.size, p.loading, p.failed, "Şarkı yok", p::next) {
            LazyColumn(state = list, contentPadding = pad) { itemsIndexed(p.items) { i, s -> SongRow(s, { play(p.items.toList(), i) }) } }
        }
    }
}
