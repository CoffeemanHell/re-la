package app.rela.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import coil3.imageLoader
import coil3.request.ImageRequest
import coil3.request.allowHardware
import coil3.toBitmap
import com.materialkolor.dynamicColorScheme
import com.materialkolor.ktx.themeColorOrNull

/** With "Kapaktan renk" on: [content] in a scheme seeded from [coverArt]'s main color (AMOLED still applies). */
@Composable
fun CoverTheme(coverArt: String, content: @Composable () -> Unit) {
    val look = LocalLook.current
    if (!look.coverColors || coverArt.isEmpty()) return content()
    val ctx = LocalContext.current
    var seed by remember(coverArt) { mutableStateOf<Color?>(null) }
    LaunchedEffect(coverArt) {
        // a small copy is enough for the color, and it is likely in Coil's cache already
        val req = ImageRequest.Builder(ctx).data(CoverArt(coverArt, 64)).allowHardware(false).build()
        seed = ctx.imageLoader.execute(req).image?.toBitmap()?.asImageBitmap()?.themeColorOrNull()
    }
    val base = MaterialTheme.colorScheme
    val dark = base.background.luminance() < 0.5f
    val scheme = seed?.let { relaScheme(dynamicColorScheme(it, dark), dark, look.amoled) } ?: base
    MaterialTheme(colorScheme = scheme, content = content)
}
