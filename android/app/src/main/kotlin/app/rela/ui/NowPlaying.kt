package app.rela.ui

import android.os.SystemClock
import androidx.activity.compose.BackHandler
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonGroup
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.FilledTonalIconToggleButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.IconButtonShapes
import androidx.compose.material3.IconToggleButton
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderState
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.rela.R
import app.rela.core.Lyrics
import app.rela.core.Rela
import app.rela.core.Song
import app.rela.core.json
import app.rela.core.listOf
import app.rela.core.text
import app.rela.playback.queueSong
import app.rela.plural
import app.rela.str
import app.rela.ui.library.Fav
import app.rela.ui.library.ItemMenu
import app.rela.ui.library.Listing
import app.rela.ui.library.SongRow
import app.rela.ui.library.add
import app.rela.ui.library.canControl
import app.rela.ui.library.getList
import app.rela.ui.library.load
import app.rela.ui.library.rememberFav
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlin.math.sign
import kotlinx.coroutines.launch
import uniffi.rela.PlayerState
import uniffi.rela.SubwaveState
import uniffi.rela.Wave

/** m:ss (h:mm:ss past an hour). */
fun clock(seconds: Double): String {
    val t = seconds.coerceAtLeast(0.0).toLong()
    val (h, m, s) = Triple(t / 3600, t / 60 % 60, t % 60)
    return if (h > 0) "%d:%02d:%02d".format(h, m, s) else "%d:%02d".format(m, s)
}

/** "FLAC · 44.1 kHz · 891 kbps": the file as the server describes it, the bitrate mpv/ExoPlayer measured if any. */
fun format(song: Song, bitrate: Long): String {
    val kbps = if (bitrate > 0) bitrate else song.num("bitRate").toLong()
    val sr = song.num("samplingRate")
    return listOfNotNull(
        song.suffix.uppercase().ifEmpty { null },
        if (sr > 0) "${(sr / 1000).toString().removeSuffix(".0")} kHz" else null,
        if (kbps > 0) "$kbps kbps" else null,
    ).joinToString(" · ")
}

/** The player page, in the cover's colors when that setting is on. */
@Composable
fun NowPlaying() {
    val st by Rela.player.collectAsStateWithLifecycle()
    val queue by Rela.queue.collectAsStateWithLifecycle()
    val sw by Rela.subwave.collectAsStateWithLifecycle()
    val s = st ?: return
    val station = json(s.stationJson)
    val cover = if (station.isNotEmpty()) (if (sw?.active == true) json(sw!!.nowJson)["id"].text() else "") else queueSong(s, queue)?.coverArt ?: ""
    CoverTheme(cover) { Surface(Modifier.fillMaxSize()) { Player() } }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Player() {
    val nav = LocalNav.current
    val st by Rela.player.collectAsStateWithLifecycle()
    val queue by Rela.queue.collectAsStateWithLifecycle()
    val s = st ?: return
    val core = Rela.core
    val song = queueSong(s, queue)
    val station = json(s.stationJson)
    val sw by Rela.subwave.collectAsStateWithLifecycle()
    val live = station.isNotEmpty() && sw?.active == true // a SUB/WAVE station: its DJ, lyrics, heart and requests
    val now = if (live) json(sw!!.nowJson) else null
    var lyrics by rememberSaveable { mutableStateOf(false) }
    var sheet by rememberSaveable { mutableStateOf(false) }
    var request by rememberSaveable { mutableStateOf(false) }
    var fade by rememberSaveable { mutableStateOf(false) }
    val hasLyrics = live || (song != null && station.isEmpty())
    BackHandler(lyrics && hasLyrics) { lyrics = false } // back closes the lyrics layer, not the player
    // one favourite state for ☆ and the ⋯ menu
    val songFav = if (song != null && station.isEmpty()) rememberFav("song", song.json) else null
    if (lyrics && hasLyrics) {
        // full screen: the lines over the whole page, ⌄ back to the player
        Box(Modifier.fillMaxSize().safeDrawingPadding()) {
            Box(Modifier.fillMaxSize().padding(horizontal = 24.dp), contentAlignment = Alignment.Center) {
                if (live) LiveLyrics(sw!!) else LyricsView(song!!.id, s.position)
            }
            FilledTonalIconButton({ lyrics = false }, Modifier.padding(12.dp)) { Icon(painterResource(R.drawable.ic_keyboard_arrow_down), str(R.string.close_lyrics)) }
        }
        return
    }
    val ctl = canControl() // following without control: the host decides
    Column(
        Modifier.fillMaxSize().safeDrawingPadding().padding(horizontal = 24.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            // shortcut: the track's cover from this Navidrome by its subsonic id; a station on another server shows none
            val onCover: (() -> Unit)? = when (LocalLook.current.coverTap) { // Settings → Çalan ekran
                "lyrics" -> if (hasLyrics) ({ lyrics = true }) else null
                "queue" -> if (station.isEmpty()) ({ sheet = true }) else null
                else -> null
            }
            val art = RoundedCornerShape(24.dp)
            // swipe the cover: left for the next song, right for the previous; it follows the finger and slides out
            val scope = rememberCoroutineScope()
            val swipe = remember { Animatable(0f) }
            val swipeable = ctl && station.isEmpty()
            Cover(
                if (live) now!!["id"].text() else song?.coverArt ?: "", 360.dp,
                Modifier.aspectRatio(1f)
                    .graphicsLayer { translationX = swipe.value; alpha = 1f - (abs(swipe.value) / size.width).coerceIn(0f, 0.6f) }
                    .pointerInput(swipeable) {
                        if (swipeable) detectHorizontalDragGestures(
                            onDragEnd = {
                                scope.launch {
                                    val w = size.width.toFloat()
                                    val dir = sign(swipe.value)
                                    if (abs(swipe.value) > w / 4) {
                                        swipe.animateTo(dir * w, tween(150))
                                        if (dir < 0) core?.next() else core?.prev()
                                        swipe.snapTo(-dir * w / 2)
                                    }
                                    swipe.animateTo(0f)
                                }
                            },
                            onDragCancel = { scope.launch { swipe.animateTo(0f) } },
                        ) { change, dx ->
                            change.consume()
                            scope.launch { swipe.snapTo(swipe.value + dx) }
                        }
                    }
                    .clip(art).then(if (onCover != null) Modifier.clickable(onClick = onCover) else Modifier), art,
            )
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Column(Modifier.weight(1f)) {
                Text(
                    now?.get("title").text().ifEmpty { null } ?: song?.title ?: station["name"].text(),
                    Modifier.clickable(enabled = station.isEmpty() && !song?.albumId.isNullOrEmpty()) {
                        nav.removeAt(nav.lastIndex)
                        nav.open(Screen.Album(song!!.albumId))
                    },
                    style = MaterialTheme.typography.titleLarge, maxLines = 2, overflow = TextOverflow.Ellipsis,
                )
                Text(
                    if (live) now!!["artist"].text() else if (station.isNotEmpty()) str(R.string.radio) else song?.artist ?: "",
                    Modifier.clickable(enabled = !song?.artistId.isNullOrEmpty()) {
                        nav.removeAt(nav.lastIndex)
                        nav.open(Screen.Artist(song!!.artistId))
                    },
                    style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                if (live) {
                    val dj = now!!["dj"].text()
                    if (dj.isNotEmpty()) Text(
                        listOf("DJ $dj", now["show"].text(), "${now["listeners"].text().ifEmpty { "0" }} dinleyici").filter { it.isNotEmpty() }.joinToString(" · "),
                        style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (!sw!!.online) Text(str(R.string.no_connection), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.error)
                }
            }
            // ☆: the song's favourite; on SUB/WAVE the station's heart
            if (live) {
                val w = sw!!
                FilledTonalIconToggleButton(w.liked, { core?.subwaveLike() }, enabled = w.likeEnabled && !w.liked && w.likeBlockedMs == 0L) {
                    Text(if (w.liked) "♥" else "♡", style = MaterialTheme.typography.titleLarge)
                }
            } else if (songFav != null) {
                val fav = songFav
                FilledTonalIconToggleButton(fav.on, { fav.toggle(quiet = true) }) {
                    Icon(painterResource(if (fav.on) R.drawable.ic_star_fill else R.drawable.ic_star), str(if (fav.on) R.string.remove_favorite else R.string.add_favorite))
                }
            }
            More(song.takeIf { station.isEmpty() }, songFav, live, s.dj, onFade = { fade = true }, onRequest = { request = true })
        }
        // the relay's waveform of this song; a new song drops the old one (the effect of the old id is cancelled)
        val waveId = song?.id.orEmpty()
        var wave by remember { mutableStateOf<Wave?>(null) }
        LaunchedEffect(waveId) {
            wave = null
            if (waveId.isNotEmpty()) wave = core?.waveform(waveId)
        }
        if (station.isEmpty() && s.duration > 0) {
            // the slider follows the core until the finger lifts, then seeks once
            val sl = remember(s.duration) { SliderState(0f, 0, 0f..s.duration.toFloat()) }
            var hold by remember { mutableStateOf<SeekHold?>(null) }
            if (!sl.isDragging) SideEffect {
                val h = hold
                val held = h != null && stillHeld(h, s.position, waveId, SystemClock.uptimeMillis())
                if (!held) hold = null
                sl.value = (if (held) h!!.to else s.position).toFloat().coerceIn(0f, s.duration.toFloat())
            }
            val pos = sl.value.toDouble()
            Column {
                Slider(
                    state = sl,
                    onValueChange = { sl.value = it },
                    enabled = ctl,
                    onValueChangeFinished = {
                        hold = SeekHold(sl.value.toDouble(), SystemClock.uptimeMillis(), waveId)
                        core?.seek(sl.value.toDouble())
                    },
                    track = {
                        val progress = { (sl.value / s.duration.toFloat()).coerceIn(0f, 1f) }
                        val w = wave
                        // the song's waveform; until it comes (or without the relay): waves while playing, flat while paused
                        if (w != null) WaveTrack(w, progress)
                        else LinearWavyProgressIndicator(
                            progress = progress,
                            modifier = Modifier.fillMaxWidth(),
                            amplitude = { if (s.playing) 1f else 0f },
                        )
                    },
                )
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(clock(pos), style = MaterialTheme.typography.labelMedium)
                    Text(clock(s.duration), style = MaterialTheme.typography.labelMedium)
                }
            }
        }
        val info = if (station.isNotEmpty()) s.streamInfo else song?.let { format(it, s.bitrate) } ?: ""
        if (info.isNotEmpty() || s.signalPath.isNotEmpty()) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (info.isNotEmpty()) Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surfaceContainerHigh) {
                    Text(info, Modifier.padding(horizontal = 12.dp, vertical = 4.dp), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                if (s.signalPath.isNotEmpty() && s.signalPath != info) {
                    Text(s.signalPath, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        // a button group: the pressed button widens and its neighbours give way; the play button is a circle
        // while paused and a rounded square while playing
        ButtonGroup(overflowIndicator = {}, modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
            fun item(content: @Composable (MutableInteractionSource, Modifier) -> Unit) = customItem({
                val src = remember { MutableInteractionSource() }
                content(src, Modifier.animateWidth(src).align(Alignment.CenterVertically))
            }, {})
            item { src, m ->
                IconToggleButton(s.shuffle, { core?.setShuffle(it) }, modifier = m, enabled = ctl, shapes = IconButtonDefaults.toggleableShapes(), interactionSource = src) {
                    Icon(painterResource(if (s.shuffle) R.drawable.ic_shuffle_on_fill else R.drawable.ic_shuffle), str(R.string.shuffle))
                }
            }
            item { src, m ->
                FilledTonalIconButton({ core?.prev() }, modifier = m.size(64.dp, 56.dp), enabled = ctl && station.isEmpty(), shapes = IconButtonDefaults.shapes(), interactionSource = src) {
                    Icon(painterResource(R.drawable.ic_skip_previous_fill), str(R.string.previous), Modifier.size(32.dp))
                }
            }
            item { src, m ->
                val round = if (s.playing) 28.dp else 48.dp
                FilledIconButton(
                    { core?.toggle() }, modifier = m.size(96.dp), enabled = ctl, interactionSource = src,
                    shapes = IconButtonShapes(RoundedCornerShape(animateDpAsState(round).value), RoundedCornerShape(20.dp)),
                ) {
                    Icon(playPausePainter(!s.playing), str(if (s.playing) R.string.pause else R.string.play), Modifier.size(44.dp))
                }
            }
            item { src, m ->
                FilledTonalIconButton({ core?.next() }, modifier = m.size(64.dp, 56.dp), enabled = ctl && station.isEmpty(), shapes = IconButtonDefaults.shapes(), interactionSource = src) {
                    Icon(painterResource(R.drawable.ic_skip_next_fill), str(R.string.next), Modifier.size(32.dp))
                }
            }
            item { src, m ->
                IconToggleButton(s.repeat != 0L, { core?.setRepeat((s.repeat + 1) % 3) }, modifier = m, enabled = ctl, shapes = IconButtonDefaults.toggleableShapes(), interactionSource = src) {
                    Icon(
                        painterResource(when (s.repeat) {
                            1L -> R.drawable.ic_repeat_on_fill
                            2L -> R.drawable.ic_repeat_one_on_fill
                            else -> R.drawable.ic_repeat
                        }),
                        str(R.string.repeat),
                    )
                }
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilledTonalIconButton(rememberBack(), shapes = IconButtonDefaults.shapes()) { Icon(painterResource(R.drawable.ic_keyboard_arrow_down), str(R.string.close)) }
            Text(str(R.string.now_playing), Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
            FilledTonalIconButton({ lyrics = true }, enabled = hasLyrics, shapes = IconButtonDefaults.shapes()) { Icon(painterResource(R.drawable.ic_lyrics), str(R.string.lyrics)) }
            FilledTonalIconButton({ sheet = true }, enabled = station.isEmpty(), shapes = IconButtonDefaults.shapes()) { Icon(painterResource(R.drawable.ic_queue_music), str(R.string.queue)) }
        }
    }
    if (sheet) QueueSheet(s, queue) { sheet = false }
    if (request && live) RequestSheet(sw!!, s.user) { request = false }
    if (fade) CrossfadeDialog(s.crossfade) { fade = false }
}

/** Now Playing's ⋯: crossfade and DJ (İstek on SUB/WAVE) first, then the song's own menu when a song plays. */
@Composable
private fun More(song: Song?, fav: Fav?, live: Boolean, dj: Boolean, onFade: () -> Unit, onRequest: () -> Unit) {
    val trigger: @Composable (() -> Unit) -> Unit = { open -> FilledTonalIconButton(open) { Icon(painterResource(R.drawable.ic_more_horiz), str(R.string.more)) } }
    val extra: @Composable ColumnScope.(() -> Unit) -> Unit = { close ->
        if (live) DropdownMenuItem({ Text(str(R.string.send_request)) }, { close(); onRequest() }, leadingIcon = { Icon(painterResource(R.drawable.ic_send), null) })
        else {
            DropdownMenuItem({ Text(str(R.string.crossfade_menu)) }, { close(); onFade() }, leadingIcon = { Icon(painterResource(R.drawable.ic_graphic_eq), null) })
            DropdownMenuItem(
                { Text(str(R.string.dj_mode)) }, { close(); Rela.core?.setDj(!dj) },
                leadingIcon = { Text("DJ", fontWeight = FontWeight.Bold) }, trailingIcon = { if (dj) Icon(painterResource(R.drawable.ic_check), null) },
            )
        }
        if (song != null) HorizontalDivider()
    }
    if (song != null && fav != null) ItemMenu("song", song.json, fav = fav, trigger = trigger, extra = extra)
    else {
        var open by remember { mutableStateOf(false) }
        Box {
            trigger { open = true }
            DropdownMenu(open, { open = false }) { extra { open = false } }
        }
    }
}

/** SUB/WAVE's lyrics of the track on air, following the station's clock. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LiveLyrics(sw: SubwaveState) {
    val ly = remember(sw.lyricsJson) { Lyrics.of(json(sw.lyricsJson)) }
    when {
        ly.lines.isEmpty() && (sw.lyricsState == "loading" || sw.lyricsState == "retry") -> LoadingIndicator()
        ly.lines.isEmpty() -> Text(str(R.string.no_lyrics), color = MaterialTheme.colorScheme.onSurfaceVariant)
        else -> LyricLines(ly, sw.elapsedMs / 1000.0, seek = false)
    }
}

/** Ask the SUB/WAVE DJ for a song and follow the answer; what is coming up below. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RequestSheet(sw: SubwaveState, user: String, onDismiss: () -> Unit) {
    var text by rememberSaveable { mutableStateOf("") }
    var name by rememberSaveable { mutableStateOf(user) }
    val req = json(sw.requestJson)
    val state = req["state"].text()
    val upcoming = json(sw.nowJson).listOf("upcoming")
    val labels = mapOf(
            "pending" to str(R.string.request_pending), "done" to str(R.string.request_done), "rejected" to str(R.string.request_rejected),
            "failed" to str(R.string.request_failed), "unknown" to str(R.string.request_unknown),
        )
    ModalBottomSheet(onDismiss) {
        Column(Modifier.padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(text, { text = it }, Modifier.fillMaxWidth(), placeholder = { Text(str(R.string.request_hint)) })
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(name, { name = it }, Modifier.weight(1f), placeholder = { Text(str(R.string.your_name)) }, singleLine = true)
                Button({ Rela.core?.subwaveRequest(text.trim(), name.trim()) }, enabled = text.isNotBlank() && state != "pending") { Text(str(R.string.send)) }
            }
            labels[state]?.let { Text(it, fontWeight = FontWeight.SemiBold) }
            val track = listOf(req["title"].text(), req["artist"].text()).filter { it.isNotEmpty() }.joinToString(" — ")
            if (track.isNotEmpty()) Text(track)
            req["queuePosition"].text().toIntOrNull()?.takeIf { it > 0 }?.let { Text(str(R.string.queue_position, it), color = MaterialTheme.colorScheme.onSurfaceVariant) }
            req["message"].text().takeIf { it.isNotEmpty() }?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            if (upcoming.isNotEmpty()) Text(str(R.string.up_next), style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 8.dp))
        }
        LazyColumn {
            items(upcoming) { u ->
                ListItem(
                    headlineContent = { Text(u["title"].text(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    supportingContent = { Text(listOf(u["artist"].text(), u["requestedBy"].text()).filter { it.isNotEmpty() }.joinToString(" · ")) },
                )
            }
        }
    }
}

/** Crossfade seconds (0 off, up to 12): a dialog with a slider; 0 is gapless. */
@Composable
private fun CrossfadeDialog(seconds: Long, onDismiss: () -> Unit) {
    var v by remember(seconds) { mutableFloatStateOf(seconds.toFloat()) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (v.roundToInt() == 0) str(R.string.crossfade_off) else str(R.string.crossfade_length, plural(R.plurals.n_seconds, v.roundToInt()))) },
        text = { Slider(v, { v = it }, valueRange = 0f..12f, steps = 11, onValueChangeFinished = { Rela.core?.setCrossfade(v.roundToInt().toLong()) }) },
        confirmButton = { TextButton(onDismiss) { Text(str(R.string.ok)) } },
    )
}

/** getLyricsBySongId: synced lines follow the position and seek on tap; plain lyrics just scroll. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LyricsView(songId: String, position: Double) {
    val l = load(songId) { Rela.get("getLyricsBySongId", mapOf("id" to songId, "enhanced" to true))?.let(Lyrics::from) }
    val ly = l.value
    when {
        ly == null && l.loading -> LoadingIndicator()
        ly == null -> Text(str(R.string.lyrics_failed), color = MaterialTheme.colorScheme.onSurfaceVariant)
        ly.lines.isEmpty() -> Text(str(R.string.no_lyrics), color = MaterialTheme.colorScheme.onSurfaceVariant)
        else -> LyricLines(ly, position, seek = canControl())
    }
}

/**
 * The lines, the current one bold and a third down the view, its sung words lit when it has word timing;
 * [seek]: a tap jumps there (not on a live station).
 */
@Composable
private fun LyricLines(ly: Lyrics, position: Double, seek: Boolean) {
    val cur = ly.lineAt(position)
    val word = ly.wordAt(cur, position)
    val on = MaterialTheme.colorScheme.onSurface
    val off = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
    val list = rememberLazyListState()
    LaunchedEffect(cur) {
        if (cur < 0) return@LaunchedEffect
        // a slow glide to the next line while it is on screen; a seek far away jumps
        val item = list.layoutInfo.visibleItemsInfo.firstOrNull { it.index == cur }
        if (item != null) list.animateScrollBy(item.offset.toFloat(), tween(700, easing = FastOutSlowInEasing))
        else list.animateScrollToItem(cur)
    }
    // the padding puts the sung line a third down even near the start and end (and clear of the close button)
    BoxWithConstraints(Modifier.fillMaxSize()) { LazyColumn(Modifier.fillMaxSize(), state = list, contentPadding = PaddingValues(top = if (ly.synced) maxHeight / 3 else 72.dp, bottom = maxHeight * 2 / 3)) {
        itemsIndexed(ly.lines) { i, line ->
            val words = line.words
            val lit = !ly.synced || i == cur
            // the line in focus fades in and sharpens instead of snapping
            val color by animateColorAsState(if (lit) on else off, tween(500))
            val blur by animateDpAsState(if (lit) 0.dp else 4.dp, tween(500))
            Text(
                if (i == cur && words != null) buildAnnotatedString {
                    words.forEachIndexed { k, w -> withStyle(SpanStyle(color = if (k <= word) on else off)) { append(w.text) } }
                } else AnnotatedString(line.text.ifEmpty { "♪" }),
                Modifier.fillMaxWidth()
                    .clickable(enabled = seek && ly.synced) { Rela.core?.seek((line.startMs - ly.offsetMs).coerceAtLeast(0) / 1000.0) }
                    .padding(vertical = 10.dp)
                    .blur(blur), // the other lines out of focus
                style = if (ly.synced) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.bodyLarge,
                // one weight for every synced line: a line turning bold would rewrap and jump
                fontWeight = if (ly.synced) FontWeight.Bold else null,
                color = color,
                textAlign = TextAlign.Start,
            )
        }
    } }
}

/** The queue (tap to jump, drag the handle to move, ✕ to remove, clear upcoming) and songs like the current one. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun QueueSheet(s: PlayerState, queue: List<Song>, onDismiss: () -> Unit) {
    var tab by remember { mutableIntStateOf(0) }
    // full height: near the queue's end the list can't put the current song on top, half a sheet would hide it
    ModalBottomSheet(onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        PrimaryTabRow(tab) {
            Tab(tab == 0, { tab = 0 }, text = { Text(str(R.string.queue_count, queue.size)) })
            Tab(tab == 1, { tab = 1 }, text = { Text(str(R.string.similar)) })
        }
        if (tab == 0) QueueList(s, queue) else Similar(queue.getOrNull(s.index.toInt())?.id ?: "")
    }
}

@Composable
private fun QueueList(s: PlayerState, queue: List<Song>) {
    val core = Rela.core
    val ctl = canControl()
    val index = s.index.toInt()
    val list = rememberLazyListState(initialFirstVisibleItemIndex = index.coerceAtLeast(0))
    val rowPx = with(LocalDensity.current) { 72.dp.toPx() }
    var drag by remember { mutableIntStateOf(-1) }
    var dy by remember { mutableFloatStateOf(0f) }
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(str(R.string.queue_total, clock(queue.sumOf { it.duration })), Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurfaceVariant)
        TextButton({ core?.clearUpcoming() }, enabled = ctl && index < queue.size - 1) { Text(str(R.string.clear_upcoming)) }
    }
    LazyColumn(state = list) {
        itemsIndexed(queue) { i, song ->
            val now = i == index
            ListItem(
                headlineContent = {
                    Text(song.title, color = if (now) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                        fontWeight = if (now) FontWeight.SemiBold else null, maxLines = 1, overflow = TextOverflow.Ellipsis)
                },
                supportingContent = { Text(listOf(song.artist, clock(song.duration)).filter { it.isNotEmpty() }.joinToString(" · "), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                leadingContent = { Cover(song.coverArt, 40.dp, Modifier.size(40.dp)) },
                trailingContent = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        IconButton({ core?.remove(i.toLong()) }, enabled = ctl) { Icon(painterResource(R.drawable.ic_close), str(R.string.remove_from_queue)) }
                        Icon(
                            painterResource(R.drawable.ic_drag_handle), str(R.string.move),
                            Modifier.padding(8.dp).pointerInput(i, ctl, queue.size) {
                                if (!ctl) return@pointerInput
                                detectDragGestures(
                                    onDragStart = { drag = i; dy = 0f },
                                    onDrag = { c, d -> c.consume(); dy += d.y },
                                    onDragEnd = {
                                        val to = (i + (dy / rowPx).roundToInt()).coerceIn(0, queue.size - 1)
                                        if (to != i) core?.moveSong(i.toLong(), to.toLong())
                                        drag = -1
                                        dy = 0f
                                    },
                                    onDragCancel = { drag = -1; dy = 0f },
                                )
                            },
                        )
                    }
                },
                modifier = Modifier.height(72.dp).zIndex(if (drag == i) 1f else 0f)
                    .graphicsLayer { translationY = if (drag == i) dy else 0f }
                    .clickable(enabled = ctl) { core?.jump(i.toLong()) },
            )
        }
    }
}

@Composable
private fun Similar(songId: String) {
    val l = load(songId) { getList("getSimilarSongs", mapOf("id" to songId, "count" to 30), "similarSongs", "song")?.map(::Song) }
    val songs = l.value ?: emptyList()
    Listing(songs.size, l.loading, l.failed, str(R.string.no_similar), l.refresh) {
        LazyColumn { items(songs) { SongRow(it, { add(listOf(it), 0) }) } }
    }
}
