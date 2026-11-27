package app.rela.ui.library

import android.text.format.Formatter
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.R
import app.rela.core.Rela
import app.rela.core.json
import app.rela.core.text
import app.rela.playback.AudioPrefs
import app.rela.plural
import app.rela.str
import app.rela.ui.LocalLook
import app.rela.ui.LocalNav
import app.rela.ui.Screen
import app.rela.ui.clearCoverCache
import app.rela.ui.coverCacheBytes
import app.rela.ui.coverCacheDir
import app.rela.ui.open
import app.rela.ui.setOption
import app.rela.ui.tabIds
import app.rela.ui.tabs
import coil3.SingletonImageLoader
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A number setting: the value moves with the finger, [onDone] gets it when the finger lifts. */
@Composable
private fun SliderRow(label: String, value: Int, range: IntRange, step: Int, enabled: Boolean = true, hint: String = "", text: (Int) -> String, onDone: (Int) -> Unit) {
    var v by remember(value) { mutableFloatStateOf(value.toFloat()) }
    ListItem(
        headlineContent = { Text("$label: ${text(v.roundToInt())}") },
        supportingContent = {
            Column {
                if (hint.isNotEmpty()) Text(hint)
                Slider(
                    v, { v = it }, enabled = enabled,
                    valueRange = range.first.toFloat()..range.last.toFloat(),
                    steps = (range.last - range.first) / step - 1,
                    onValueChangeFinished = { onDone(v.roundToInt()) },
                )
            }
        },
    )
}

private val shapes get() = listOf("soft" to str(R.string.shape_soft), "square" to str(R.string.shape_square), "circle" to str(R.string.shape_circle))

/** A labelled single choice (segmented buttons). */
@Composable
private fun ChoiceRow(label: String, options: List<Pair<String, String>>, selected: String, onPick: (String) -> Unit) =
    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
        Text(label, Modifier.padding(bottom = 6.dp), style = MaterialTheme.typography.bodyLarge)
        Segments(options, selected, onPick = onPick)
    }

/** A device-side audio switch (AudioPrefs); the decks read it when the playback service starts again. */
@Composable
private fun AudioSwitch(label: String, hint: String, key: String, enabled: Boolean = true) {
    val ctx = LocalContext.current
    var on by remember { mutableStateOf(if (key == "floatOutput") AudioPrefs.floatOutput(ctx) else AudioPrefs.offload(ctx)) }
    SwitchRow(label, hint, on, enabled) { on = it; AudioPrefs.set(ctx, key, it) }
}

/**
 * The transcode of one network, as "opus-128" ("" = the original). Only what the server makes: a profile ("opus-320") is
 * picked as it is, a format alone ("opus": the server's bitrates are not known) has a slider for the bitrate.
 */
@Composable
private fun TranscodeRows(label: String, value: String, offered: List<String>, onPick: (String) -> Unit) {
    val format = value.substringBefore('-', "")
    val kbps = value.substringAfter('-', "").toIntOrNull() ?: 128
    val picked = if (format in offered) format else value // the saved one stays shown when the server doesn't offer it now
    val shown = (offered + picked).filter { it.isNotEmpty() }.distinct()
    ChoiceRow(label, listOf("" to str(R.string.original)) + shown.map { it to it.uppercase().replace('-', ' ') }, picked) {
        onPick(if (it.isEmpty() || '-' in it) it else "$it-$kbps")
    }
    if (format in offered) SliderRow(str(R.string.bitrate), bitrates.indexOf(kbps).coerceAtLeast(0), bitrates.indices, 1, text = { "${bitrates[it]} kbps" }) {
        onPick("$format-${bitrates[it]}")
    }
}

private val bitrates = listOf(64, 96, 128, 160, 192, 256, 320)
private val cacheSizes = listOf(0, 1, 2, 5, 10, 20, 50) // GB

private data class Category(val page: String, val title: String, val sub: String, val icon: Int)

private val categories get() = listOf(
    Category("look", str(R.string.appearance), str(R.string.appearance_sub), R.drawable.ic_palette_fill),
    Category("player", str(R.string.player_screen), str(R.string.player_screen_sub), R.drawable.ic_play_arrow_fill),
    Category("bar", str(R.string.bottom_bar), str(R.string.bottom_bar_sub), R.drawable.ic_bottom_navigation_fill),
    Category("playback", str(R.string.playback), str(R.string.playback_sub), R.drawable.ic_music_note_2_fill),
    Category("dj", "DJ", str(R.string.dj_sub), R.drawable.ic_shuffle),
    Category("data", str(R.string.data), str(R.string.data_sub), R.drawable.ic_data_table),
    Category("account", str(R.string.account), str(R.string.account_sub), R.drawable.ic_account_circle_fill),
)

/** A card in a group: the group's ends are round, the cards between them nearly square. */
private fun groupShape(i: Int, n: Int): Shape {
    val big = 24.dp
    val small = 4.dp
    return RoundedCornerShape(
        topStart = if (i == 0) big else small, topEnd = if (i == 0) big else small,
        bottomStart = if (i == n - 1) big else small, bottomEnd = if (i == n - 1) big else small,
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun CategoryCard(title: String, sub: String, icon: Int, shape: Shape, onClick: (() -> Unit)?) = ListItem(
    headlineContent = { Text(title) },
    supportingContent = { Text(sub) },
    leadingContent = {
        Box(Modifier.size(48.dp).clip(MaterialShapes.Cookie9Sided.toShape()).background(MaterialTheme.colorScheme.primaryContainer), contentAlignment = Alignment.Center) {
            Icon(painterResource(icon), null, tint = MaterialTheme.colorScheme.onPrimaryContainer)
        }
    },
    colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
    modifier = Modifier.padding(horizontal = 16.dp).clip(shape).then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier),
)

/** The cover cache's size, and a button that empties it (disk and Coil's memory copies). */
@Composable
private fun CoverCacheRow() {
    val ctx = LocalContext.current
    val dir = remember { coverCacheDir(ctx) }
    var bytes by remember { mutableLongStateOf(0L) }
    var tick by remember { mutableIntStateOf(0) }
    LaunchedEffect(tick) { bytes = withContext(Dispatchers.IO) { coverCacheBytes(dir) } }
    val scope = rememberCoroutineScope()
    ListItem(
        headlineContent = { Text(str(R.string.cover_cache)) },
        supportingContent = { Text(Formatter.formatFileSize(ctx, bytes)) },
        trailingContent = {
            OutlinedButton({
                scope.launch {
                    withContext(Dispatchers.IO) { clearCoverCache(dir) }
                    SingletonImageLoader.get(ctx).memoryCache?.clear()
                    tick++
                }
            }) { Text(str(R.string.clear)) }
        },
    )
}

/** The DJ's settings page: mode, the knobs (the advanced ones folded), "more / less like this" list, state, reset. */
private fun androidx.compose.foundation.lazy.LazyListScope.djPage(dj: Boolean, num: (String, Int) -> Int, set: (String, Any) -> Unit) {
    item { SwitchRow(str(R.string.dj_mode), str(R.string.dj_mode_hint), dj) { Rela.core?.setDj(it) } }
    items(djMain, key = { it.key }) { k -> KnobRow(k, num(k.key, k.from), dj, set) }
    item { DjAdvanced(dj, num, set) }
    item {
        ListItem(
            headlineContent = { Text(str(R.string.dj_defaults)) },
            supportingContent = { Text(str(R.string.dj_defaults_hint)) },
            trailingContent = { OutlinedButton({ Rela.core?.resetDjSettings() }) { Text(str(R.string.dj_defaults_button)) } },
        )
    }
    item { DjFeedbackList() }
    item {
        val nav = LocalNav.current
        ListItem(
            headlineContent = { Text(str(R.string.dj_state)) },
            supportingContent = { Text(str(R.string.dj_state_hint)) },
            modifier = Modifier.clickable { nav.open(Screen.DjState) },
        )
    }
    item {
        ListItem(
            headlineContent = { Text(str(R.string.dj_memory)) },
            supportingContent = { Text(str(R.string.dj_memory_hint)) },
            trailingContent = { OutlinedButton({ Rela.core?.resetDjMemory() }) { Text(str(R.string.reset)) } },
        )
    }
}

@Composable
private fun KnobRow(k: Knob, value: Int, enabled: Boolean, set: (String, Any) -> Unit) =
    SliderRow(str(k.label), value.coerceIn(k.from, k.to), k.from..k.to, k.step, enabled, if (k.hint != 0) str(k.hint) else "", text = { knobText(k, it) }) { set(k.key, it) }

@Composable
private fun DjAdvanced(dj: Boolean, num: (String, Int) -> Int, set: (String, Any) -> Unit) {
    var open by rememberSaveable { mutableStateOf(false) }
    Column {
        SwitchRow(str(R.string.dj_advanced), "", open, dj) { open = it }
        if (open) djAdvanced.forEach { k -> KnobRow(k, num(k.key, k.from), dj, set) }
    }
}

/** "More / less like this" choices, newest first, each with a button that takes it back. */
@Composable
private fun DjFeedbackList() {
    var list by remember { mutableStateOf<List<kotlinx.serialization.json.JsonObject>>(emptyList()) }
    LaunchedEffect(Unit) { list = (Rela.local("djFeedback") as? kotlinx.serialization.json.JsonArray)?.mapNotNull { it as? kotlinx.serialization.json.JsonObject } ?: emptyList() }
    Column {
        Header(str(R.string.dj_feedback))
        if (list.isEmpty()) Text(str(R.string.dj_feedback_none), Modifier.padding(horizontal = 16.dp, vertical = 8.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
        list.forEach { f ->
            val more = f["dir"].text() == "1"
            ListItem(
                headlineContent = { Text(f["title"].text()) },
                supportingContent = { Text(listOf(str(if (more) R.string.dj_more_short else R.string.dj_less_short), f["artist"].text()).filter { it.isNotEmpty() }.joinToString(" · ")) },
                trailingContent = {
                    OutlinedButton({
                        Rela.core?.djFeedback(f.toString(), 0)
                        list = list - f
                    }) { Text(str(R.string.dj_remove)) }
                },
            )
        }
    }
}

/** Settings: a card per category, each opening its own page; Hakkında stands apart and only shows the version. */
@Composable
fun Settings() {
    val nav = LocalNav.current
    val ctx = LocalContext.current
    val version = remember { runCatching { ctx.packageManager.getPackageInfo(ctx.packageName, 0).versionName }.getOrNull() ?: "" }
    Page(str(R.string.settings)) { pad ->
        LazyColumn(contentPadding = pad, verticalArrangement = Arrangement.spacedBy(2.dp)) {
            itemsIndexed(categories) { i, c ->
                CategoryCard(c.title, c.sub, c.icon, groupShape(i, categories.size)) { nav.open(Screen.SettingsPage(c.page)) }
            }
            item { Spacer(Modifier.height(24.dp)) }
            item { CategoryCard(str(R.string.about), str(R.string.version, version), R.drawable.ic_info_fill, groupShape(0, 1), null) }
        }
    }
}

/** One category's settings. */
@Composable
fun SettingsPage(page: String) {
    val st by Rela.player.collectAsStateWithLifecycle()
    val s = st ?: return
    val core = Rela.core
    val o = json(s.optionsJson)
    fun on(key: String) = o[key].text() == "true"
    fun num(key: String, default: Int) = o[key].text().toDoubleOrNull()?.toInt() ?: default
    fun set(key: String, v: Any) = setOption(key, v)
    val look = LocalLook.current
    Page(categories.firstOrNull { it.page == page }?.title ?: str(R.string.settings)) { pad ->
        LazyColumn(contentPadding = pad) {
            when (page) {
                "look" -> {
                    item {
                        val themes = listOf("system" to str(R.string.theme_system), "light" to str(R.string.theme_light), "dark" to str(R.string.theme_dark))
                        // anything but light/dark (the core's default is the desktop theme "nothing") follows the system
                        Segments(themes, s.theme.takeIf { it == "light" || it == "dark" } ?: "system", Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                            core?.setTheme(it)
                        }
                    }
                    item { SwitchRow(str(R.string.amoled), str(R.string.amoled_hint), on("amoled")) { set("amoled", it) } }
                    item { SwitchRow(str(R.string.cover_colors), str(R.string.cover_colors_hint), look.coverColors) { set("coverColors", it) } }
                    item { ChoiceRow(str(R.string.grid), listOf(2, 3, 4).map { "$it" to plural(R.plurals.n_columns, it) }, "${look.gridColumns}") { set("gridColumns", it.toInt()) } }
                    item { ChoiceRow(str(R.string.cover_shape), shapes, look.coverShape) { set("coverShape", it) } }
                    item { ChoiceRow(str(R.string.artist_image), shapes, look.artistShape) { set("artistShape", it) } }
                    item { SwitchRow(str(R.string.alpha_scroll), str(R.string.alpha_scroll_hint), look.alphaScroll) { set("alphaScroll", it) } }
                }
                "dj" -> djPage(s.dj, ::num, ::set)
                "playback" -> {
                    item { Header(str(R.string.playback)) }
                    item {
                        SliderRow(str(R.string.crossfade), s.crossfade.toInt(), 0..12, 1, hint = str(R.string.crossfade_hint), text = { if (it == 0) str(R.string.off) else plural(R.plurals.n_seconds, it) }) {
                            core?.setCrossfade(it.toLong())
                        }
                    }
                    item { SwitchRow(str(R.string.auto_continue), str(R.string.auto_continue_hint), on("autoContinue")) { set("autoContinue", it) } }
                    item { Header(str(R.string.audio_experimental)) }
                    item { AudioSwitch(str(R.string.float_output), str(R.string.float_output_hint), "floatOutput") }
                    item {
                        val offload = remember { AudioPrefs.offloadSupported() }
                        AudioSwitch(
                            "Audio offload",
                            if (offload) str(R.string.offload_hint)
                            else str(R.string.offload_unsupported),
                            "audioOffload",
                            enabled = offload,
                        )
                    }
                    item { Header(str(R.string.server)) }
                    item { SwitchRow(str(R.string.scrobble), str(R.string.scrobble_hint), on("scrobble")) { set("scrobble", it) } }
                    item { SwitchRow(str(R.string.send_now_playing), str(R.string.send_now_playing_hint), on("nowPlaying")) { set("nowPlaying", it) } }
                }
                "player" -> {
                    item {
                        ChoiceRow(str(R.string.cover_tap), listOf("lyrics" to str(R.string.lyrics), "queue" to str(R.string.queue), "none" to str(R.string.nothing)), look.coverTap) { set("coverTap", it) }
                    }
                }
                "bar" -> {
                    items(tabs) { t ->
                        // Kitaplık is the start screen and the way to Settings: always shown
                        SwitchRow(t.label, "", t.id in look.tabs, enabled = t.id != "library") { on ->
                            set("tabs", tabIds.filter { if (it == t.id) on else it in look.tabs }.joinToString(","))
                        }
                    }
                }
                "data" -> {
                    item { Header("Transcoding") }
                    item {
                        var formats by remember { mutableStateOf<List<String>?>(null) }
                        LaunchedEffect(s.online) { formats = core?.transcodeFormats() }
                        Column {
                            val f = formats
                            Text(
                                if (s.online && f != null && f.isEmpty()) str(R.string.transcode_none) else str(R.string.transcode_hint),
                                Modifier.padding(horizontal = 16.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            TranscodeRows("Wi-Fi", o["transcodeWifi"].text(), f.orEmpty()) { set("transcodeWifi", it) }
                            TranscodeRows(str(R.string.mobile_data), o["transcodeMobile"].text(), f.orEmpty()) { set("transcodeMobile", it) }
                        }
                    }
                    item { Header(str(R.string.offline_songs)) }
                    item {
                        SliderRow(
                            str(R.string.size_limit), cacheSizes.indexOf(num("offlineGb", 0)).coerceAtLeast(0), cacheSizes.indices, 1,
                            hint = str(R.string.offline_hint),
                            text = { if (it == 0) str(R.string.off) else "${cacheSizes[it]} GB" },
                        ) { set("offlineGb", cacheSizes[it]) }
                    }
                    item { SliderRow(str(R.string.download_ahead), num("offlineAhead", 3), 0..20, 1, num("offlineGb", 0) > 0, text = { plural(R.plurals.n_songs, it) }) { set("offlineAhead", it) } }
                    item {
                        ListItem(
                            headlineContent = { Text(str(R.string.song_cache)) },
                            supportingContent = { Text(Formatter.formatFileSize(LocalContext.current, s.cacheBytes.toLong())) },
                            trailingContent = { OutlinedButton({ core?.clearSongCache() }, enabled = s.cacheBytes > 0uL) { Text(str(R.string.clear)) } },
                        )
                    }
                    item { Header(str(R.string.covers)) }
                    item {
                        ChoiceRow(str(R.string.cover_quality), listOf("low" to str(R.string.quality_low), "medium" to str(R.string.quality_medium), "high" to str(R.string.quality_high)), look.coverQuality) { set("coverQuality", it) }
                    }
                    item { CoverCacheRow() }
                }
                "account" -> {
                    item {
                        ListItem(
                            headlineContent = { Text(s.user) },
                            supportingContent = { Text(core?.setting("url") ?: "") },
                            trailingContent = { OutlinedButton({ core?.logout() }) { Text(str(R.string.log_out)) } },
                        )
                    }
                }
            }
        }
    }
}
