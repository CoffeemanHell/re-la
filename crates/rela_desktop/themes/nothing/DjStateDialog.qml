// The DJ's last pick: the sitting it read, then the songs it weighed, best first, with what each score is made of.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

Dialog {
    id: d
    property var w // the window
    readonly property DjData dj: DjData { parent: d.contentItem } // a parent: Player.get answers only such owners
    function show() { dj.refresh(); open() }
    function refresh() { dj.refresh() }
    objectName: "djState"
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(920, d.w.width - 40); height: Math.min(620, d.w.height - 40)
    padding: 24
    background: Rectangle { color: App.theme.bg; border.width: 1; border.color: App.theme.line }
    Overlay.modal: Rectangle { color: "#cc000000" }

    readonly property var cols: [ // key, title, width
        ["total", qsTr("PUAN"), 56], ["fit", qsTr("UYUM"), 56], ["taste", qsTr("ZEVK"), 56], ["similar", qsTr("BENZER"), 64],
        ["fresh", qsTr("YENİ"), 52], ["recent", qsTr("TEKRAR"), 64], ["feedback", qsTr("GERİ B."), 64]
    ]
    function num(v) { return v ? Number(v).toFixed(2) : "–" }

    contentItem: ColumnLayout {
        spacing: 14
        RowLayout {
            spacing: 8
            Dots { text: qsTr("DJ DURUMU"); font.pixelSize: 24; Layout.fillWidth: true }
            Pill { text: qsTr("YENİLE"); onClicked: d.refresh() }
            Pill { text: "×"; onClicked: d.close() }
        }
        Hint {
            visible: d.dj.picked
            text: (d.dj.state.searching ? qsTr("ARAYIŞ MODUNDA: ATLAMALAR EKSİ SAYILMIYOR, DJ DAHA ÇOK KEŞFEDİYOR.") + "\n" : "")
                + qsTr("ÇIKIŞ NOKTASI: %1").arg(((d.dj.state.seed || {}).title || "–").toUpperCase())
                + ((d.dj.state.session || []).length ? "\n" + qsTr("BU OTURUM: %1").arg(d.dj.state.session.map(t => t.tag.toUpperCase() + " " + (t.weight > 0 ? "+" : "") + t.weight.toFixed(1)).join(", ")) : "") // tag names: plain capitals, no Turkish İ
        }
        Mono { visible: !d.dj.picked; text: qsTr("DJ HENÜZ SEÇİM YAPMADI"); color: App.theme.muted; Layout.topMargin: 24; Layout.alignment: Qt.AlignHCenter }
        RowLayout { // column titles, over the rows' cells (rows: 6 px padding, 12 px left for the scroll bar)
            visible: d.dj.picked
            Layout.leftMargin: 6; Layout.rightMargin: 18
            spacing: 8
            Mono { text: qsTr("ŞARKI"); Layout.fillWidth: true; color: App.theme.muted; font.pixelSize: 9 }
            Repeater {
                model: d.cols
                delegate: Mono {
                    required property var modelData
                    text: modelData[1]; Layout.preferredWidth: modelData[2]; horizontalAlignment: Text.AlignRight; color: App.theme.muted; font.pixelSize: 9
                }
            }
        }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true
            visible: d.dj.picked
            clip: true
            model: d.dj.state.rows || []
            ScrollBar.vertical: Scroll {}
            delegate: Item {
                id: row
                required property var modelData
                readonly property bool picked: !!modelData.picked
                objectName: "djRow_" + modelData.id
                width: ListView.view.width - 12; height: 44
                HoverHandler { id: hov }
                ToolTip.visible: hov.hovered; ToolTip.delay: 500
                ToolTip.text: [row.modelData.why || "",
                               row.modelData.disliked ? qsTr("Sevilmiyor") : "",
                               row.modelData.skip ? qsTr("Eksi: %1").arg(Number(row.modelData.skip).toFixed(1)) : "",
                               row.modelData.lastHeardDays !== null && row.modelData.lastHeardDays !== undefined ? qsTr("%n gün önce dinlendi", "", row.modelData.lastHeardDays) : ""]
                              .filter(x => x).join("\n") || qsTr("Seçilmedi")
                Rectangle { anchors.fill: parent; color: row.picked ? App.theme.surface2 : hov.hovered ? App.theme.surface : "transparent" }
                Rectangle { visible: row.picked; width: 3; height: parent.height; color: App.theme.accent }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 6
                    spacing: 8
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 2
                        Mono { text: (row.modelData.title || "").toUpperCase(); Layout.fillWidth: true; font.bold: row.picked }
                        Mono {
                            text: (row.modelData.artist || "").toUpperCase() + (row.picked && row.modelData.why ? "  ·  " + DjKnobs.upper(row.modelData.why) : "")
                            Layout.fillWidth: true; color: App.theme.muted; font.pixelSize: 9
                        }
                    }
                    Repeater {
                        model: d.cols
                        delegate: Mono {
                            required property var modelData
                            text: d.num(row.modelData[modelData[0]]); Layout.preferredWidth: modelData[2]; horizontalAlignment: Text.AlignRight
                            color: modelData[0] === "total" && row.modelData.disliked ? App.theme.accent : App.theme.fg
                        }
                    }
                }
            }
        }
    }
}
