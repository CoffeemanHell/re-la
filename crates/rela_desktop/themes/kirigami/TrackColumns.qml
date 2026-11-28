// The column row shared by TrackRow and TrackHeader: same spacing, same columns, so cells line up.
// Which optional columns show: Player.options.columns (right click on the header); some drop as the row narrows.
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Layouts
import shared
import Rela

RowLayout {
    property bool hideAlbum: false
    // a string, so the cells are rebuilt only when the set changes, not on every pixel of a resize;
    // none before the row has a width (a new row starts at 0: the narrow set would be built and thrown away)
    readonly property string keys: width > 0 ? Columns.shown(Player.options.columns, width, hideAlbum) : ""
    readonly property var columns: keys ? keys.split(",").map(Columns.byKey) : []
    anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
    spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
}
