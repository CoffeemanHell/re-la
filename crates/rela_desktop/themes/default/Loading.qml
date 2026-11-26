// Page state overlay: a spinner while loading, "Yüklenemedi" with a retry button after a failure.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: l
    property bool loading: false
    property bool failed: false
    signal retry()
    anchors.fill: parent
    visible: loading || failed
    BusyIndicator { anchors.centerIn: parent; running: l.loading; visible: l.loading }
    ColumnLayout {
        anchors.centerIn: parent
        visible: l.failed && !l.loading
        Plain { Layout.alignment: Qt.AlignHCenter; text: qsTr("Yüklenemedi"); color: palette.placeholderText }
        Button { Layout.alignment: Qt.AlignHCenter; text: qsTr("Tekrar dene"); icon.name: "view-refresh"; onClicked: l.retry() }
    }
}
