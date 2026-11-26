// SUB/WAVE request: ask the DJ for a song and follow the answer (Subwave.request); the upcoming tracks sit below.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Item {
    id: f
    readonly property var req: Subwave.request
    readonly property string st: req && req.state ? req.state : ""
    readonly property var labels: ({ pending: qsTr("Bekliyor"), done: qsTr("Çalınacak"), rejected: qsTr("Reddedildi"), failed: qsTr("Başarısız"), unknown: qsTr("Bilinmiyor") })

    ColumnLayout {
        anchors.fill: parent
        spacing: 10
        TextField {
            id: text
            objectName: "reqText"
            Layout.fillWidth: true
            placeholderText: qsTr("Ne çalsın? (ör. Radiohead'den bir şey)")
            onAccepted: if (send.enabled) send.clicked()
        }
        RowLayout {
            Layout.fillWidth: true
            TextField { id: name; objectName: "reqName"; Layout.fillWidth: true; placeholderText: qsTr("Adın"); text: Player.user }
            Button {
                id: send
                objectName: "reqSend"
                text: qsTr("Gönder")
                enabled: text.text.trim() !== "" && f.st !== "pending"
                onClicked: Subwave.sendRequest(text.text, name.text)
            }
        }
        Plain { visible: f.st !== ""; text: f.labels[f.st] || ""; font.weight: Font.DemiBold }
        Plain {
            visible: !!f.req.title
            Layout.fillWidth: true; elide: Text.ElideRight
            text: [f.req.title, f.req.artist].filter(x => x).join(" — ")
        }
        Plain { visible: f.req.queuePosition > 0; text: qsTr("Sırada: %1").arg(f.req.queuePosition); color: palette.placeholderText }
        Plain { visible: !!f.req.message; Layout.fillWidth: true; wrapMode: Text.Wrap; text: f.req.message || ""; color: palette.placeholderText }
        Plain { text: qsTr("Sıradakiler"); color: palette.placeholderText; visible: upcoming.count > 0 }
        ListView {
            id: upcoming
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true; spacing: 2
            model: Subwave.now.upcoming || []
            delegate: ItemDelegate {
                required property var modelData
                id: item
                width: ListView.view.width - ListView.view.rightMargin
                contentItem: RowLayout {
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; text: item.modelData.title }
                    Plain { text: item.modelData.artist + (item.modelData.requestedBy ? " · " + item.modelData.requestedBy : ""); color: palette.placeholderText }
                }
            }
            ScrollBar.vertical: Scroll {}
        }
    }
}
