// "Bilgileri göster…": one song's tags and file details (getSong) as label / value rows; empty fields are left out.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Dialog {
    id: d
    title: qsTr("Parça Bilgisi")
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(720, Overlay.overlay ? Overlay.overlay.width - 40 : 720)
    height: Math.min(600, Overlay.overlay ? Overlay.overlay.height - 40 : 600)
    standardButtons: Dialog.Close
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
            [qsTr("Parça Adı"), s.title],
            [qsTr("Albüm"), s.album],
            [qsTr("Sanatçılar"), names(s.artists) || s.artist],
            [qsTr("Albüm sanatçıları"), names(s.albumArtists) || s.displayAlbumArtist],
            [qsTr("Besteciler"), s.displayComposer || composers.join(", ") || s.composer],
            [qsTr("Türler"), names(s.genres) || s.genre],
            [qsTr("Süre"), s.duration ? clock(s.duration) : ""],
            [qsTr("Yorum"), s.comment],
            [qsTr("Yıl"), s.year],
            [qsTr("Parça numarası"), s.track],
            [qsTr("Disk numarası"), s.discNumber],
            [qsTr("Dosya yolu"), s.path],
            [qsTr("İçerik türü"), s.contentType],
            [qsTr("Dosya tipi"), s.suffix],
            [qsTr("Bit hızı"), s.bitRate ? s.bitRate + " kbps" : ""],
            [qsTr("Örnekleme"), s.samplingRate ? s.samplingRate / 1000 + " kHz" : ""],
            [qsTr("Bit derinliği"), s.bitDepth ? s.bitDepth + " bit" : ""],
            [qsTr("Kanallar"), s.channelCount],
            [qsTr("Boyut"), s.size ? Qt.locale().formattedDataSize(s.size, 1, Locale.DataSizeTraditionalFormat) : ""],
            [qsTr("Çalma sayısı"), s.playCount],
            [qsTr("Eklenme"), s.created ? new Date(s.created).toLocaleDateString(Qt.locale(), Locale.ShortFormat) : ""],
            ["ReplayGain", gain.trackGain !== undefined ? qsTr("parça %1 dB").arg(gain.trackGain) + (gain.albumGain !== undefined ? ", " + qsTr("albüm %1 dB").arg(gain.albumGain) : "") : ""],
        ]
        return all.filter(r => r[1] !== undefined && r[1] !== null && String(r[1]) !== "" && String(r[1]) !== "0")
                  .map(r => [r[0], String(r[1])])
    }

    contentItem: Flickable {
        clip: true
        contentHeight: list.implicitHeight
        ScrollBar.vertical: ScrollBar {}
        ColumnLayout {
            id: list
            objectName: "infoRows"
            width: d.availableWidth - 16

            spacing: 10
            Plain { visible: d.failed; text: qsTr("Yüklenemedi"); color: palette.placeholderText }
            Repeater {
                model: d.rows
                RowLayout {
                    required property var modelData
                    id: r
                    Layout.fillWidth: true
                    spacing: 16
                    ToolButton {
                        Layout.preferredWidth: 28; Layout.preferredHeight: 28
                        opacity: r.modelData[0] === qsTr("Dosya yolu") ? 1 : 0
                        enabled: opacity > 0
                        icon.name: "edit-copy"
                        ToolTip.visible: hovered; ToolTip.text: qsTr("Kopyala"); ToolTip.delay: 500
                        onClicked: Player.copy(r.modelData[1])
                    }
                    Plain {
                        Layout.preferredWidth: 150
                        horizontalAlignment: Text.AlignRight
                        text: r.modelData[0]; font.weight: Font.DemiBold
                    }
                    Plain { Layout.fillWidth: true; text: r.modelData[1]; wrapMode: Text.WrapAnywhere }
                }
            }
        }
    }
}
