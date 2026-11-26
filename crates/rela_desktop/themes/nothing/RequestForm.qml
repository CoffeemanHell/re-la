// SUB/WAVE request: ask the DJ for a song and follow the answer (Subwave.request); the upcoming tracks sit below.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Item {
    id: f
    readonly property var req: Subwave.request
    readonly property string st: req && req.state ? req.state : ""
    readonly property var labels: ({ pending: qsTr("BEKLİYOR"), done: qsTr("ÇALINACAK"), rejected: qsTr("REDDEDİLDİ"), failed: qsTr("BAŞARISIZ"), unknown: qsTr("BİLİNMİYOR") })
    function up(s) { return (s || "").toUpperCase() }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12
        Field {
            id: text
            objectName: "reqText"
            Layout.fillWidth: true
            placeholderText: qsTr("NE ÇALSIN?")
            onAccepted: if (send.enabled) send.clicked()
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Field { id: name; objectName: "reqName"; Layout.fillWidth: true; placeholderText: qsTr("ADIN"); text: Player.user }
            Pill {
                id: send
                objectName: "reqSend"
                text: qsTr("GÖNDER"); on: enabled
                enabled: text.text.trim() !== "" && f.st !== "pending"
                onClicked: Subwave.sendRequest(text.text, name.text)
            }
        }
        Mono { visible: f.st !== ""; text: f.labels[f.st] || ""; color: App.theme.accent }
        Mono { visible: !!f.req.title; Layout.fillWidth: true; text: f.up([f.req.title, f.req.artist].filter(x => x).join(" — ")) }
        Mono { visible: f.req.queuePosition > 0; text: qsTr("SIRADA: %1").arg(f.req.queuePosition); color: App.theme.muted }
        Mono { visible: !!f.req.message; Layout.fillWidth: true; wrapMode: Text.Wrap; elide: Text.ElideNone; text: f.up(f.req.message); color: App.theme.muted }
        Mono { visible: upcoming.count > 0; Layout.topMargin: 8; text: qsTr("SIRADAKİLER"); color: App.theme.muted }
        ListView {
            id: upcoming
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: Subwave.now.upcoming || []
            delegate: Item {
                required property var modelData
                id: item
                width: ListView.view.width; height: 36
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
                RowLayout {
                    anchors.fill: parent
                    spacing: 12
                    Mono { Layout.fillWidth: true; text: f.up(item.modelData.title) }
                    Mono {
                        Layout.maximumWidth: parent.width / 2
                        text: f.up(item.modelData.artist + (item.modelData.requestedBy ? " · " + item.modelData.requestedBy : "")); color: App.theme.muted; font.pixelSize: 10
                    }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
}
