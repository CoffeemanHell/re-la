pragma Singleton
import QtQuick

QtObject {
    property bool canControl: true
    property bool available: false
    property bool allowControl: true
    property bool hidden: false
    property string following: ""
    property var users: []
    property var listeners: []
    signal listenerJoined(string name)
    signal listenerLeft(string name)
    function follow(u) {}
    function unfollow() {}
}
