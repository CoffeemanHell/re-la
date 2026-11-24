// Square album art, monochrome until hovered (mono: false keeps the colors).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Effects
import Rela

Item {
    id: cv
    property string art: ""
    property string url: "" // a plain image URL (SUB/WAVE cover); wins over art
    property int px // ignored: kept so older themes that set it still load
    // Decode size: the layout size rounded up to a power of two. Qt multiplies sourceSize by the screen's scale,
    // and equal buckets let the same cover share one download and one cache entry across the app.
    readonly property int decodePx: width > 0 ? Math.max(64, Math.pow(2, Math.ceil(Math.log2(Math.max(width, height))))) : 0
    property bool mono: true
    Rectangle { anchors.fill: parent; color: App.theme.surface; border.color: App.theme.line }
    Image {
        id: img
        anchors.fill: parent; anchors.margins: 1
        source: !cv.decodePx ? "" : cv.url ? cv.url : cv.art ? "image://cover/" + cv.art : ""
        sourceSize: Qt.size(cv.decodePx, cv.decodePx)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    MultiEffect {
        anchors.fill: img; source: img
        visible: img.status === Image.Ready
        saturation: cv.mono && !hov.hovered ? -1 : 0
        Behavior on saturation { NumberAnimation { duration: 250 } }
    }
    HoverHandler { id: hov }
}
