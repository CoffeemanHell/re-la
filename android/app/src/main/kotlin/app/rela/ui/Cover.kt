package app.rela.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.Dp
import app.rela.R
import app.rela.core.Rela
import coil3.ImageLoader
import coil3.compose.AsyncImage
import coil3.compose.LocalPlatformContext
import coil3.memory.MemoryCache
import coil3.request.ImageRequest
import java.util.concurrent.ConcurrentHashMap
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.navigation3.ui.LocalNavAnimatedContentScope
import coil3.decode.DataSource
import coil3.decode.ImageSource
import coil3.fetch.FetchResult
import coil3.fetch.Fetcher
import coil3.fetch.SourceFetchResult
import coil3.key.Keyer
import coil3.request.Options
import okio.Buffer
import okio.FileSystem

/** A coverArt id at a pixel size; the bytes come from the core's cache (the token never reaches Kotlin). */
data class CoverArt(val id: String, val px: Int)

class CoverFetcher(private val c: CoverArt) : Fetcher {
    override suspend fun fetch(): FetchResult? {
        val bytes = Rela.core?.cover(c.id, c.px.toUInt()) ?: return null
        return SourceFetchResult(ImageSource(Buffer().write(bytes), FileSystem.SYSTEM), null, DataSource.DISK)
    }

    class Factory : Fetcher.Factory<CoverArt> {
        override fun create(data: CoverArt, options: Options, imageLoader: ImageLoader) = CoverFetcher(data)
    }
}

class CoverKeyer : Keyer<CoverArt> {
    override fun key(data: CoverArt, options: Options) = "${data.id}@${data.px}"
}

/** Set around NavDisplay: the layout covers fly in between pages. */
val LocalShared = staticCompositionLocalOf<SharedTransitionScope?> { null }

/**
 * This cover flies to the one with the same [key] on the next page and back (an album card and its album page, an
 * artist row and the artist page), as in Navic. An empty key, or outside a page, changes nothing.
 */
@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
fun Modifier.sharedCover(key: String): Modifier {
    val shared = LocalShared.current
    if (key.isEmpty() || shared == null) return this
    val page = LocalNavAnimatedContentScope.current
    return with(shared) { this@sharedCover.sharedElement(rememberSharedContentState(key), page) }
}

/** The memory cache key each cover id last loaded under: a bigger copy shows the smaller one until it is there. */
private val lastCover = ConcurrentHashMap<String, MemoryCache.Key>()

/** Album art in a rounded box; a note under it while there is none. */
@Composable
fun Cover(id: String, size: Dp, modifier: Modifier = Modifier, shape: Shape = RoundedCornerShape(8)) {
    val px = with(LocalDensity.current) { size.roundToPx() }
    // AsyncImage over the note, not SubcomposeAsyncImage: subcomposition per cover makes long grids stutter
    Box(modifier.clip(shape).background(MaterialTheme.colorScheme.surfaceContainerHighest), contentAlignment = Alignment.Center) {
        Icon(painterResource(R.drawable.ic_music_note), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
        val ctx = LocalPlatformContext.current
        val art = CoverArt(id, coverPx(px, LocalLook.current.coverQuality))
        if (id.isNotEmpty()) AsyncImage(
            model = remember(art) {
                ImageRequest.Builder(ctx).data(art).placeholderMemoryCacheKey(lastCover[id])
                    .listener(onSuccess = { _, r -> r.memoryCacheKey?.let { lastCover[id] = it } }).build()
            },
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.matchParentSize(),
        )
    }
}
