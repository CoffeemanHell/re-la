package app.rela.ui

import androidx.compose.runtime.getValue
import androidx.compose.material3.ShortNavigationBarItem
import androidx.compose.material3.ShortNavigationBar
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.SharedTransitionLayout
import androidx.compose.animation.fadeOut
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.ui.unit.IntOffset
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.fadeIn
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.ui.NavDisplay
import androidx.navigationevent.DirectNavigationEventInput
import androidx.navigationevent.NavigationEvent
import androidx.navigationevent.compose.LocalNavigationEventDispatcherOwner
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.withFrameNanos
import kotlinx.coroutines.launch
import androidx.navigation3.runtime.rememberSaveableStateHolderNavEntryDecorator
import androidx.lifecycle.viewmodel.navigation3.rememberViewModelStoreNavEntryDecorator
import app.rela.R
import app.rela.core.Rela
import app.rela.playback.queueSong
import app.rela.ui.library.Album
import app.rela.ui.library.Albums
import app.rela.ui.library.Artist
import app.rela.ui.library.Artists
import app.rela.ui.library.Favorites
import app.rela.ui.library.Genre
import app.rela.ui.library.Genres
import app.rela.ui.library.Library
import app.rela.ui.library.Playlist
import app.rela.ui.library.Playlists
import app.rela.ui.library.Tracks
import app.rela.ui.library.Radio
import app.rela.ui.library.Search
import app.rela.ui.library.Settings
import app.rela.ui.library.SettingsPage
import app.rela.ui.library.Together

/** Where the app is; the back stack is a list of these. */
sealed interface Screen {
    data object Library : Screen
    data object Search : Screen
    data object Together : Screen
    data object Settings : Screen
    /** A settings category: look, player, bar, playback, data, account. */
    data class SettingsPage(val page: String) : Screen
    data object NowPlaying : Screen
    /** [type]: a getAlbumList2 order; null = the one last picked on the Albümler tab. */
    data class Albums(val type: String? = null) : Screen
    /** [from]: which list on the page below opened it, when one album can show twice there (its cover flies back to it). */
    data class Album(val id: String, val from: String = "") : Screen
    data class Artist(val id: String, val from: String = "") : Screen
    data class Genre(val name: String) : Screen
    data class Playlist(val id: String) : Screen
    data object Artists : Screen
    data object Genres : Screen
    data object Playlists : Screen
    data object Favorites : Screen
    data object Tracks : Screen
    data object Radio : Screen
}

data class Tab(val id: String, val screen: Screen, val label: String, val icon: Int)

/** All bottom bar tabs, in [tabIds] order; Settings → Alt çubuk picks which show. */
val tabs = listOf(
    Tab("library", Screen.Library, "Kitaplık", R.drawable.ic_library_music_fill),
    Tab("albums", Screen.Albums(), "Albümler", R.drawable.ic_album_fill),
    Tab("playlists", Screen.Playlists, "Playlistler", R.drawable.ic_playlist_play),
    Tab("artists", Screen.Artists, "Sanatçılar", R.drawable.ic_artist_fill),
    Tab("radio", Screen.Radio, "Radyo", R.drawable.ic_radio_fill),
    Tab("together", Screen.Together, "Birlikte", R.drawable.ic_headphones_fill),
)

val LocalNav = staticCompositionLocalOf<SnapshotStateList<Screen>> { error("no nav") }

fun SnapshotStateList<Screen>.open(s: Screen) {
    if (lastOrNull() != s) add(s)
}

/**
 * Back from a button (←, ⌄), animated: sent as the back key sends it, started then completed a couple of frames
 * later. NavDisplay animates that path; a bare removeAt, or both events in one frame, cuts to the page below.
 */
@Composable
fun rememberBack(): () -> Unit {
    val nav = LocalNav.current
    val owner = LocalNavigationEventDispatcherOwner.current
    val scope = rememberCoroutineScope()
    val input = remember(owner) { DirectNavigationEventInput() }
    DisposableEffect(owner) {
        owner?.navigationEventDispatcher?.addInput(input)
        onDispose { owner?.navigationEventDispatcher?.removeInput(input) }
    }
    return {
        if (owner == null) { if (nav.size > 1) nav.removeAt(nav.lastIndex) } else {
            input.backStarted(NavigationEvent())
            scope.launch { repeat(2) { withFrameNanos { } }; input.backCompleted() }
        }
    }
}

/**
 * Pages move sideways: a new one slides in from the right over the old, which drifts a quarter to the left; back
 * slides it out to the right ([out] = -1: to the left, for a back swipe from the right edge). Both stay opaque.
 */
private fun slide(forward: Boolean, out: Int = 1): ContentTransform {
    val spec = tween<IntOffset>(350, easing = FastOutSlowInEasing)
    return if (forward) slideInHorizontally(spec) { it } togetherWith slideOutHorizontally(spec) { -it / 4 }
    // going back the leaving page stays on top, or the one coming back covers it and it seems to vanish
    else (slideInHorizontally(spec) { -out * it / 4 } togetherWith slideOutHorizontally(spec) { out * it }).apply { targetContentZIndex = -1f }
}

/**
 * Album and artist pages, as in Navic: a quick fade (in with a slight slide), so the cover flying between the card and
 * the page carries the move; a back swipe flies it with the finger.
 */
private val coverPage = run {
    val pop = { fadeIn(tween(220)) togetherWith fadeOut(tween(120)) }
    NavDisplay.transitionSpec { (fadeIn(tween(220, delayMillis = 60)) + slideInHorizontally(tween(300)) { it / 12 }) togetherWith fadeOut(tween(120)) } +
        NavDisplay.popTransitionSpec { pop() } + NavDisplay.predictivePopTransitionSpec { pop() }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun App() {
    val s by Rela.player.collectAsStateWithLifecycle()
    val lk = remember(s?.optionsJson) { look(s?.optionsJson ?: "{}") }
    // a saved login reconnecting at start: the spinner, not a flash of the form (the library waits for online, as
    // the core answers calls before it with null); once that connect fails, the form as before
    val resume = remember { BooleanArray(2) } // [decided, resume]
    s?.let { st ->
        if (!resume[0]) { resume[0] = true; resume[1] = st.connecting }
        if (!st.online && !st.connecting) resume[1] = false
    }
    RelaTheme(s?.theme ?: "system", lk.amoled) {
        CompositionLocalProvider(LocalLook provides lk) { Surface(Modifier.fillMaxSize()) {
            val state = s
            when {
                state == null || (!state.online && resume[1]) -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { LoadingIndicator() }
                !state.online -> Login(state)
                else -> Shell()
            }
        } }
    }
}

@Composable
private fun Shell() {
    val nav = remember { mutableStateListOf<Screen>(Screen.Library) }
    val s by Rela.player.collectAsStateWithLifecycle()
    val queue by Rela.queue.collectAsStateWithLifecycle()
    val playing = nav.lastOrNull() == Screen.NowPlaying
    CompositionLocalProvider(LocalNav provides nav) {
        Scaffold(
            contentWindowInsets = WindowInsets(0), // each screen's top bar takes the status bar
            bottomBar = {
                if (!playing) Column {
                    s?.let { st -> MiniPlayer(st, queueSong(st, queue)) { nav.open(Screen.NowPlaying) } }
                    ShortNavigationBar {
                        val shown = LocalLook.current.tabs
                        tabs.filter { it.id in shown }.forEach { t ->
                            ShortNavigationBarItem(
                                selected = nav.firstOrNull() == t.screen,
                                onClick = {
                                    nav.clear()
                                    nav.add(t.screen)
                                },
                                icon = { Icon(painterResource(t.icon), null) },
                                label = { Text(t.label) },
                            )
                        }
                    }
                }
            },
        ) { pad ->
            SharedTransitionLayout { CompositionLocalProvider(LocalShared provides this) { NavDisplay(
                backStack = nav,
                onBack = { if (nav.size > 1) nav.removeAt(nav.lastIndex) },
                // a ViewModel per entry: what a screen loaded stays while it is under another one (see kept())
                entryDecorators = listOf(rememberSaveableStateHolderNavEntryDecorator(), rememberViewModelStoreNavEntryDecorator()),
                modifier = Modifier.padding(pad).consumeWindowInsets(pad),
                transitionSpec = { slide(forward = true) },
                popTransitionSpec = { slide(forward = false) },
                // the back swipe moves the page with the finger, away from the edge it started at (1 = EDGE_RIGHT)
                predictivePopTransitionSpec = { edge -> slide(forward = false, out = if (edge == 1) -1 else 1) },
                entryProvider = entryProvider {
                    entry<Screen.Library> { Library() }
                    entry<Screen.Albums> { Albums(it.type) }
                    entry<Screen.Album>(metadata = coverPage) { Album(it.id, it.from) }
                    entry<Screen.Artist>(metadata = coverPage) { Artist(it.id, it.from) }
                    entry<Screen.Genre> { Genre(it.name) }
                    entry<Screen.Playlist> { Playlist(it.id) }
                    entry<Screen.Artists> { Artists() }
                    entry<Screen.Genres> { Genres() }
                    entry<Screen.Playlists> { Playlists() }
                    entry<Screen.Favorites> { Favorites() }
                    entry<Screen.Tracks> { Tracks() }
                    entry<Screen.Search> { Search() }
                    entry<Screen.Together> { Together() }
                    entry<Screen.Settings> { Settings() }
                    entry<Screen.SettingsPage> { SettingsPage(it.page) }
                    entry<Screen.Radio> { Radio() }
                    entry<Screen.NowPlaying>(
                        metadata = NavDisplay.transitionSpec {
                            slideInVertically { it } togetherWith ExitTransition.KeepUntilTransitionsFinished
                        } + NavDisplay.popTransitionSpec {
                            fadeIn() togetherWith slideOutVertically { it }
                        } + NavDisplay.predictivePopTransitionSpec {
                            fadeIn() togetherWith slideOutVertically { it }
                        },
                    ) { NowPlaying() }
                },
            ) } }
        }
    }
}
