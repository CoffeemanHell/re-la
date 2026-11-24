package app.rela.ui

import androidx.compose.material3.ColorScheme
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import kotlinx.serialization.json.JsonPrimitive
import app.rela.core.Rela
import app.rela.core.json
import app.rela.core.text

/** The appearance options (core options, set in Settings → Görünüm); a missing or bad value gives the default. */
data class Look(
    val amoled: Boolean = false,
    val gridColumns: Int = 2,
    val coverShape: String = "soft",
    val artistShape: String = "circle",
    val coverColors: Boolean = false,
    val alphaScroll: Boolean = true,
    val albumView: String = "grid",
    val albumOrder: String = "newest",
    val coverTap: String = "lyrics",
    val tabs: List<String> = tabIds,
    val coverQuality: String = "high",
)

/** The bottom bar tabs in bar order; Kitaplık is the start screen, so it is always shown. */
val tabIds = listOf("library", "albums", "playlists", "artists", "radio", "together")

private fun tabs(value: String): List<String> {
    val on = value.split(",").map { it.trim() }.toSet() + "library"
    return tabIds.filter { it in on }
}

/** The pixels to ask for a cover at [quality]: low halves them, medium takes three quarters. */
fun coverPx(px: Int, quality: String): Int = when (quality) {
    "low" -> px / 2
    "medium" -> px * 3 / 4
    else -> px
}.coerceAtLeast(1)

private val shapes = setOf("soft", "square", "circle")

fun look(optionsJson: String): Look {
    val o = json(optionsJson)
    val d = Look()
    fun on(key: String, default: Boolean) = when (o[key].text()) { "true" -> true; "false" -> false; else -> default }
    fun pick(key: String, allowed: Set<String>, default: String) = o[key].text().takeIf { it in allowed } ?: default
    return Look(
        amoled = on("amoled", d.amoled),
        gridColumns = o["gridColumns"].text().toDoubleOrNull()?.toInt()?.coerceIn(2, 4) ?: d.gridColumns,
        coverShape = pick("coverShape", shapes, d.coverShape),
        artistShape = pick("artistShape", shapes, d.artistShape),
        coverColors = on("coverColors", d.coverColors),
        alphaScroll = on("alphaScroll", d.alphaScroll),
        albumView = pick("albumView", setOf("grid", "list"), d.albumView),
        albumOrder = o["albumOrder"].text().ifEmpty { d.albumOrder },
        coverTap = pick("coverTap", setOf("lyrics", "queue", "none"), d.coverTap),
        tabs = o["tabs"].text().takeIf { o.containsKey("tabs") }?.let(::tabs) ?: d.tabs,
        coverQuality = pick("coverQuality", setOf("low", "medium", "high"), d.coverQuality),
    )
}

/** AMOLED: a dark scheme's background and surface turn black; the containers keep their tones so cards still show. */
fun relaScheme(base: ColorScheme, dark: Boolean, amoled: Boolean): ColorScheme =
    if (dark && amoled) base.copy(background = Color.Black, surface = Color.Black) else base

val LocalLook = staticCompositionLocalOf { Look() }

/** A core option; the core parses the value as JSON, so text goes quoted. */
fun setOption(key: String, v: Any) {
    Rela.core?.setOptionJson(key, if (v is String) JsonPrimitive(v).toString() else v.toString())
}
