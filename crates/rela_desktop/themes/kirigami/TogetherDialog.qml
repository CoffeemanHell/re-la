// Listening together: who is online and whom they follow (those listening to me first), and the relay switches.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts
import Rela

Dialog {
    id: d
    property var w // the window
    function show() { tabs.currentIndex = 0; open() }
    objectName: "together"
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(520, d.w.width - 40); height: Math.min(480, d.w.height - 40)
    padding: 24; topPadding: 16 // the settings window's margins
    contentItem: ColumnLayout {
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
        RowLayout { // the settings window's header
            PageTitle { text: qsTr("Birlikte dinle"); Layout.fillWidth: true }
            ToolButton { icon.name: "window-close"; onClicked: d.close() }
        }
        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: qsTr("Kişiler") }
            TabButton { text: qsTr("Ayarlar") }
        }
        StackLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            currentIndex: tabs.currentIndex
            Flickable {
                id: peopleFlick
                clip: true
                contentHeight: people.implicitHeight
                ScrollBar.vertical: Scroll {}
                ColumnLayout {
                    id: people
                    width: peopleFlick.width - peopleFlick.rightMargin
                    spacing: 10
                    RowLayout {
                        visible: Sync.following !== ""
                        Layout.fillWidth: true
                        Plain { text: qsTr("%1 ile dinliyorsun").arg(Sync.following); font.weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                        Button { text: qsTr("Ayrıl"); onClicked: Sync.unfollow() }
                    }
                    Hint { visible: Sync.following !== ""; text: qsTr("Ayrılınca aynı kuyrukla kendi başına devam edersin.") }
                    Hint { text: qsTr("Birini dinlemeye başlayınca onun çaldığını aynı saniyede duyarsın.") }
                    Hint { visible: Sync.users.length === 0; text: qsTr("Şu an çevrimiçi kimse yok.") }
                    Repeater {
                        // those listening to me first, then the rest in the relay's order
                        model: Sync.users.filter(u => Sync.listeners.indexOf(u.user) >= 0).concat(Sync.users.filter(u => Sync.listeners.indexOf(u.user) < 0))
                        RowLayout {
                            id: userRow
                            required property var modelData
                            readonly property string root: modelData.following || modelData.user
                            readonly property bool mine: Sync.listeners.indexOf(modelData.user) >= 0
                            objectName: "user_" + modelData.user
                            function radioLine(more) { // the station, then its title and artist when known
                                const st = modelData.station
                                return more.length === 2 ? qsTr("📻 %1 · %2 · %3").arg(st).arg(more[0]).arg(more[1])
                                     : more.length === 1 ? qsTr("📻 %1 · %2").arg(st).arg(more[0]) : qsTr("📻 %1").arg(st)
                            }
                            Layout.fillWidth: true
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Plain {
                                    Layout.fillWidth: true; elide: Text.ElideRight
                                    text: userRow.mine ? qsTr("%1  →  seni dinliyor").arg(userRow.modelData.user)
                                        : userRow.modelData.following ? qsTr("%1  →  %2").arg(userRow.modelData.user).arg(userRow.modelData.following) : userRow.modelData.user
                                    font.weight: userRow.mine ? Font.DemiBold : Font.Normal
                                    color: userRow.mine ? Kirigami.Theme.highlightColor : Kirigami.Theme.textColor
                                }
                                Hint {
                                    visible: !!userRow.modelData.station || !!userRow.modelData.title
                                    text: userRow.modelData.station
                                          ? userRow.radioLine([userRow.modelData.title, userRow.modelData.artist].filter(x => x))
                                          : (userRow.modelData.playing ? qsTr("▶  %1  ·  %2") : qsTr("❚❚  %1  ·  %2")).arg(userRow.modelData.title || "").arg(userRow.modelData.artist || "")
                                    elide: Text.ElideRight; wrapMode: Text.NoWrap
                                }
                            }
                            Button {
                                visible: !userRow.mine
                                text: Sync.following === userRow.root ? qsTr("Dinliyorsun") : qsTr("Dinle")
                                enabled: Sync.following !== userRow.root
                                onClicked: Sync.follow(userRow.modelData.user)
                            }
                        }
                    }
                }
            }
            ColumnLayout {
                spacing: 10
                SwitchRow {
                    label: qsTr("Katılanlar kontrol edebilir")
                    hint: qsTr("Seni dinleyenler şarkı değiştirebilir, duraklatabilir, kuyruğu düzenleyebilir.")
                    checked: Sync.allowControl
                    onToggled: on => Sync.allowControl = on
                }
                SwitchRow {
                    label: qsTr("Gizli")
                    hint: qsTr("Çevrimiçi listesinde görünmezsin, kimse seni dinlemeye başlayamaz.")
                    checked: Sync.hidden
                    onToggled: on => Sync.hidden = on
                }
                Item { Layout.fillHeight: true }
            }
        }
    }
}
