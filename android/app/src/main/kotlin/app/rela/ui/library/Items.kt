package app.rela.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeFlexibleTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.text
import app.rela.icons.Icons
import app.rela.icons.filled.Settings
import app.rela.icons.outlined.ArrowBack
import app.rela.icons.outlined.Search
import app.rela.ui.Cover
import app.rela.ui.LocalLook
import app.rela.ui.LocalNav
import app.rela.ui.Screen
import app.rela.ui.clock
import app.rela.ui.open
import app.rela.ui.rememberBack
import app.rela.ui.sharedCover
import kotlinx.serialization.json.JsonObject

/**
 * A library screen: large flexible top bar that collapses on scroll, back when there is somewhere to go back to,
 * pull to refresh when [onRefresh] is given; [compact]: a small bar for pages with a hero; [root]: a tab's first page, with the search and settings icons. [content] gets the padding for its list.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Page(
    title: String,
    subtitle: String = "",
    refreshing: Boolean = false,
    onRefresh: (() -> Unit)? = null,
    actions: @Composable RowScope.() -> Unit = {},
    root: Boolean = false,
    compact: Boolean = false,
    content: @Composable (PaddingValues) -> Unit,
) {
    val nav = LocalNav.current
    val bar = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    Scaffold(
        // only for the large bar: a compact bar never sets the collapse limit, so the behaviour would eat every scroll
        if (compact) Modifier else Modifier.nestedScroll(bar.nestedScrollConnection),
        topBar = {
            val goBack = rememberBack()
            // fixed when the page first shows (saved, as it leaves composition while covered): a tab's first page
            // shows no ← while another page opens over it or closes back to it
            val hasBack = rememberSaveable { nav.size > 1 }
            val back = @Composable { if (hasBack) IconButton(goBack) { Icon(Icons.Outlined.ArrowBack, "Geri") } }
            val icons: @Composable RowScope.() -> Unit = {
                actions()
                if (root) { // a tab: search and settings live here
                    IconButton({ nav.open(Screen.Search) }) { Icon(Icons.Outlined.Search, "Ara") }
                    IconButton({ nav.open(Screen.Settings) }) { Icon(Icons.Filled.Settings, "Ayarlar") }
                }
            }
            // compact: a page with its own hero (album, artist, playlist) only needs back and the menu up here
            if (compact) TopAppBar({ Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) }, navigationIcon = back, actions = icons)
            else LargeFlexibleTopAppBar(
                title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis, fontFamily = FontFamily.Serif, fontWeight = FontWeight.Bold) },
                subtitle = if (subtitle.isEmpty()) null else ({ Text(subtitle, maxLines = 1, overflow = TextOverflow.Ellipsis) }),
                navigationIcon = back,
                actions = icons,
                scrollBehavior = bar,
            )
        },
    ) { pad ->
        if (onRefresh == null) content(pad)
        else PullToRefreshBox(refreshing, onRefresh, Modifier.padding(top = pad.calculateTopPadding())) {
            content(PaddingValues(bottom = pad.calculateBottomPadding()))
        }
    }
}

/** Over an empty list: the loading indicator, a retry after a failure, or [empty]. Draws nothing once there are rows. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun BoxScope.Status(rows: Int, loading: Boolean, failed: Boolean, empty: String, retry: () -> Unit) {
    if (rows > 0) return
    Column(Modifier.align(Alignment.Center).padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        when {
            loading -> LoadingIndicator()
            failed -> {
                Text("Yüklenemedi", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Button(retry) { Text("Tekrar dene") }
            }
            else -> Text(empty, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** Box filling the page with [Status] over [content]. */
@Composable
fun Listing(rows: Int, loading: Boolean, failed: Boolean, empty: String, retry: () -> Unit, content: @Composable () -> Unit) =
    Box(Modifier.fillMaxSize()) {
        content()
        Status(rows, loading, failed, empty, retry)
    }

/** A getAlbumList2 / getArtist / getStarred2 album: cover, name, artist; opens the album. */
@Composable
fun AlbumCard(a: JsonObject, modifier: Modifier = Modifier, size: Dp = 160.dp, from: String = "") {
    val nav = LocalNav.current
    val id = a["id"].text()
    Column(modifier.clickable { peeks["album:$id"] = a; nav.open(Screen.Album(id, from)) }.padding(6.dp)) {
        Cover(a["coverArt"].text(), size, Modifier.fillMaxWidth().aspectRatio(1f).sharedCover(albumCover(id, from)), coverShape(LocalLook.current.coverShape))
        Text(a["name"].text(), Modifier.padding(top = 6.dp), style = MaterialTheme.typography.titleSmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text(a["artist"].text(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/** An album as a list row: cover, name, artist; opens the album. */
@Composable
fun AlbumRow(a: JsonObject) {
    val nav = LocalNav.current
    ListItem(
        headlineContent = { Text(a["name"].text(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = { Text(a["artist"].text(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        leadingContent = { Cover(a["coverArt"].text(), 56.dp, Modifier.size(56.dp).sharedCover(albumCover(a["id"].text())), coverShape(LocalLook.current.coverShape)) },
        modifier = Modifier.clickable { peeks["album:${a["id"].text()}"] = a; nav.open(Screen.Album(a["id"].text())) },
    )
}

/** Fixed width for horizontal strips. */
@Composable
fun StripCard(a: JsonObject, from: String = "") = AlbumCard(a, Modifier.width(150.dp), 140.dp, from)

/**
 * What a card showed of an album or artist (cover, name, artist), by "album:id" / "artist:id": its page draws the hero
 * from it at once, so the cover has somewhere to fly to while the page is fetched. Main thread only.
 */
val peeks = object : LinkedHashMap<String, JsonObject>() {
    override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, JsonObject>?) = size > 100
}

/** The [sharedCover] keys of an album's and an artist's cover. */
fun albumCover(id: String, from: String = "") = "album:$id:$from"
fun artistCover(id: String, from: String = "") = "artist:$id:$from"

@Composable
fun ArtistRow(a: JsonObject) {
    val nav = LocalNav.current
    val n = a["albumCount"].text()
    ListItem(
        headlineContent = { Text(a["name"].text(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = if (n.isEmpty()) null else ({ Text("$n albüm") }),
        leadingContent = { Cover(a["coverArt"].text(), 48.dp, Modifier.size(48.dp).sharedCover(artistCover(a["id"].text())), CircleShape) },
        trailingContent = { ItemMenu("artist", a) },
        modifier = Modifier.clickable { peeks["artist:${a["id"].text()}"] = a; nav.open(Screen.Artist(a["id"].text())) },
    )
}

/**
 * A song: cover (or [number], e.g. the track on an album page), title, artist and length, ⋮ menu. The song that is
 * playing now is drawn in the primary color. [card]: a rounded card (album, playlist pages); the artist is left out
 * when it is the [albumArtist].
 */
@Composable
fun SongRow(song: Song, onClick: () -> Unit, number: String = "", remove: (() -> Unit)? = null, card: Boolean = false, albumArtist: String = "") {
    val st = Rela.player.collectAsStateWithLifecycle()
    val queue = Rela.queue.collectAsStateWithLifecycle()
    // derived: the row redraws when the current song changes, not on every position tick
    val now by remember(song.id) { derivedStateOf { st.value?.let { queue.value.getOrNull(it.index.toInt())?.id } == song.id } }
    val color = if (now) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
    ListItem(
        headlineContent = { Text(song.title, color = color, fontWeight = if (now) FontWeight.SemiBold else null, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(listOf(song.artist.takeIf { it != albumArtist } ?: "", clock(song.duration)).filter { it.isNotEmpty() }.joinToString(" · "), maxLines = 1, overflow = TextOverflow.Ellipsis)
        },
        leadingContent = {
            if (number.isEmpty()) Cover(song.coverArt, 48.dp, Modifier.size(48.dp))
            else Box(Modifier.size(32.dp), contentAlignment = Alignment.Center) {
                Text(number, color = color, style = MaterialTheme.typography.labelLarge)
            }
        },
        trailingContent = { ItemMenu("song", song.json, remove) },
        colors = if (card) ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surfaceContainer) else ListItemDefaults.colors(),
        modifier = (if (card) Modifier.padding(horizontal = 16.dp, vertical = 2.dp).clip(RoundedCornerShape(12.dp)) else Modifier).clickable(onClick = onClick),
    )
}

@Composable
fun Header(text: String, modifier: Modifier = Modifier) =
    Text(text, modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp), style = MaterialTheme.typography.titleMedium)
