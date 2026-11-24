// Internet radio stations (getInternetRadioStations). Click plays the stream; the queue waits.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

RadioData {
    id: root

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        visible: root.stations !== null && root.stations.length > 0
        PageTitle { text: "RADYO" }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.stations || []
            delegate: ListRow {
                required property var modelData
                // SUB/WAVE is asked at the home page, or at the stream's own address when none is set
                readonly property string home: modelData.homePageUrl || modelData.originUrl || ""
                readonly property var peek: Subwave.peeked[home]
                Component.onCompleted: if (home) Subwave.probe(home)
                title: modelData.name
                // SUB/WAVE: what is on air now; any other station: its home page
                detail: (peek && peek.title ? [peek.title, peek.artist].filter(x => x).join(" — ") + (peek.dj ? " · " + peek.dj : "")
                                            : modelData.homePageUrl || "").toUpperCase()
                current: Player.station.id === modelData.id
                enabled: Sync.canControl
                onClicked: Player.playStation(modelData.id)
            }
            ScrollBar.vertical: Scroll {}
        }
    }
    Empty { visible: root.stations !== null && root.stations.length === 0; text: "İSTASYON YOK" }
    Loading { loading: root.stations === null && !root.failed; failed: root.failed; onRetry: root.load() }
}
