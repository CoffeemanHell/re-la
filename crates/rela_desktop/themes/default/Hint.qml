// Small grey explanation under a setting; hidden while empty.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Plain {
    color: palette.placeholderText
    wrapMode: Text.WordWrap
    Layout.fillWidth: true
    font.pointSize: Qt.application.font.pointSize * 0.9
    visible: text !== ""
}
