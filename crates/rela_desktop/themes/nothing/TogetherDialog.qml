// Listening together: who is online and whom they follow (those listening to me first), and the relay switches.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Dialog {
    id: d
    property var w // the window
    property int tab: 0
    function show() { tab = 0; open() }
    objectName: "together"
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(560, d.w.width - 40); height: Math.min(480, d.w.height - 40)
    padding: 24
    background: Rectangle { color: App.theme.bg; border.width: 1; border.color: App.theme.line }
    Overlay.modal: Rectangle { color: "#cc000000" }
    contentItem: ColumnLayout {
        spacing: 16
        RowLayout {
            spacing: 8
            Pill { text: "KİŞİLER"; on: d.tab === 0; onClicked: d.tab = 0 }
            Pill { text: "AYARLAR"; on: d.tab === 1; onClicked: d.tab = 1 }
            Item { Layout.fillWidth: true }
            Pill { text: "×"; onClicked: d.close() }
        }
        Flickable {
            visible: d.tab === 0
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            contentHeight: people.implicitHeight
            ScrollBar.vertical: Scroll {}
            ColumnLayout {
                id: people
                width: parent.width
                spacing: 12
                RowLayout {
                    visible: Sync.following !== ""
                    Layout.fillWidth: true
                    Mono { text: d.w.up(Sync.following) + " İLE DİNLİYORSUN"; Layout.fillWidth: true }
                    Pill { text: "AYRIL"; onClicked: Sync.unfollow() }
                }
                Hint { visible: Sync.following !== ""; text: "AYRILINCA AYNI KUYRUKLA KENDİ BAŞINA DEVAM EDERSİN." }
                Hint { text: "BİRİNİ DİNLEMEYE BAŞLAYINCA ONUN ÇALDIĞINI AYNI SANİYEDE DUYARSIN." }
                Mono { visible: Sync.users.length === 0; text: "ŞU AN ÇEVRİMİÇİ KİMSE YOK"; color: App.theme.muted }
                Repeater {
                    // those listening to me first, then the rest in the relay's order
                    model: Sync.users.filter(u => Sync.listeners.indexOf(u.user) >= 0).concat(Sync.users.filter(u => Sync.listeners.indexOf(u.user) < 0))
                    RowLayout {
                        id: userRow
                        required property var modelData
                        readonly property string root: modelData.following || modelData.user
                        readonly property bool mine: Sync.listeners.indexOf(modelData.user) >= 0
                        objectName: "user_" + modelData.user
                        Layout.fillWidth: true
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 2
                            Mono {
                                Layout.fillWidth: true
                                text: d.w.up(userRow.modelData.user) + (userRow.mine ? "  →  SENİ DİNLİYOR" : userRow.modelData.following ? "  →  " + d.w.up(userRow.modelData.following) : "")
                                color: userRow.mine ? App.theme.accent : App.theme.fg
                            }
                            Mono {
                                visible: !!userRow.modelData.station || !!userRow.modelData.title
                                // a station: its name, then what it plays; a song: playing or paused
                                text: d.w.up(userRow.modelData.station
                                      ? ["RADYO " + userRow.modelData.station, userRow.modelData.title, userRow.modelData.artist].filter(x => x).join(" · ")
                                      : (userRow.modelData.playing ? "▶  " : "❚❚  ") + (userRow.modelData.title || "") + "  ·  " + (userRow.modelData.artist || ""))
                                color: App.theme.muted; font.pixelSize: 9; Layout.fillWidth: true
                            }
                        }
                        Pill {
                            visible: !userRow.mine
                            text: Sync.following === userRow.root ? "DİNLİYORSUN" : "DİNLE"
                            on: Sync.following === userRow.root
                            enabled: Sync.following !== userRow.root
                            onClicked: Sync.follow(userRow.modelData.user)
                        }
                    }
                }
            }
        }
        ColumnLayout {
            visible: d.tab === 1
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 16
            SwitchRow {
                label: "KATILANLAR KONTROL EDEBİLİR"
                hint: "SENİ DİNLEYENLER ŞARKI DEĞİŞTİREBİLİR, DURAKLATABİLİR, KUYRUĞU DÜZENLEYEBİLİR."
                checked: Sync.allowControl
                onToggled: on => Sync.allowControl = on
            }
            SwitchRow {
                label: "GİZLİ"
                hint: "ÇEVRİMİÇİ LİSTESİNDE GÖRÜNMEZSİN, KİMSE SENİ DİNLEMEYE BAŞLAYAMAZ."
                checked: Sync.hidden
                onToggled: on => Sync.hidden = on
            }
            Item { Layout.fillHeight: true }
        }
    }
}
