// Internet radio stations (getInternetRadioStations). Click plays the stream; the queue waits.
// The data and actions only: each theme draws it (its page inherits this).
import QtQuick
import Rela

Item {
    id: root
    property var args
    property var stations: null
    property bool failed: false

    function load() {
        stations = null
        failed = false
        Player.get(root, "getInternetRadioStations", {}, r => {
            if (!r) return failed = true
            stations = r.internetRadioStations.internetRadioStation || []
        })
    }
    Component.onCompleted: load()
}
