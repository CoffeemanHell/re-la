// "Bilgileri göster…": one song's tags and file details (getSong) as label / value rows; empty fields are left out.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Dialog {
    id: d
    title: qsTr("PARÇA BİLGİSİ")
    padding: 24
    background: Rectangle { color: App.theme.surface; border.width: 1; border.color: App.theme.line }
    header: Mono { text: d.title; padding: 24; bottomPadding: 0; color: App.theme.muted }
    footer: Item { implicitHeight: 54; Pill { anchors.right: parent.right; anchors.rightMargin: 24; anchors.verticalCenter: parent.verticalCenter; text: qsTr("KAPAT"); onClicked: d.close() } }
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(720, Overlay.overlay ? Overlay.overlay.width - 40 : 720)
    height: Math.min(600, Overlay.overlay ? Overlay.overlay.height - 40 : 600)
    property var rows: []   // [label, value]
    property bool failed: false

    function show(id) {
        rows = []
        failed = false
        open()
        Player.get(d, "getSong", { id: id }, r => r ? rows = build(r.song) : failed = true)
    }
    function names(list) { return (list || []).map(a => a.name).filter(n => n).join(", ") }
    function clock(t) { t = Math.floor(t || 0); return Math.floor(t / 60) + ":" + String(t % 60).padStart(2, "0") }
    function build(s) {
        const composers = (s.contributors || []).filter(c => c.role === "composer").map(c => c.artist && c.artist.name)
        const gain = s.replayGain || {}
        const all = [
            [qsTr("PARÇA ADI"), s.title],
            [qsTr("ALBÜM"), s.album],
            [qsTr("SANATÇILAR"), names(s.artists) || s.artist],
            [qsTr("ALBÜM SANATÇILARI"), names(s.albumArtists) || s.displayAlbumArtist],
            [qsTr("BESTECİLER"), s.displayComposer || composers.join(", ") || s.composer],
            [qsTr("TÜRLER"), names(s.genres) || s.genre],
            [qsTr("SÜRE"), s.duration ? clock(s.duration) : ""],
            [qsTr("YORUM"), s.comment],
            [qsTr("YIL"), s.year],
            [qsTr("PARÇA NUMARASI"), s.track],
            [qsTr("DİSK NUMARASI"), s.discNumber],
            [qsTr("DOSYA YOLU"), s.path],
            [qsTr("İÇERİK TÜRÜ"), s.contentType],
            [qsTr("DOSYA TİPİ"), s.suffix],
            [qsTr("BİT HIZI"), s.bitRate ? s.bitRate + " kbps" : ""],
            [qsTr("ÖRNEKLEME"), s.samplingRate ? s.samplingRate / 1000 + " kHz" : ""],
            [qsTr("BİT DERİNLİĞİ"), s.bitDepth ? s.bitDepth + " bit" : ""],
            [qsTr("KANALLAR"), s.channelCount],
            [qsTr("BOYUT"), s.size ? Qt.locale().formattedDataSize(s.size, 1, Locale.DataSizeTraditionalFormat) : ""],
            [qsTr("ÇALMA SAYISI"), s.playCount],
            [qsTr("EKLENME"), s.created ? new Date(s.created).toLocaleDateString(Qt.locale(), Locale.ShortFormat) : ""],
            ["REPLAYGAIN", gain.trackGain !== undefined ? qsTr("parça %1 dB").arg(gain.trackGain) + (gain.albumGain !== undefined ? ", " + qsTr("albüm %1 dB").arg(gain.albumGain) : "") : ""],
        ]
        return all.filter(r => r[1] !== undefined && r[1] !== null && String(r[1]) !== "" && String(r[1]) !== "0")
                  .map(r => [r[0], String(r[1])])
    }

    contentItem: Flickable {
        clip: true
        contentHeight: list.implicitHeight
        ScrollBar.vertical: Scroll {}
        ColumnLayout {
            id: list
            objectName: "infoRows"
            width: d.availableWidth - 16

            spacing: 10
            Mono { visible: d.failed; text: qsTr("YÜKLENEMEDİ"); color: App.theme.muted }
            Repeater {
                model: d.rows
                RowLayout {
                    required property var modelData
                    id: r
                    Layout.fillWidth: true
                    spacing: 16
                    Mono { Layout.preferredWidth: 150; horizontalAlignment: Text.AlignRight; text: r.modelData[0]; color: App.theme.muted }
                    Mono { Layout.fillWidth: true; text: r.modelData[1]; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone }
                    Pill { visible: r.modelData[0] === qsTr("DOSYA YOLU"); text: qsTr("KOPYALA"); onClicked: Player.copy(r.modelData[1]) }
                }
            }
        }
    }
}
