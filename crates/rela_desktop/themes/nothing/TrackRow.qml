// One song as a table row (album, playlist, favorites, all tracks). Optional columns: Columns.qml and the header menu;
// some drop as the row narrows (bit rate below 1100 px, year 950, album 800 or hideAlbum, rating 700).
// Double click plays `list` from this row; right click opens the song menu.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

Item {
    id: row
    required property var song
    required property int index
    property var list: null
    property int number: index + 1
    property bool hideAlbum: false
    property bool removable: false // playlist page: the menu offers "PLAYLIST'TEN ÇIKAR"
    signal removeRequested()
    readonly property var w: ApplicationWindow.window
    readonly property bool current: !!song && song.id !== undefined && song.id === Player.current.id
    function up(s) { return (s || "").toUpperCase() }
    width: ListView.view ? ListView.view.width - ListView.view.leftMargin - ListView.view.rightMargin : parent ? parent.width : 0
    height: 40

    function openMenu() { w.menu("song", song, { onRemove: removable ? () => row.removeRequested() : null }) }
    HoverHandler { id: hov }
    TapHandler {
        acceptedButtons: Qt.LeftButton
        enabled: Sync.canControl && !!row.list
        onDoubleTapped: Player.play(row.list, row.index)
    }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: row.openMenu() }
    Rectangle { anchors.fill: parent; color: hov.hovered ? App.theme.surface : "transparent" }
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
    TrackColumns {
        anchors.fill: parent
        hideAlbum: row.hideAlbum
        Mono {
            objectName: "col_num"
            Layout.preferredWidth: 28
            text: row.current ? "●" : String(row.number).padStart(2, "0")
            color: row.current ? App.theme.accent : App.theme.muted
        }
        Mono { objectName: "col_title"; Layout.fillWidth: true; Layout.preferredWidth: 300; text: row.up(row.song.title); font.bold: row.current }
        Repeater {
            model: parent.columns
            Loader {
                required property var modelData
                readonly property var c: modelData
                objectName: "col_" + c.key
                Layout.preferredWidth: c.w; Layout.fillWidth: !!c.fill; Layout.fillHeight: true
                sourceComponent: c.kind === "artists" ? artistsCell : c.kind === "heart" ? heartCell : c.kind === "stars" ? starsCell : textCell
                onLoaded: if (item.c !== undefined) item.c = c
            }
        }
    }

    // cells of the optional columns; text cells get their column (c) from the Loader
    Component {
        id: textCell
        Mono {
            property var c: ({}) // set by the Loader
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: c.align === "right" ? Text.AlignRight : c.align === "center" ? Text.AlignHCenter : Text.AlignLeft
            text: c.kind === "duration" ? (row.w ? row.w.fmt(row.song.duration) : "") : row.up(Columns.cell(c, row.song))
            color: App.theme.muted
        }
    }
    Component {
        id: artistsCell
        Item {
            clip: true
            Row {
                anchors.verticalCenter: parent.verticalCenter
                Repeater {
                    // every artist of the song opens its page; without an artists list, the one artist
                    model: row.song.artists && row.song.artists.length ? row.song.artists : row.song.artist ? [{ id: row.song.artistId, name: row.song.artist }] : []
                    Link {
                        required property var modelData
                        required property int index
                        text: row.up(modelData.name) + (index < row.song.artists?.length - 1 ? "," : "")
                        rightPadding: 6
                        color: App.theme.muted
                        active: !!modelData.id
                        onActivated: row.w.go("artist", { id: modelData.id })
                    }
                }
            }
        }
    }
    Component { id: heartCell; Item { Heart { anchors.centerIn: parent; implicitHeight: 22; item: row.song } } }
    Component { id: starsCell; Item { Stars { anchors.centerIn: parent; item: row.song } } }
}
