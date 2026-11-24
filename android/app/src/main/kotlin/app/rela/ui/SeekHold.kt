package app.rela.ui

import kotlin.math.abs

/**
 * A seek the core may not have taken yet: until then its snapshots still carry the old place, and following them
 * would throw the seek bar back for a moment after the finger lifts.
 */
data class SeekHold(val to: Double, val atMs: Long, val song: String)

/** Show the seek target instead of [reported] while the core is elsewhere: at most 3 s, same song. */
fun stillHeld(h: SeekHold, reported: Double, song: String, nowMs: Long): Boolean =
    h.song == song && abs(reported - h.to) >= 1.0 && nowMs - h.atMs < 3_000
