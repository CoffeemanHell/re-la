// Mono caption that opens a page when clicked (artist, album). Plain text when there is nowhere to go.
import QtQuick

Mono {
    id: link
    signal activated()
    property bool active: true
    font.underline: active && hov.hovered
    HoverHandler { id: hov; enabled: link.active; cursorShape: Qt.PointingHandCursor }
    TapHandler { enabled: link.active; onTapped: link.activated() }
}
