// Stands in for Player (import Rela) in QML tests: records every call; get/set answer from `replies` (keyed by method) a tick later.
pragma Singleton
import QtQuick

QtObject {
    property var calls: []
    property var replies: ({})
    property bool hold: false     // keep replies back until release()
    property var held: []
    property var queue: []
    readonly property int queueLength: queue.length
    property int index: -1
    property var current: ({})
    property var station: ({})
    property var options: ({ autoContinue: true, scrobble: true, nowPlaying: true })
    property bool playing: false
    property bool online: true
    property bool signedIn: true
    property bool loginFailed: false
    property bool dj: false
    property bool connecting: false
    property bool exclusive: false
    property int crossfade: 0
    property string user: "me"
    property string status: ""
    property string theme: "default"
    property string signalPath: ""
    property string streamInfo: ""   // what the playing stream is: "FLAC · 44,1 kHz · stereo · 890 kbps"
    property double volume: 100
    property double position: 0
    property double duration: 0
    property bool shuffle: false
    property int repeat: 0
    property int bitrate: 0
    property var waveform: []
    property var waveformRms: []

    function reset() { calls = []; replies = {}; queue = []; index = -1; current = {}; station = {}; playing = false; hold = false; held = []; shuffle = false; repeat = 0; position = 0; duration = 0; bitrate = 0; streamInfo = ""; dj = false; connecting = false; loginFailed = false; online = true; signedIn = true; options = { autoContinue: true, scrobble: true, nowPlaying: true } }
    function rec(c) { calls = calls.concat([c]) }
    function called(name, method) { return calls.filter(c => c[0] === name && (method === undefined || c[1] === method)) }
    function answer(method, done) {
        const r = replies[method]
        const f = () => done(r === undefined ? null : r)
        if (hold) held = held.concat([f])
        else Qt.callLater(f)
    }
    function release() { const h = held; held = []; hold = false; h.forEach(f => f()) }
    function get(owner, method, params, done) { rec(["get", method, params]); answer(method, done) }
    function set(owner, method, params, done) { rec(["set", method, params]); answer(method, done) }
    function play(songs, start) { rec(["play", songs, start]) }
    function add(songs, where) { rec(["add", songs, where]) }
    function playStation(id) { rec(["playStation", id]) }
    function jump(i) { rec(["jump", i]) }
    function remove(i) { rec(["remove", i]) }
    function move(a, b) { rec(["move", a, b]) }
    function clearUpcoming() { rec(["clearUpcoming"]) }
    function toggle() { rec(["toggle"]) }
    function next() { rec(["next"]) }
    function prev() { rec(["prev"]) }
    function seek(s) { rec(["seek", s]) }
    function copy(t) { rec(["copy", t]) }
    function setting(k) { return "" }
    function setOption(k, v) { rec(["setOption", k, v]); const o = Object.assign({}, options); o[k] = v; options = o }
    function shuffledJson(songs) { rec(["shuffledJson"]); return JSON.stringify(JSON.parse(songs).reverse()) } // reversed: a test can tell
    function downloadNameJson(item, kind) { rec(["downloadNameJson", JSON.parse(item), kind]); return "file" }
    function download(id, f) { rec(["download", id, f]) }
    function login(u, n, p) { rec(["login", u, n, p]) }
    function logout() { rec(["logout"]) }


}
