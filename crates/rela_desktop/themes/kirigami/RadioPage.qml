// Internet radio stations (getInternetRadioStations). Click plays the stream; the queue waits.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

RadioData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: Kirigami.Units.largeSpacing * 3
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
        visible: root.stations !== null && root.stations.length > 0
        PageTitle { text: qsTr("Radyo") }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -Kirigami.Units.largeSpacing * 3; rightMargin: Kirigami.Units.largeSpacing * 3 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
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
                    anchors.fill: parent; radius: Kirigami.Units.cornerRadius
                    color: row.on ? Qt.rgba(Kirigami.Theme.highlightColor.r, Kirigami.Theme.highlightColor.g, Kirigami.Theme.highlightColor.b, 0.22) : hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent"
                }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
                    Item {
                        Layout.preferredWidth: 24; Layout.fillHeight: true
                        Bars { anchors.centerIn: parent; visible: row.on; active: Player.playing }
                        Plain { anchors.centerIn: parent; visible: !row.on; text: "◉"; color: Kirigami.Theme.disabledTextColor }
                    }
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: row.modelData.name; font.weight: row.on ? Font.Bold : Font.Medium }
                    Plain { text: row.sub; color: Kirigami.Theme.disabledTextColor; elide: Text.ElideRight; Layout.maximumWidth: 320 }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.stations !== null && root.stations.length === 0; text: qsTr("İstasyon yok") }
    Loading { loading: root.stations === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
