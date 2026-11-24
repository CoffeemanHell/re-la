// Page state overlay: blinking "YÜKLENİYOR" while loading, "YÜKLENEMEDİ" with a retry pill after a failure.
import QtQuick
import QtQuick.Layouts
import Rela

Item {
    id: l
    property bool loading: false
    property bool failed: false
    signal retry()
    anchors.fill: parent
    visible: loading || failed
    Mono {
        anchors.centerIn: parent
        visible: l.loading
        text: "YÜKLENİYOR"
        color: App.theme.muted
        SequentialAnimation on opacity {
            running: l.loading; loops: Animation.Infinite
            NumberAnimation { to: 0.2; duration: 500 }
            NumberAnimation { to: 1; duration: 500 }
        }
    }
    ColumnLayout {
        anchors.centerIn: parent
        visible: l.failed && !l.loading
        spacing: 14
        Mono { Layout.alignment: Qt.AlignHCenter; text: "YÜKLENEMEDİ"; color: App.theme.accent }
        Pill { Layout.alignment: Qt.AlignHCenter; text: "TEKRAR DENE"; onClicked: l.retry() }
    }
}
