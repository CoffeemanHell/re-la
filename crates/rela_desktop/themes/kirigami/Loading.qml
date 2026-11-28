// Page state overlay: Kirigami's loading placeholder while loading, "Yüklenemedi" with a retry button after a failure.
import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: l
    property bool loading: false
    property bool failed: false
    signal retry()
    anchors.fill: parent
    visible: loading || failed
    Kirigami.LoadingPlaceholder { anchors.centerIn: parent; visible: l.loading; text: "" }
    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: l.failed && !l.loading
        icon.name: "network-disconnect"
        text: qsTr("Yüklenemedi")
        helpfulAction: Kirigami.Action { text: qsTr("Tekrar dene"); icon.name: "view-refresh"; onTriggered: l.retry() }
    }
}
