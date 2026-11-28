// Lyrics of a song (getLyricsBySongId). Synced lyrics follow the playback: the current line is
// highlighted and kept in the middle; clicking a line jumps there. Loads only while active.
// live: lyrics of a SUB/WAVE stream (from `subwave`), timed by the stream clock; a stream has no seek.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import shared
import Rela

LyricsData {
    id: ly

    ListView {
        id: view
        anchors.fill: parent
        clip: true
        spacing: 6
        model: ly.lines
        currentIndex: ly.line
        highlightRangeMode: ly.synced ? ListView.ApplyRange : ListView.NoHighlightRange
        preferredHighlightBegin: height / 2 - 24; preferredHighlightEnd: height / 2 + 24
        highlightMoveDuration: 450; highlightMoveVelocity: -1 // glide to the line at a fixed pace
        topMargin: 16; bottomMargin: 16
        ScrollBar.vertical: Scroll {}
        delegate: Label {
            required property var modelData
            required property int index
            id: lyricLine
            width: ListView.view.width - ListView.view.rightMargin - 24; x: 12
            text: ly.lineText(index, Kirigami.Theme.textColor, Qt.rgba(color.r, color.g, color.b, 0.4))
            textFormat: ly.karaoke(index) ? Text.StyledText : Text.PlainText
            renderType: Text.CurveRendering // glyphs as curves: stay sharp while `scale` shrinks the other lines
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            readonly property bool on: !ly.synced || index === ly.line
            // one font size for all lines (no reflow); the current line grows and brightens smoothly
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.3
            font.weight: Font.DemiBold
            color: Kirigami.Theme.textColor
            scale: on ? 1 : 0.88
            opacity: on ? 1 : 0.45
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
        text: ly.lyState === "auth" ? qsTr("Parola gerekli") : ly.lyState === "off" ? qsTr("Sözler kapalı") : qsTr("Söz bulunamadı")
    }
    Loading { loading: ly.loading; failed: ly.failed; onRetry: { ly.forId = ""; ly.load() } }
}
