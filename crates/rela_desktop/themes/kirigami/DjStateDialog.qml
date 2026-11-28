// The DJ's last pick: the sitting it read, then the songs it weighed, best first, with what each score is made of.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
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
    width: Math.min(900, d.w.width - 40); height: Math.min(620, d.w.height - 40)
    padding: 24; topPadding: 16

    readonly property var cols: [ // key, title, width
        ["total", qsTr("Puan"), 56], ["fit", qsTr("Uyum"), 52], ["taste", qsTr("Zevk"), 52], ["similar", qsTr("Benzer"), 56],
        ["fresh", qsTr("Yeni"), 48], ["recent", qsTr("Tekrar"), 56], ["feedback", qsTr("Geri b."), 56]
    ]
    function num(v) { return v ? Number(v).toFixed(2) : "–" }

    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            PageTitle { text: qsTr("DJ durumu"); Layout.fillWidth: true }
            ToolButton { icon.name: "view-refresh"; ToolTip.visible: hovered; ToolTip.text: qsTr("Yenile"); onClicked: d.refresh() }
            ToolButton { icon.name: "window-close"; onClicked: d.close() }
        }
        Hint {
            Layout.fillWidth: true
            visible: d.dj.picked
            text: [ // one whole sentence per line
                d.dj.state.searching ? qsTr("Arayış modunda: atlamalar eksi sayılmıyor, DJ daha çok keşfediyor.") : "",
                qsTr("Çıkış noktası: %1").arg((d.dj.state.seed || {}).title || "–"),
                (d.dj.state.session || []).length ? qsTr("Bu oturum: %1").arg(d.dj.state.session.map(t => t.tag + " " + (t.weight > 0 ? "+" : "") + t.weight.toFixed(1)).join(", ")) : ""
            ].filter(l => l).join("\n")
        }
        Plain { visible: !d.dj.picked; text: qsTr("DJ henüz seçim yapmadı"); color: Kirigami.Theme.disabledTextColor; Layout.topMargin: 24; Layout.alignment: Qt.AlignHCenter }
        RowLayout { // column titles, over the rows' cells (rows: 6 px padding, 12 px left for the scroll bar)
            visible: d.dj.picked
            Layout.leftMargin: 6; Layout.rightMargin: 18
            spacing: 8
            Plain { text: qsTr("Şarkı"); Layout.fillWidth: true; color: Kirigami.Theme.disabledTextColor }
            Repeater {
                model: d.cols
                delegate: Plain {
                    required property var modelData
                    text: modelData[1]; Layout.preferredWidth: modelData[2]; horizontalAlignment: Text.AlignRight; color: Kirigami.Theme.disabledTextColor
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
                width: ListView.view.width - 12; height: 40
                HoverHandler { id: hov }
                ToolTip.visible: hov.hovered; ToolTip.delay: Kirigami.Units.toolTipDelay
                ToolTip.text: [row.modelData.why || "",
                               row.modelData.disliked ? qsTr("Sevilmiyor") : "",
                               row.modelData.skip ? qsTr("Eksi: %1").arg(Number(row.modelData.skip).toFixed(1)) : "",
                               row.modelData.lastHeardDays !== null && row.modelData.lastHeardDays !== undefined ? qsTr("%n gün önce dinlendi", "", row.modelData.lastHeardDays) : ""]
                              .filter(x => x).join("\n") || qsTr("Seçilmedi")
                Rectangle {
                    anchors.fill: parent; radius: Kirigami.Units.cornerRadius
                    color: row.picked ? Qt.rgba(Kirigami.Theme.highlightColor.r, Kirigami.Theme.highlightColor.g, Kirigami.Theme.highlightColor.b, 0.18) : hov.hovered ? Kirigami.Theme.alternateBackgroundColor : "transparent"
                }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 6
                    spacing: 8
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 0
                        Plain { text: row.modelData.title; elide: Text.ElideRight; Layout.fillWidth: true; font.weight: row.picked ? Font.DemiBold : Font.Normal }
                        Plain {
                            text: row.modelData.artist + (row.picked && row.modelData.why ? "  ·  " + row.modelData.why : "")
                            elide: Text.ElideRight; Layout.fillWidth: true; color: Kirigami.Theme.disabledTextColor
                            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 0.85
                        }
                    }
                    Repeater {
                        model: d.cols
                        delegate: Plain {
                            required property var modelData
                            text: d.num(row.modelData[modelData[0]]); Layout.preferredWidth: modelData[2]; horizontalAlignment: Text.AlignRight
                            font.features: { "tnum": 1 }
                            color: modelData[0] === "total" && row.modelData.disliked ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
                        }
                    }
                }
            }
        }
    }
}
