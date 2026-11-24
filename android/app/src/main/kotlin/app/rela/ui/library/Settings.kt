package app.rela.ui.library

import android.text.format.Formatter
import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.toShape
import androidx.compose.material3.MaterialShapes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import app.rela.R
import app.rela.ui.LocalNav
import app.rela.ui.Screen
import app.rela.ui.open
import app.rela.ui.clearCoverCache
import app.rela.ui.coverCacheBytes
import app.rela.ui.coverCacheDir
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.rememberCoroutineScope
import coil3.SingletonImageLoader
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import app.rela.ui.tabIds
import app.rela.ui.tabs
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.core.Rela
import app.rela.playback.AudioPrefs
import app.rela.core.json
import app.rela.core.text
import app.rela.ui.LocalLook
import app.rela.ui.setOption
import kotlin.math.roundToInt

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

private val shapes = listOf("soft" to "Yumuşak", "square" to "Kare", "circle" to "Daire")

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

private data class Category(val page: String, val title: String, val sub: String, val icon: Int)

private val categories = listOf(
    Category("look", "Görünüm", "Tema, renkler, ızgara, kapak şekilleri", R.drawable.ic_palette_fill),
    Category("player", "Çalan ekran", "Kapağa dokununca ne olacağı", R.drawable.ic_play_arrow_fill),
    Category("bar", "Alt çubuk", "Görünen sekmeler", R.drawable.ic_bottom_navigation_fill),
    Category("playback", "Çalma", "Geçiş, DJ, ses, dinleme bildirimi", R.drawable.ic_music_note_2_fill),
    Category("data", "Veri ve depolama", "Kapak kalitesi ve önbellek", R.drawable.ic_data_table),
    Category("account", "Hesap", "Sunucu ve çıkış", R.drawable.ic_account_circle_fill),
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
        headlineContent = { Text("Kapak önbelleği") },
        supportingContent = { Text(Formatter.formatFileSize(ctx, bytes)) },
        trailingContent = {
            OutlinedButton({
                scope.launch {
                    withContext(Dispatchers.IO) { clearCoverCache(dir) }
                    SingletonImageLoader.get(ctx).memoryCache?.clear()
                    tick++
                }
            }) { Text("Temizle") }
        },
    )
}

/** Settings: a card per category, each opening its own page; Hakkında stands apart and only shows the version. */
@Composable
fun Settings() {
    val nav = LocalNav.current
    val ctx = LocalContext.current
    val version = remember { runCatching { ctx.packageManager.getPackageInfo(ctx.packageName, 0).versionName }.getOrNull() ?: "" }
    Page("Ayarlar") { pad ->
        LazyColumn(contentPadding = pad, verticalArrangement = Arrangement.spacedBy(2.dp)) {
            itemsIndexed(categories) { i, c ->
                CategoryCard(c.title, c.sub, c.icon, groupShape(i, categories.size)) { nav.open(Screen.SettingsPage(c.page)) }
            }
            item { Spacer(Modifier.height(24.dp)) }
            item { CategoryCard("Hakkında", "re-la, sürüm $version", R.drawable.ic_info_fill, groupShape(0, 1), null) }
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
    Page(categories.firstOrNull { it.page == page }?.title ?: "Ayarlar") { pad ->
        LazyColumn(contentPadding = pad) {
            when (page) {
                "look" -> {
                    item {
                        val themes = listOf("system" to "Sistem", "light" to "Açık", "dark" to "Koyu")
                        // anything but light/dark (the core's default is the desktop theme "nothing") follows the system
                        Segments(themes, s.theme.takeIf { it == "light" || it == "dark" } ?: "system", Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                            core?.setTheme(it)
                        }
                    }
                    item { SwitchRow("Saf siyah (AMOLED)", "Koyu temada arka plan tamamen siyah olur.", on("amoled")) { set("amoled", it) } }
                    item { SwitchRow("Kapaktan renk", "Albüm, sanatçı ve çalan ekranı kapağın rengini alır.", look.coverColors) { set("coverColors", it) } }
                    item { ChoiceRow("Izgara", listOf("2" to "2 sütun", "3" to "3 sütun", "4" to "4 sütun"), "${look.gridColumns}") { set("gridColumns", it.toInt()) } }
                    item { ChoiceRow("Kapak şekli", shapes, look.coverShape) { set("coverShape", it) } }
                    item { ChoiceRow("Sanatçı resmi", shapes, look.artistShape) { set("artistShape", it) } }
                    item { SwitchRow("Alfabetik kaydırma", "Sanatçılar listesinin kenarında harf çubuğu.", look.alphaScroll) { set("alphaScroll", it) } }
                }
                "playback" -> {
                    item { Header("Çalma") }
                    item {
                        SliderRow("Şarkı geçişi", s.crossfade.toInt(), 0..12, 1, hint = "0 = boşluksuz (gapless) geçiş.", text = { if (it == 0) "Kapalı" else "$it sn" }) {
                            core?.setCrossfade(it.toLong())
                        }
                    }
                    item { SwitchRow("Otomatik devam", "Kuyruk bitmeden benzer şarkılar eklenir. Nasıl seçileceğini DJ modu belirler.", on("autoContinue")) { set("autoContinue", it) } }
                    item {
                        SwitchRow("DJ modu", "Kapalıyken Navidrome'un benzer şarkıları olduğu gibi eklenir. Açıkken aşağıdaki ağırlıklarla seçilir.", s.dj) { core?.setDj(it) }
                    }
                    item { SliderRow("Dil bağlılığı", num("radioLanguage", 0), 0..100, 10, s.dj, text = { "%$it" }) { set("radioLanguage", it) } }
                    item { SliderRow("Tür bağlılığı", num("radioGenre", 0), 0..100, 10, s.dj, text = { "%$it" }) { set("radioGenre", it) } }
                    item { SliderRow("Her seferde eklenen", num("radioBatch", 10), 5..30, 5, s.dj, text = { "$it şarkı" }) { set("radioBatch", it) } }
                    item { Header("Ses (Deneysel)") }
                    item { AudioSwitch("Float çıkış hassasiyeti", "Ses 32 bit kayan noktalı olarak çıkar. Yeniden başlatma gerekir.", "floatOutput") }
                    item {
                        val offload = remember { AudioPrefs.offloadSupported() }
                        AudioSwitch(
                            "Audio offload",
                            if (offload) "Çözme işini ses donanımına bırakır, pil tasarrufu sağlar; bazı cihazlarda şarkı geçişiyle çakışabilir. FLAC için çalışmaz. Yeniden başlatma gerekir."
                            else "Bu cihaz desteklemiyor.",
                            "audioOffload",
                            enabled = offload,
                        )
                    }
                    item { Header("Sunucu") }
                    item { SwitchRow("Dinlenenleri gönder", "Şarkının yarısı (en fazla 4 dakikası) çalınca bir dinleme sayılır.", on("scrobble")) { set("scrobble", it) } }
                    item { SwitchRow("Şimdi çalıyor bilgisini gönder", "Şarkı başlayınca sunucuya ne çaldığını bildirir.", on("nowPlaying")) { set("nowPlaying", it) } }
                }
                "player" -> {
                    item {
                        ChoiceRow("Kapağa dokununca", listOf("lyrics" to "Sözler", "queue" to "Kuyruk", "none" to "Hiçbiri"), look.coverTap) { set("coverTap", it) }
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
                    item {
                        ChoiceRow("Kapak kalitesi", listOf("low" to "Düşük", "medium" to "Orta", "high" to "Yüksek"), look.coverQuality) { set("coverQuality", it) }
                    }
                    item { CoverCacheRow() }
                }
                "account" -> {
                    item {
                        ListItem(
                            headlineContent = { Text(s.user) },
                            supportingContent = { Text(core?.setting("url") ?: "") },
                            trailingContent = { OutlinedButton({ core?.logout() }) { Text("Çıkış yap") } },
                        )
                    }
                }
            }
        }
    }
}
