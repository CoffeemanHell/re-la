// Star rating 0..5 (setRating) as dots: click a dot to rate, the same dot again to clear.
// Changes at once; a failed write puts the old rating back and says so.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import shared
import Rela

Row {
    id: st
    property var item
    readonly property var w: ApplicationWindow.window
    readonly property Rating rated: Rating { item: st.item; w: st.w; owner: st }
    readonly property int rating: rated.rating
    spacing: 2
    function rate(n) { rated.rate(n) }
    Repeater {
        model: 5
        AbstractButton {
            required property int index
            id: star
            objectName: "star" + (index + 1)
            hoverEnabled: true
            implicitWidth: 20; implicitHeight: 24
            contentItem: Text {
                text: star.index < st.rating ? "●" : "○"
                color: star.index < st.rating ? App.theme.accent : star.hovered ? App.theme.fg : App.theme.muted
                font.pixelSize: 14
                horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
            }
            onClicked: st.rate(index + 1)
        }
    }
}
