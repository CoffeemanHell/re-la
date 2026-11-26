// Every song (search3 with an empty query), 100 at a time while scrolling. Double click plays from that row.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared

TracksData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24
        spacing: 12
        PageTitle { text: qsTr("Tüm parçalar") }
        TrackHeader { Layout.fillWidth: true }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.rightMargin: -24; rightMargin: 24 // the list runs to the window edge so the scroll bar sits there; rows keep the 24 px gutter
            clip: true; spacing: 2
            model: root.rows
            delegate: TrackRow { required property int i; song: root.loaded[i]; list: root.loaded }

            ScrollBar.vertical: Scroll {}
            onContentYChanged: if (contentY + height > contentHeight - 600) root.load()
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load()
        }
    }
    Empty { visible: !root.loading && !root.failed && root.rows.count === 0; text: qsTr("Şarkı yok") }
    Loading { loading: root.loading && root.rows.count === 0; failed: root.failed && root.rows.count === 0; onRetry: root.load() }
}
