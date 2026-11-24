// One song as a table row (album, playlist, favorites, all tracks). Optional columns: Columns.qml and the header menu;
// some drop as the row narrows (bit rate below 1100 px, year 950, album 800 or hideAlbum, rating 700).
// Double click plays `list` from this row; right click opens the song menu.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
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
    property bool removable: false // playlist page: the menu offers "Playlist'ten çıkar"
    signal removeRequested()
    readonly property var w: ApplicationWindow.window
    readonly property bool current: !!song && song.id !== undefined && song.id === Player.current.id
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
    Rectangle {
        anchors.fill: parent; radius: 4
        color: row.current ? Qt.rgba(palette.highlight.r, palette.highlight.g, palette.highlight.b, 0.22)
             : hov.hovered ? palette.alternateBase : "transparent"
    }
    TrackColumns {
        anchors.fill: parent
        hideAlbum: row.hideAlbum
        Item {
            objectName: "col_num"
            Layout.preferredWidth: 28; Layout.fillHeight: true
            Plain { anchors.centerIn: parent; visible: !row.current; text: row.number; color: palette.placeholderText }
            Bars { anchors.centerIn: parent; visible: row.current; active: Player.playing }
        }
        Plain {
            objectName: "col_title"
            Layout.fillWidth: true; Layout.preferredWidth: 300
            elide: Text.ElideRight; text: row.song.title || ""
            font.weight: row.current ? Font.Bold : Font.Normal // not the highlight color: the row is already tinted with it
        }
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
        Plain {
            property var c: ({}) // set by the Loader
            elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter
            horizontalAlignment: c.align === "right" ? Text.AlignRight : c.align === "center" ? Text.AlignHCenter : Text.AlignLeft
            text: c.kind === "duration" ? (row.w ? row.w.fmt(row.song.duration) : "") : Columns.cell(c, row.song)
            color: c.key === "bitrate" ? palette.placeholderText : palette.windowText
            font.features: { "tnum": 1 }
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
                        text: modelData.name + (index < row.song.artists?.length - 1 ? "," : "")
                        rightPadding: 6
                        active: !!modelData.id
                        onActivated: row.w.go("artist", { id: modelData.id })
                    }
                }
            }
        }
    }
    Component { id: heartCell; Item { Heart { anchors.centerIn: parent; item: row.song } } }
    Component { id: starsCell; Item { Stars { anchors.centerIn: parent; item: row.song } } }
}
