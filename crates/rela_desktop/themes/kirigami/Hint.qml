// Small grey explanation under a setting; hidden while empty.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import QtQuick.Layouts

Plain {
    color: Kirigami.Theme.disabledTextColor
    wrapMode: Text.WordWrap
    Layout.fillWidth: true
    font.pointSize: Kirigami.Theme.defaultFont.pointSize * 0.9
    visible: text !== ""
}
