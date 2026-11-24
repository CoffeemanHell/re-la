// Monospace caption: the theme's body text.
import QtQuick
import Rela

Text {
    textFormat: Text.PlainText // names come from outside: a tag in one is not HTML
    color: App.theme.fg
    font.family: App.theme.font
    font.pixelSize: 11
    font.letterSpacing: 1.2
    elide: Text.ElideRight
}
