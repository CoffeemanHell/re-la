// Column titles above TrackRows (same TrackColumns, so they line up). Right click: which columns to show.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

Item {
    id: h
    property bool hideAlbum: false
    property alias columnMenu: columnMenu
    implicitHeight: 28
    component Head: Mono { color: App.theme.muted; font.pixelSize: 9 }
    TrackColumns {
        id: cols
        anchors.fill: parent
        hideAlbum: h.hideAlbum
        Head { text: "#"; Layout.preferredWidth: 28 }
        Head { text: qsTr("PARÇA ADI"); Layout.fillWidth: true; Layout.preferredWidth: 300 }
        Repeater {
            model: cols.columns
            Head {
                required property var modelData
                text: modelData.title.toLocaleUpperCase(Qt.locale(Qt.uiLanguage))
                Layout.preferredWidth: modelData.w; Layout.fillWidth: !!modelData.fill
                horizontalAlignment: modelData.align === "right" ? Text.AlignRight : modelData.align === "center" ? Text.AlignHCenter : Text.AlignLeft
            }
        }
    }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: columnMenu.popup() }
    Menu {
        id: columnMenu
        readonly property var chosen: String(Player.options.columns === undefined ? Columns.defaults : Player.options.columns).split(",")
        Instantiator {
            model: Columns.all
            onObjectAdded: (i, o) => columnMenu.insertItem(i, o)
            onObjectRemoved: (i, o) => columnMenu.removeItem(o)
            MenuItem {
                required property var modelData
                text: modelData.title.toLocaleUpperCase(Qt.locale(Qt.uiLanguage))
                checkable: true
                checked: columnMenu.chosen.indexOf(modelData.key) >= 0
                onTriggered: Player.setOption("columns", Columns.all.map(c => c.key).filter(k => k === modelData.key ? checked : columnMenu.chosen.indexOf(k) >= 0).join(","))
            }
        }
    }
}
