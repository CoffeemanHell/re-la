// A clickable list line: optional cover, a title, a muted detail on the right.
import QtQuick
import QtQuick.Layouts
import Rela

Item {
    id: row
    property string title
    property string detail
    property string art: ""
    property bool showArt: false
    property bool current: false
    signal clicked()
    signal menuRequested() // right click
    width: ListView.view ? ListView.view.width : parent ? parent.width : 0
    height: showArt ? 52 : 40
    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: row.clicked() }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: row.menuRequested() }

    Rectangle { anchors.fill: parent; color: hov.hovered ? App.theme.surface : "transparent" }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
    RowLayout {
        anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 6
        spacing: 12
        Mono { visible: row.current; text: "●"; color: App.theme.accent }
        Cover { visible: row.showArt; Layout.preferredWidth: 38; Layout.preferredHeight: 38; art: row.art }
        Mono { Layout.fillWidth: true; text: (row.title || "").toUpperCase(); font.bold: row.current }
        Mono { Layout.maximumWidth: row.width / 2; text: row.detail; color: App.theme.muted; font.pixelSize: 10 }
    }
}
