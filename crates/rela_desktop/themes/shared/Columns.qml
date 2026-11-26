// The track table's optional columns, in display order ("#" and the title always come first).
// key: stored in options.columns; w: preferred width; fill: takes spare width; align: left / right / center;
// kind: how the cell draws (text, artists, heart, stars, duration, date); minRow: dropped below this row width.
pragma Singleton
import QtQuick

QtObject {
    readonly property var all: [
        { key: "artist", title: qsTr("Sanatçı"), w: 180, fill: true, kind: "artists" },
        { key: "album", title: qsTr("Albüm"), w: 180, fill: true, minRow: 800, text: s => s.album },
        { key: "albumArtist", title: qsTr("Albüm sanatçısı"), w: 160, fill: true, text: s => s.displayAlbumArtist || (s.albumArtists || []).map(a => a.name).join(", ") },
        { key: "composer", title: qsTr("Besteci"), w: 160, fill: true, text: s => s.displayComposer || "" },
        { key: "genre", title: qsTr("Tür"), w: 120, fill: true, text: s => s.genres && s.genres.length ? s.genres.map(g => g.name).join(", ") : s.genre },
        { key: "duration", title: qsTr("Süre"), w: 48, align: "right", kind: "duration" },
        { key: "year", title: qsTr("Yıl"), w: 44, align: "center", minRow: 950, text: s => s.year },
        { key: "fav", title: qsTr("Fav."), w: 40, align: "center", kind: "heart" },
        { key: "rating", title: qsTr("Derecelendirme"), w: 104, align: "center", minRow: 700, kind: "stars" },
        { key: "plays", title: qsTr("Çalınma"), w: 60, align: "right", text: s => s.playCount },
        { key: "lastPlayed", title: qsTr("Son çalınma"), w: 96, kind: "date", text: s => s.played },
        { key: "comment", title: qsTr("Yorum"), w: 160, fill: true, text: s => s.comment },
        { key: "bpm", title: "BPM", w: 44, align: "right", text: s => s.bpm },
        { key: "bitrate", title: qsTr("Bit hızı"), w: 64, align: "right", minRow: 1100, text: s => s.bitRate },
        { key: "size", title: qsTr("Boyut"), w: 72, align: "right", text: s => s.size ? Qt.locale().formattedDataSize(s.size, 1, Locale.DataSizeTraditionalFormat) : "" },
        { key: "suffix", title: qsTr("Dosya tipi"), w: 72, text: s => s.suffix },
        { key: "created", title: qsTr("Eklenme tarihi"), w: 96, kind: "date", text: s => s.created },
        { key: "path", title: qsTr("Dosya yolu"), w: 220, fill: true, text: s => s.path },
    ]
    readonly property string defaults: "artist,album,duration,year,fav,rating,bitrate"

    // Keys to show for a row of this width: the chosen ones in table order, unknown keys skipped.
    function shown(chosen, width, hideAlbum) {
        const want = String(chosen === undefined ? defaults : chosen).split(",")
        return all.filter(c => want.indexOf(c.key) >= 0 && !(c.key === "album" && hideAlbum) && !(c.minRow && width < c.minRow))
                  .map(c => c.key).join(",")
    }
    function byKey(k) { return all.find(c => c.key === k) }
    // Cell text for text / date columns; "" when the song has no value (0 counts as none).
    function cell(c, s) {
        const v = c.text ? c.text(s) : ""
        if (v === undefined || v === null || v === 0 || v === "") return ""
        return c.kind === "date" ? new Date(v).toLocaleDateString(Qt.locale(), 1) : String(v) // 1 = Locale.ShortFormat
    }
}
