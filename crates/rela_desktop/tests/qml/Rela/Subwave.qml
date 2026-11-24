// Stands in for Subwave (the SUB/WAVE client) in QML tests: plain properties, calls are recorded.
pragma Singleton
import QtQuick

QtObject {
    property bool active: false
    property var now: ({})
    property string coverUrl: ""
    property double elapsedMs: 0
    property bool online: true
    property var peeked: ({})
    property var lyrics: ({})
    property string lyricsState: ""
    property bool likeEnabled: false
    property bool liked: false
    property int likeBlockedMs: 0
    property var request: ({})
    property var calls: []

    function reset() { active = false; now = {}; coverUrl = ""; elapsedMs = 0; online = true; peeked = {}; lyrics = {}; lyricsState = ""; likeEnabled = false; liked = false; likeBlockedMs = 0; request = {}; calls = [] }
    function rec(c) { calls = calls.concat([c]) }
    function called(name) { return calls.filter(c => c[0] === name) }
    function probe(url) { rec(["probe", url]) }
    function like() { rec(["like"]) }
    function sendRequest(text, name) { rec(["sendRequest", text, name]) }
}
