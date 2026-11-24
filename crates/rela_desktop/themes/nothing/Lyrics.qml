// Lyrics of a song (getLyricsBySongId). Synced lyrics follow the playback: the current line is
// lit and kept in the middle; clicking a line jumps there. Loads only while active.
// live: lyrics of a SUB/WAVE stream (from `subwave`), timed by the stream clock; a stream has no seek.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import shared
import Rela

LyricsData {
    id: ly

    ListView {
        anchors.fill: parent
        clip: true
        spacing: 10
        model: ly.lines
        currentIndex: ly.line
        highlightRangeMode: ly.synced ? ListView.ApplyRange : ListView.NoHighlightRange
        preferredHighlightBegin: height / 2 - 24; preferredHighlightEnd: height / 2 + 24
        highlightMoveDuration: 450; highlightMoveVelocity: -1 // glide to the line at a fixed pace
        topMargin: 16; bottomMargin: 16
        ScrollBar.vertical: Scroll {}
        delegate: Text {
            required property var modelData
            required property int index
            id: lyricLine
            width: ListView.view.width - 24; x: 12
            text: ly.lineText(index, App.theme.fg, Qt.rgba(color.r, color.g, color.b, 0.4))
            textFormat: ly.karaoke(index) ? Text.StyledText : Text.PlainText
            renderType: Text.CurveRendering // glyphs as curves: stay sharp while `scale` shrinks the other lines
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            font.family: App.theme.font
            readonly property bool on: !ly.synced || index === ly.line
            // one size for all lines (no reflow); the current line grows and brightens smoothly
            font.pixelSize: 16
            color: App.theme.fg
            scale: on ? 1 : 0.86
            opacity: on ? 1 : 0.4
            Behavior on scale { NumberAnimation { duration: 300; easing.type: Easing.OutCubic } }
            Behavior on opacity { NumberAnimation { duration: 300 } }

            TapHandler {
                enabled: ly.synced && !ly.live && Sync.canControl
                onTapped: Player.seek(Math.max(0, (lyricLine.modelData.start - ly.offset) / 1000))
            }
        }
    }
    Empty {
        visible: !ly.loading && !ly.failed && ly.songId !== "" && ly.lines.length === 0 && (!ly.live || ly.lyState !== "")
        text: ly.lyState === "auth" ? "PAROLA GEREKLİ" : ly.lyState === "off" ? "SÖZLER KAPALI" : "SÖZ BULUNAMADI"
    }
    Loading { loading: ly.loading; failed: ly.failed; onRetry: { ly.forId = ""; ly.load() } }
}
