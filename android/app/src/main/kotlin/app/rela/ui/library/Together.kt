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
import app.rela.R
import app.rela.core.Rela
import app.rela.core.text
import app.rela.str
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
    Page(str(R.string.listen_together), root = true) { pad ->
        Column(Modifier.padding(top = pad.calculateTopPadding())) {
            PrimaryTabRow(tab) {
                Tab(tab == 0, { tab = 0 }, text = { Text(str(R.string.people)) })
                Tab(tab == 1, { tab = 1 }, text = { Text(str(R.string.settings)) })
            }
            if (tab == 1) {
                SwitchRow(str(R.string.allow_control), str(R.string.allow_control_hint), sync?.allowControl == true) {
                    core?.setAllowControl(it)
                }
                SwitchRow(str(R.string.hidden), str(R.string.hidden_hint), sync?.hidden == true) { core?.setHidden(it) }
                return@Column
            }
            LazyColumn(contentPadding = PaddingValues(bottom = pad.calculateBottomPadding())) {
                if (sync?.available != true) item { Hint(str(R.string.relay_unreachable)) }
                if (following.isNotEmpty()) item {
                    ListItem(
                        headlineContent = { Text(str(R.string.listening_with, following), fontWeight = FontWeight.SemiBold) },
                        supportingContent = { Text(str(R.string.leave_hint)) },
                        trailingContent = { Button({ core?.unfollow() }) { Text(str(R.string.leave)) } },
                    )
                }
                item { Hint(str(R.string.follow_hint)) }
                if (sync?.available == true && users.isEmpty()) item { Hint(str(R.string.nobody_online)) }
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
                                name + if (mine) "  →  " + str(R.string.listening_to_you) else u["following"].text().let { if (it.isEmpty()) "" else "  →  $it" },
                                color = if (mine) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                                fontWeight = if (mine) FontWeight.SemiBold else null,
                            )
                        },
                        supportingContent = if (what.isEmpty()) null else ({ Text(what, maxLines = 1) }),
                        trailingContent = {
                            if (!mine) FilledTonalButton({ core?.follow(name) }, enabled = following != root) {
                                Text(str(if (following == root) R.string.listening else R.string.listen))
                            }
                        },
                    )
                }
            }
        }
    }
}
