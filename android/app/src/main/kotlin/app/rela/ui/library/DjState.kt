package app.rela.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.rela.R
import app.rela.core.Rela
import app.rela.core.text
import app.rela.plural
import app.rela.str
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonPrimitive

private fun JsonObject.num(key: String) = runCatching { this[key]?.jsonPrimitive?.doubleOrNull }.getOrNull() ?: 0.0
private fun fmt(v: Double) = if (v == 0.0) "–" else "%.2f".format(v)

/** What the DJ weighed for its last pick: the sitting it read, then the songs, best first; a tap shows a song's parts, a pull reloads. */
@Composable
fun DjState() {
    var tick by remember { mutableIntStateOf(0) }
    var state by remember { mutableStateOf<JsonObject?>(null) }
    LaunchedEffect(tick) { state = Rela.local("djState") as? JsonObject }
    val rows = (state?.get("rows") as? JsonArray)?.mapNotNull { it as? JsonObject } ?: emptyList()
    var open by remember { mutableStateOf("") }
    Page(str(R.string.dj_state), onRefresh = { tick++ }) { pad -> // pull to refresh
        LazyColumn(contentPadding = pad) {
            val st = state
            if (st == null || rows.isEmpty()) {
                item { Text(str(R.string.dj_state_empty), Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
                return@LazyColumn
            }
            item {
                val session = (st["session"] as? JsonArray)?.mapNotNull { it as? JsonObject }?.joinToString(", ") { t ->
                    val w = t.num("weight")
                    t["tag"].text() + " " + (if (w > 0) "+" else "") + "%.1f".format(w)
                }.orEmpty()
                val seed = (st["seed"] as? JsonObject)?.get("title").text()
                Text(
                    listOfNotNull(
                        str(R.string.dj_searching).takeIf { st["searching"].text() == "true" },
                        str(R.string.dj_seed, seed.ifEmpty { "–" }),
                        str(R.string.dj_session, session).takeIf { session.isNotEmpty() },
                    ).joinToString("\n"),
                    Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                    style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            items(rows, key = { it["id"].text() }) { r ->
                val id = r["id"].text()
                val picked = r["picked"].text().isNotEmpty()
                ListItem(
                    headlineContent = { Text(r["title"].text(), maxLines = 1, overflow = TextOverflow.Ellipsis, fontWeight = if (picked) FontWeight.SemiBold else null) },
                    supportingContent = {
                        val parts = if (open == id) listOf(
                            str(R.string.dj_part_fit, fmt(r.num("fit"))), str(R.string.dj_part_taste, fmt(r.num("taste"))),
                            str(R.string.dj_part_similar, fmt(r.num("similar"))), str(R.string.dj_part_fresh, fmt(r.num("fresh"))),
                            str(R.string.dj_part_recent, fmt(r.num("recent"))), str(R.string.dj_part_feedback, fmt(r.num("feedback"))),
                            str(R.string.dj_disliked).takeIf { r["disliked"].text() == "true" },
                            str(R.string.dj_skips, "%.1f".format(r.num("skip"))).takeIf { r.num("skip") > 0 },
                            r["lastHeardDays"].text().toIntOrNull()?.let { plural(R.plurals.dj_heard_days_ago, it) },
                        ).filterNotNull().joinToString("\n") else ""
                        Text(listOf(r["artist"].text(), r["why"].text()).filter { it.isNotEmpty() }.joinToString(" · ") + if (parts.isEmpty()) "" else "\n$parts")
                    },
                    trailingContent = { Text(fmt(r.num("total")), color = if (r["disliked"].text() == "true") MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface) },
                    colors = ListItemDefaults.colors(containerColor = if (picked) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surface),
                    modifier = Modifier.clickable { open = if (open == id) "" else id },
                )
            }
        }
    }
}
