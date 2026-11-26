// One album (getAlbum): header with play / shuffle / add, then the songs (disc headers on multi-disc albums).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

AlbumData {
    id: root
    scroller: songList

    ListView {
        id: songList
        anchors.fill: parent
        visible: root.album !== null
        clip: true
        leftMargin: 36; rightMargin: 36; bottomMargin: 36
        model: root.songs
        ScrollBar.vertical: Scroll {}
        header: ColumnLayout {
            width: ListView.view.width - 72
            spacing: 0
            RowLayout {
                Layout.fillWidth: true
                spacing: 28
                Cover {
                    Layout.preferredWidth: 220; Layout.preferredHeight: 220; Layout.topMargin: 36; Layout.bottomMargin: 24
                    art: root.album ? root.album.coverArt || "" : ""; mono: false
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Mono { text: qsTr("ALBÜM"); color: App.theme.muted }
                    Dots {
                        Layout.fillWidth: true
                        text: root.album ? root.w.up(root.album.name) : ""
                        font.pixelSize: 40; lineHeight: 0.9
                        wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                    }
                    Link {
                        Layout.maximumWidth: parent.width
                        text: root.album ? root.w.up(root.album.artist) : ""; font.pixelSize: 14
                        active: !!(root.album && root.album.artistId)
                        onActivated: root.w.go("artist", { id: root.album.artistId })
                    }
                    Flow { // year · genres (each opens its genre) · songs · length
                        Layout.fillWidth: true
                        readonly property var genres: !root.album ? [] : root.album.genres && root.album.genres.length ? root.album.genres.map(g => g.name)
                                                                     : root.album.genre ? [root.album.genre] : []
                        Mono { color: App.theme.muted; text: root.album && root.album.year ? root.album.year + "  ·  " : "" }
                        Repeater {
                            model: parent.genres
                            Link {
                                required property string modelData
                                text: root.w.up(modelData); color: App.theme.muted; rightPadding: 10 // genres apart without a separator, so each is one clean link
                                onActivated: root.w.go("genre", { name: modelData })
                            }
                        }
                        Mono {
                            color: App.theme.muted
                            text: !root.album ? "" : (parent.genres.length ? " ·  " : "") + qsTr("%n ŞARKI", "", root.songs.length) + "  ·  " + root.w.up(root.length(root.album.duration || 0))
                        }
                    }
                    RowLayout {
                        Layout.topMargin: 10
                        spacing: 8
                        Pill { text: qsTr("ÇAL"); on: true; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.songs, 0) }
                        Pill { text: qsTr("KARIŞTIR"); enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.shuffled(root.songs), 0) }
                        Pill { text: qsTr("SIRAYA EKLE"); enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.add(root.songs, 2) }
                        Pill {
                            id: addAll
                            text: qsTr("TÜMÜNÜ LİSTEYE"); enabled: root.songs.length > 0
                            onClicked: albumMenu.popup(addAll, 0, addAll.height)
                            PlaylistMenu { id: albumMenu; songs: root.songs; w: root.w }
                        }
                        Heart { item: root.album; kind: "album" }
                        Pill { text: "⋯"; onClicked: root.w.menu("album", root.album) }
                    }
                }
            }
            TrackHeader { Layout.fillWidth: true; Layout.topMargin: 12; hideAlbum: true }
        }
        delegate: Column {
            id: d
            required property var modelData
            required property int index
            readonly property bool discStart: root.multiDisc && (index === 0 || (root.songs[index - 1].discNumber || 1) !== (modelData.discNumber || 1))
            width: ListView.view.width - 72
            Mono {
                visible: d.discStart
                topPadding: 18; bottomPadding: 6
                text: qsTr("DİSK %1").arg(String(d.modelData.discNumber || 1).padStart(2, "0"))
                color: App.theme.accent
            }
            TrackRow { song: d.modelData; index: d.index; list: root.songs; number: d.modelData.track || d.index + 1; hideAlbum: true }

        }
    }
    Loading { loading: root.loading; failed: root.failed; onRetry: root.load() }
}
