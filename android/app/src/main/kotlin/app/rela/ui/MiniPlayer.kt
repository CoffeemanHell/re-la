package app.rela.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.R
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.json
import app.rela.core.text
import app.rela.str
import app.rela.ui.library.canControl
import uniffi.rela.PlayerState

/** Above the bottom bar while something is queued (or a station plays): tap opens Now Playing. */
@Composable
fun MiniPlayer(s: PlayerState, song: Song?, onOpen: () -> Unit) {
    val station = json(s.stationJson)
    val sw by Rela.subwave.collectAsStateWithLifecycle()
    val now = if (station.isNotEmpty() && sw?.active == true) json(sw!!.nowJson) else null // SUB/WAVE: what is on air
    val title = song?.title ?: now?.get("title").text().ifEmpty { station["name"].text() }
    if (title.isEmpty()) return
    val ctl = canControl() // following without control: the host decides
    val sub = if (station.isNotEmpty()) now?.get("artist").text().ifEmpty { str(R.string.radio) } else song?.artist ?: ""
    // a card floating over the nav bar, its progress a thin line along the bottom edge
    Surface(
        Modifier.padding(horizontal = 8.dp, vertical = 4.dp).clickable(onClick = onOpen),
        shape = RoundedCornerShape(16.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
    ) {
        Column {
            Row(
                Modifier.fillMaxWidth().padding(start = 8.dp, end = 4.dp, top = 8.dp, bottom = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Cover(song?.coverArt ?: "", 56.dp, Modifier.size(56.dp), RoundedCornerShape(8.dp))
                Column(Modifier.weight(1f)) {
                    Text(title, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(sub, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                Box {
                    IconButton({ Rela.core?.toggle() }, enabled = ctl, colors = IconButtonDefaults.iconButtonVibrantColors()) {
                        Icon(playPausePainter(!s.playing), str(if (s.playing) R.string.pause else R.string.play), Modifier.size(32.dp))
                    }
                }
                if (station.isEmpty()) {
                    IconButton({ Rela.core?.next() }, enabled = ctl) { Icon(painterResource(R.drawable.ic_skip_next_fill), str(R.string.next), Modifier.size(32.dp)) }
                }
            }
            LinearProgressIndicator(
                progress = { if (s.duration > 0) (s.position / s.duration).toFloat().coerceIn(0f, 1f) else 0f },
                modifier = Modifier.fillMaxWidth().height(2.dp),
                drawStopIndicator = {},
                gapSize = 0.dp,
            )
        }
    }
}
