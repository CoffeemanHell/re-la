package app.rela.ui.library

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.R
import app.rela.core.Rela
import app.rela.core.json
import app.rela.core.listOf
import app.rela.core.songs
import app.rela.core.text
import kotlinx.coroutines.delay

/** search3 as you type (a short pause first): artists, albums, songs. A tapped song plays now. */
@Composable
fun Search() {
    var q by rememberSaveable { mutableStateOf("") }
    val query = q.trim()
    val l = load(query) {
        if (query.isEmpty()) return@load null
        delay(350) // typed on: this load is cancelled for the next one
        getObj("search3", mapOf("query" to query, "artistCount" to 6, "albumCount" to 12, "songCount" to 30), "searchResult3")
    }
    val r = l.value
    val artists = r?.listOf("artist") ?: emptyList()
    val albums = r?.listOf("album") ?: emptyList()
    val songs = r?.songs("song") ?: emptyList()
    Page("Ara") { pad ->
        LazyColumn(contentPadding = pad) {
            item {
                OutlinedTextField(
                    q, { q = it },
                    Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
                    placeholder = { Text("Şarkı, albüm, sanatçı") },
                    leadingIcon = { Icon(painterResource(R.drawable.ic_search), null) },
                    trailingIcon = { if (q.isNotEmpty()) IconButton({ q = "" }) { Icon(painterResource(R.drawable.ic_close), "Temizle") } },
                    singleLine = true,
                )
            }
            when {
                query.isEmpty() -> {}
                r == null && l.loading -> item { Text("Aranıyor…", Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
                r == null -> item { Text("Arama başarısız", Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
                artists.isEmpty() && albums.isEmpty() && songs.isEmpty() ->
                    item { Text("Sonuç yok", Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            if (artists.isNotEmpty()) {
                item { Header("Sanatçılar") }
                items(artists) { ArtistRow(it) }
            }
            if (albums.isNotEmpty()) {
                item { Header("Albümler") }
                item { LazyRow(contentPadding = PaddingValues(horizontal = 10.dp)) { items(albums) { StripCard(it) } } }
            }
            if (songs.isNotEmpty()) {
                item { Header("Şarkılar") }
                items(songs) { s -> SongRow(s, { add(listOf(s), 0) }) }
            }
        }
    }
}

/** Internet radio stations (getInternetRadioStations): a tap plays the stream, the queue waits. */
@Composable
fun Radio() {
    val st by Rela.player.collectAsStateWithLifecycle()
    val on = json(st?.stationJson ?: "{}")["id"].text()
    val ctl = canControl()
    val l = load { getList("getInternetRadioStations", emptyMap(), "internetRadioStations", "internetRadioStation") }
    val stations = l.value ?: emptyList()
    Page("Radyo", refreshing = l.loading && l.value != null, onRefresh = l.refresh, root = true) { pad ->
        Listing(stations.size, l.loading, l.failed, "İstasyon yok", l.refresh) {
            LazyColumn(contentPadding = pad) {
                items(stations) { s ->
                    val id = s["id"].text()
                    val playing = id == on
                    ListItem(
                        headlineContent = {
                            Text(s["name"].text(), fontWeight = if (playing) FontWeight.Bold else null,
                                color = if (playing) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface)
                        },
                        supportingContent = s["homePageUrl"].text().takeIf { it.isNotEmpty() }?.let { { Text(it, maxLines = 1) } },
                        leadingContent = { Icon(painterResource(R.drawable.ic_radio), null) },
                        modifier = Modifier.clickable(enabled = ctl) { Rela.core?.playStation(id) },
                    )
                }
            }
        }
    }
}
