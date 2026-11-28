// Star rating 0..5 (setRating): click a star to rate, the same star again to clear.
// Changes at once; a failed write puts the old rating back and says so.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Controls
import shared

Row {
    id: st
    property var item
    readonly property var w: ApplicationWindow.window
    readonly property Rating rated: Rating { item: st.item; w: st.w; owner: st }
    readonly property int rating: rated.rating
    function rate(n) { rated.rate(n) }
    Repeater {
        model: 5
        ToolButton {
            required property int index
            id: star
            objectName: "star" + (index + 1)
            text: index < st.rating ? "★" : "☆"
            implicitWidth: 26; implicitHeight: 26
            contentItem: Plain {
                text: star.text
                color: star.index < st.rating ? st.Kirigami.Theme.highlightColor : st.Kirigami.Theme.disabledTextColor
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.2
                horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
            }
            ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay
            ToolTip.text: index + 1 === st.rating ? qsTr("Puanı kaldır") : qsTr("%n yıldız", "", index + 1)
            onClicked: st.rate(index + 1)
        }
    }
}
