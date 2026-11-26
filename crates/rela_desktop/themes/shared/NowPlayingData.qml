// Now playing: big square cover and the title in dot-matrix type; queue and similar songs on the right.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var args
    readonly property var w: Window.window
    readonly property var cur: Player.current
    readonly property bool live: !!Player.station.id && Subwave.active // a SUB/WAVE station is playing
    readonly property var sw: Subwave.now
    function clock(t) { t = Math.max(0, Math.floor(t || 0)); return Math.floor(t / 60) + ":" + String(t % 60).padStart(2, "0") }
    function clockLong(t) {
        t = Math.max(0, Math.floor(t || 0))
        return t < 3600 ? clock(t) : Math.floor(t / 3600) + ":" + String(Math.floor(t % 3600 / 60)).padStart(2, "0") + ":" + String(t % 60).padStart(2, "0")
    }
    // apart from strip: strip follows the position, this only the queue
    readonly property real total: Player.queue.reduce((a, s) => a + (s.duration || 0), 0)
    // "Çalıyor (3/11) 0:21/5:57 · Toplam süre: 52:47 | FLAC 44.1 kHz, 891 kbps"
    readonly property string strip: {
        if (Player.station.id) return Player.streamInfo // a live stream: what mpv decodes, no queue or timeline
        if (!cur.id) return ""
        const kbps = Player.bitrate || cur.bitRate
        const fmt = !cur.suffix ? "" : " | " + cur.suffix.toUpperCase() + (cur.samplingRate ? " " + cur.samplingRate / 1000 + " kHz" : "")
                                       + (kbps ? ", " + kbps + " kbps" : "")
        return (Player.playing ? qsTr("Çalıyor") : qsTr("Duraklatıldı")) + " (" + (Player.index + 1) + "/" + Player.queueLength + ") "
             + clock(Player.position) + "/" + clock(Player.duration)
             + " · " + qsTr("Toplam süre: %1").arg(clockLong(root.total)) + fmt
    }
}
