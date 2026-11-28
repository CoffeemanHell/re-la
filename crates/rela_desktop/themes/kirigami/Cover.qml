// Album art with rounded corners; a note glyph while loading or when there is none.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
import QtQuick.Effects

Item {
    id: cv
    property string art: ""
    property string url: "" // a plain image URL (SUB/WAVE cover); wins over art
    property int px // ignored: kept so older themes that set it still load
    // Decode size: the layout size rounded up to a power of two. Qt multiplies sourceSize by the screen's scale,
    // and equal buckets let the same cover share one download and one cache entry across the app.
    readonly property int decodePx: width > 0 ? Math.max(64, Math.pow(2, Math.ceil(Math.log2(Math.max(width, height))))) : 0
    property real radius: Kirigami.Units.cornerRadius
    Rectangle {
        anchors.fill: parent; radius: cv.radius; color: Kirigami.Theme.alternateBackgroundColor
        Text { anchors.centerIn: parent; text: "♪"; color: Kirigami.Theme.disabledTextColor; font.pixelSize: cv.height * 0.4; visible: img.status !== Image.Ready }
    }
    Image {
        id: img
        anchors.fill: parent
        source: !cv.decodePx ? "" : cv.url ? cv.url : cv.art ? "image://cover/" + cv.art : ""
        sourceSize: Qt.size(cv.decodePx, cv.decodePx)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    Rectangle { id: mask; anchors.fill: parent; radius: cv.radius; visible: false; layer.enabled: true }
    MultiEffect {
        anchors.fill: parent; source: img; visible: img.status === Image.Ready
        maskEnabled: true; maskSource: mask; maskThresholdMin: 0.5; maskSpreadAtMin: 1.0
    }
}
