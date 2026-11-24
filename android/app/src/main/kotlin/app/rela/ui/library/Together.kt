package app.rela.ui.library

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.core.Rela
import app.rela.core.text
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull

@Composable
fun Hint(text: String) = Text(text, Modifier.padding(horizontal = 16.dp, vertical = 8.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

@Composable
fun SwitchRow(label: String, hint: String, checked: Boolean, enabled: Boolean = true, onChange: (Boolean) -> Unit) = ListItem(
    headlineContent = { Text(label) },
    supportingContent = if (hint.isEmpty()) null else ({ Text(hint) }),
    trailingContent = { Switch(checked, onChange, enabled = enabled) },
)

/** Listening together: who is online and whom they follow (those listening to me first), and the relay switches. */
@Composable
fun Together() {
    val s by Rela.sync.collectAsStateWithLifecycle()
    val core = Rela.core
    var tab by rememberSaveable { mutableIntStateOf(0) }
    val sync = s
    val users = remember(sync?.usersJson) {
        (runCatching { Json.parseToJsonElement(sync?.usersJson ?: "[]") }.getOrNull() as? JsonArray)?.mapNotNull { it as? JsonObject } ?: emptyList()
    }
    val listeners = sync?.listeners ?: emptyList()
    val following = sync?.following ?: ""
    Page("Birlikte dinle", root = true) { pad ->
        Column(Modifier.padding(top = pad.calculateTopPadding())) {
            PrimaryTabRow(tab) {
                Tab(tab == 0, { tab = 0 }, text = { Text("Kişiler") })
                Tab(tab == 1, { tab = 1 }, text = { Text("Ayarlar") })
            }
            if (tab == 1) {
                SwitchRow("Katılanlar kontrol edebilir", "Seni dinleyenler şarkı değiştirebilir, duraklatabilir, kuyruğu düzenleyebilir.", sync?.allowControl == true) {
                    core?.setAllowControl(it)
                }
                SwitchRow("Gizli", "Çevrimiçi listesinde görünmezsin, kimse seni dinlemeye başlayamaz.", sync?.hidden == true) { core?.setHidden(it) }
                return@Column
            }
            LazyColumn(contentPadding = PaddingValues(bottom = pad.calculateBottomPadding())) {
                if (sync?.available != true) item { Hint("Birlikte dinleme sunucusuna bağlanılamadı.") }
                if (following.isNotEmpty()) item {
                    ListItem(
                        headlineContent = { Text("$following ile dinliyorsun", fontWeight = FontWeight.SemiBold) },
                        supportingContent = { Text("Ayrılınca aynı kuyrukla kendi başına devam edersin.") },
                        trailingContent = { Button({ core?.unfollow() }) { Text("Ayrıl") } },
                    )
                }
                item { Hint("Birini dinlemeye başlayınca onun çaldığını aynı saniyede duyarsın.") }
                if (sync?.available == true && users.isEmpty()) item { Hint("Şu an çevrimiçi kimse yok.") }
                items(users.sortedBy { it["user"].text() !in listeners }) { u ->
                    val name = u["user"].text()
                    val root = u["following"].text().ifEmpty { name }
                    val mine = name in listeners
                    val station = u["station"].text()
                    val playing = (u["playing"] as? JsonPrimitive)?.booleanOrNull == true
                    val what = if (station.isNotEmpty()) listOf("📻 $station", u["title"].text(), u["artist"].text()).filter { it.isNotEmpty() }.joinToString(" · ")
                    else listOf(u["title"].text(), u["artist"].text()).filter { it.isNotEmpty() }.joinToString(" · ").let { if (it.isEmpty()) "" else (if (playing) "▶  " else "❚❚  ") + it }
                    ListItem(
                        headlineContent = {
                            Text(
                                name + if (mine) "  →  seni dinliyor" else u["following"].text().let { if (it.isEmpty()) "" else "  →  $it" },
                                color = if (mine) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                                fontWeight = if (mine) FontWeight.SemiBold else null,
                            )
                        },
                        supportingContent = if (what.isEmpty()) null else ({ Text(what, maxLines = 1) }),
                        trailingContent = {
                            if (!mine) FilledTonalButton({ core?.follow(name) }, enabled = following != root) {
                                Text(if (following == root) "Dinliyorsun" else "Dinle")
                            }
                        },
                    )
                }
            }
        }
    }
}
