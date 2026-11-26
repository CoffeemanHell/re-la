// Internet radio stations (getInternetRadioStations). Click plays the stream; the queue waits.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

RadioData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        visible: root.stations !== null && root.stations.length > 0
        PageTitle { text: qsTr("Radyo") }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.stations || []
            delegate: Item {
                id: row
                required property var modelData
                readonly property bool on: Player.station.id === modelData.id
                // SUB/WAVE is asked at the home page, or at the stream's own address when none is set
                readonly property string home: modelData.homePageUrl || modelData.originUrl || ""
                readonly property var peek: Subwave.peeked[home]
                // SUB/WAVE: what is on air now; any other station: its home page
                readonly property string sub: peek && peek.title
                    ? [peek.title, peek.artist].filter(x => x).join(" — ") + (peek.dj ? " · " + peek.dj : "") : modelData.homePageUrl || ""
                Component.onCompleted: if (home) Subwave.probe(home)
                width: ListView.view.width - ListView.view.rightMargin; height: 52
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { enabled: Sync.canControl; onTapped: Player.playStation(row.modelData.id) }
                Rectangle {
                    anchors.fill: parent; radius: 4
                    color: row.on ? Qt.rgba(palette.highlight.r, palette.highlight.g, palette.highlight.b, 0.22) : hov.hovered ? palette.alternateBase : "transparent"
                }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                    spacing: 12
                    Item {
                        Layout.preferredWidth: 24; Layout.fillHeight: true
                        Bars { anchors.centerIn: parent; visible: row.on; active: Player.playing }
                        Plain { anchors.centerIn: parent; visible: !row.on; text: "◉"; color: palette.placeholderText }
                    }
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name; font.weight: row.on ? Font.Bold : Font.Medium }
                    Plain { text: row.sub; color: palette.placeholderText; elide: Text.ElideRight; Layout.maximumWidth: 320 }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.stations !== null && root.stations.length === 0; text: qsTr("İstasyon yok") }
    Loading { loading: root.stations === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
