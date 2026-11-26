package app.rela.ui.library

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp
import app.rela.R

/** The Albümler sort sheet's rows: getAlbumList2 type → label (a string resource). */
val albumOrders = listOf(
    "alphabeticalByArtist" to R.string.order_by_artist,
    "alphabeticalByName" to R.string.order_by_name,
    "frequent" to R.string.order_frequent,
    "recent" to R.string.order_recent,
    "newest" to R.string.order_newest,
    "highest" to R.string.order_highest,
    "random" to R.string.random,
    "byYear" to R.string.year,
)

/** getAlbumList2 parameters (no paging): the starred filter wins; only byYear has a direction in Subsonic. */
fun albumParams(type: String, starred: Boolean, descending: Boolean): Map<String, Any> = when {
    starred -> mapOf("type" to "starred")
    type == "byYear" -> mapOf("type" to type, "fromYear" to if (descending) 9999 else 0, "toYear" to if (descending) 0 else 9999)
    else -> mapOf("type" to type)
}

/** The cover/artist shape settings: soft, square, circle. */
fun coverShape(name: String): Shape = when (name) {
    "square" -> RoundedCornerShape(4.dp)
    "circle" -> CircleShape
    else -> RoundedCornerShape(16.dp)
}
