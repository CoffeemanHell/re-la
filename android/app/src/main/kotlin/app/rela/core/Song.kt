package app.rela.core

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.longOrNull

/** Song JSON as the core and the desktop themes see it: read the keys you need, the rest stays as it came. */
data class Song(val json: JsonObject) {
    val id get() = str("id")
    val title get() = str("title")
    val artist get() = str("artist")
    val album get() = str("album")
    val albumId get() = str("albumId")
    val artistId get() = str("artistId")
    val coverArt get() = str("coverArt")
    val suffix get() = str("suffix")
    val year get() = num("year").toInt()
    val track get() = num("track").toInt()
    val disc get() = num("discNumber").toInt().coerceAtLeast(1)
    /** Seconds. */
    val duration get() = num("duration")
    val starred get() = json["starred"] != null
    val rating get() = num("userRating").toInt()

    fun str(key: String) = json[key].text()
    fun num(key: String) = (json[key] as? JsonPrimitive)?.let { it.doubleOrNull ?: it.longOrNull?.toDouble() } ?: 0.0

    companion object {
        fun list(e: JsonElement?): List<Song> =
            (e as? JsonArray)?.mapNotNull { (it as? JsonObject)?.let(::Song) } ?: emptyList()
    }
}

/** A JSON value as text: strings as they are, numbers as written, anything else empty. */
fun JsonElement?.text(): String = (this as? JsonPrimitive)?.contentOrNull ?: ""

/** A key's array (Subsonic gives one object instead of a one-item array for some lists). */
fun JsonObject.listOf(key: String): List<JsonObject> = when (val v = this[key]) {
    is JsonArray -> v.mapNotNull { it as? JsonObject }
    is JsonObject -> listOf(v)
    else -> emptyList()
}

/** A key's songs (album `song`, playlist `entry`, ...). */
fun JsonObject.songs(key: String): List<Song> = listOf(key).map(::Song)

/** An album's or playlist's length as the desktop writes it: "42 dk", "1 sa 5 dk". */
fun length(seconds: Double): String {
    val d = seconds.toLong()
    return if (d >= 3600) "${d / 3600} sa ${d % 3600 / 60} dk" else "${Math.round(seconds / 60)} dk"
}

fun JsonElement?.obj(): JsonObject = (this as? JsonObject) ?: JsonObject(emptyMap())
