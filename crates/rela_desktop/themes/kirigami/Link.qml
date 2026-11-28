// Text that opens a page when clicked (artist, album). Plain text when there is nowhere to go.
import QtQuick
import QtQuick.Controls
import org.kde.kirigami as Kirigami

Plain {
    id: link
    signal activated()
    property bool active: true
    elide: Text.ElideRight
    font.underline: active && hov.hovered
    Kirigami.Theme.inherit: true // the color stays the caller's; only the underline marks a link, as in KDE apps
    HoverHandler { id: hov; enabled: link.active; cursorShape: Qt.PointingHandCursor }
    TapHandler { enabled: link.active; onTapped: link.activated() }
}
