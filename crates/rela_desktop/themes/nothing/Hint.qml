// Small grey explanation under a setting; hidden while empty.
import QtQuick
import QtQuick.Layouts
import Rela

Mono {
    color: App.theme.muted; font.pixelSize: 9
    wrapMode: Text.WordWrap; elide: Text.ElideNone
    Layout.fillWidth: true
    visible: text !== ""
}
