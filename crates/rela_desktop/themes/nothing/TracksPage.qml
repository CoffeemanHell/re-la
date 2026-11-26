// Every song (search3 with an empty query), 100 at a time while scrolling. Double click plays from that row.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared

TracksData {
    id: root
    scroller: list

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 36
        spacing: 18
        PageTitle { text: qsTr("TÜM PARÇALAR") }
        TrackHeader { Layout.fillWidth: true }
        ListView {
            id: list
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            model: root.rows
            delegate: TrackRow { required property int i; song: root.loaded[i]; list: root.loaded }

            ScrollBar.vertical: Scroll {}
            onContentYChanged: if (contentY + height > contentHeight - 600) root.load()
            onContentHeightChanged: if (contentHeight > 0 && contentHeight < height) root.load()
        }
    }
    Empty { visible: !root.loading && !root.failed && root.rows.count === 0; text: qsTr("ŞARKI YOK") }
    Loading { loading: root.loading && root.rows.count === 0; failed: root.failed && root.rows.count === 0; onRetry: root.load() }
}
