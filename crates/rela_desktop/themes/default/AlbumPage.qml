// One album (getAlbum): header with play / shuffle / add, then the songs (disc headers on multi-disc albums).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
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
        leftMargin: 24; rightMargin: 24; bottomMargin: 24
        model: root.songs
        ScrollBar.vertical: ScrollBar {}
        header: ColumnLayout {
            width: ListView.view.width - 48
            spacing: 0
            RowLayout {
                Layout.fillWidth: true
                spacing: 24
                Cover {
                    Layout.preferredWidth: 220; Layout.preferredHeight: 220; Layout.topMargin: 24; Layout.bottomMargin: 16
                    art: root.album ? root.album.coverArt || "" : ""; radius: 8
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4
                    Plain { text: "ALBÜM"; color: palette.placeholderText; font.letterSpacing: 1.1; font.pointSize: Qt.application.font.pointSize * 0.85 }
                    Plain {
                        Layout.fillWidth: true; elide: Text.ElideRight
                        text: root.album ? root.album.name : ""
                        font.pointSize: Qt.application.font.pointSize * 2; font.weight: Font.Bold
                    }
                    Link {
                        Layout.maximumWidth: parent.width
                        text: root.album ? root.album.artist || "" : ""
                        font.pointSize: Qt.application.font.pointSize * 1.15
                        active: !!(root.album && root.album.artistId)
                        onActivated: root.w.go("artist", { id: root.album.artistId })
                    }
                    Flow { // year · genres (each opens its genre) · songs · length
                        Layout.fillWidth: true
                        readonly property var genres: !root.album ? [] : root.album.genres && root.album.genres.length ? root.album.genres.map(g => g.name)
                                                                     : root.album.genre ? [root.album.genre] : []
                        Plain { color: palette.placeholderText; text: root.album && root.album.year ? root.album.year + "  ·  " : "" }
                        Repeater {
                            model: parent.genres
                            Link {
                                required property string modelData
                                text: modelData; color: palette.placeholderText; rightPadding: 10 // genres apart without a separator, so each is one clean link
                                onActivated: root.w.go("genre", { name: modelData })
                            }
                        }
                        Plain {
                            color: palette.placeholderText
                            text: !root.album ? "" : (parent.genres.length ? " ·  " : "") + root.songs.length + " şarkı  ·  " + root.length(root.album.duration || 0)
                        }
                    }
                    RowLayout {
                        Layout.topMargin: 12
                        spacing: 8
                        Button { text: "Çal"; icon.name: "media-playback-start"; highlighted: true; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.songs, 0) }
                        Button { text: "Karıştır"; icon.name: "media-playlist-shuffle"; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.play(root.shuffled(root.songs), 0) }
                        Button { text: "Sıraya ekle"; icon.name: "list-add"; enabled: Sync.canControl && root.songs.length > 0; onClicked: Player.add(root.songs, 2) }
                        Button {
                            id: addAll
                            text: "Tümünü playlist'e ekle"; icon.name: Util.icon("view-media-playlist"); enabled: root.songs.length > 0
                            onClicked: albumMenu.popup(addAll, 0, addAll.height)
                            PlaylistMenu { id: albumMenu; songs: root.songs; w: root.w }
                        }
                        Heart { item: root.album; kind: "album" }
                        Button {
                            text: "⋯"; implicitWidth: implicitHeight; ToolTip.visible: hovered; ToolTip.text: "Diğer"; ToolTip.delay: 500
                            onClicked: root.w.menu("album", root.album)
                        }
                    }
                }
            }
            TrackHeader { Layout.fillWidth: true; hideAlbum: true }
        }
        delegate: Column {
            id: d
            required property var modelData
            required property int index
            readonly property bool discStart: root.multiDisc && (index === 0 || (root.songs[index - 1].discNumber || 1) !== (modelData.discNumber || 1))
            width: ListView.view.width - 48
            Plain {
                visible: d.discStart
                topPadding: 16; bottomPadding: 4
                text: "Disk " + (d.modelData.discNumber || 1)
                color: palette.placeholderText; font.weight: Font.DemiBold
            }
            TrackRow { song: d.modelData; index: d.index; list: root.songs; number: d.modelData.track || d.index + 1; hideAlbum: true }

        }
    }
    Loading { loading: root.loading; failed: root.failed; onRetry: root.load() }
}
