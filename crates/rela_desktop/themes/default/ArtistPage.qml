// One artist: picture, bio and similar artists (getArtistInfo2), play the discography / artist radio, popular songs, albums.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

ArtistData {
    id: root
    scroller: flick

    ItemMenu { id: artistMenu; kind: "artist"; item: root.artist || ({}); w: root.w }

    Flickable {
        id: flick
        onContentHeightChanged: if (root.pendingY >= 0 && contentHeight - height >= root.pendingY) { contentY = root.pendingY; root.pendingY = -1 }
        onMovementStarted: root.pendingY = -1
        anchors.fill: parent
        visible: root.artist !== null
        contentHeight: col.implicitHeight + 48
        clip: true
        ScrollBar.vertical: ScrollBar {}
        ColumnLayout {
            id: col
            x: 24; y: 24; width: parent.width - 48
            spacing: 12

            RowLayout {
                spacing: 24
                Cover {
                    Layout.preferredWidth: 200; Layout.preferredHeight: 200; Layout.alignment: Qt.AlignTop
                    art: root.artist ? root.artist.coverArt || "" : ""; radius: 8
                }
                ColumnLayout {
                    Layout.fillWidth: true; Layout.alignment: Qt.AlignTop
                    spacing: 6
                    Plain {
                        Layout.fillWidth: true; elide: Text.ElideRight
                        text: root.artist ? root.artist.name : ""
                        font.pointSize: Qt.application.font.pointSize * 2.2; font.weight: Font.Bold
                    }
                    Label {
                        id: bioText
                        property bool open: false
                        Layout.fillWidth: true
                        visible: root.bio !== ""
                        text: root.bio
                        textFormat: Text.StyledText // decodes &amp; and friends
                        wrapMode: Text.WordWrap
                        maximumLineCount: open ? 1000 : 4
                        elide: Text.ElideRight
                        color: palette.placeholderText
                        TapHandler { onTapped: bioText.open = !bioText.open }
                        HoverHandler { cursorShape: bioText.truncated || bioText.open ? Qt.PointingHandCursor : Qt.ArrowCursor }
                    }
                    Flow {
                        Layout.fillWidth: true
                        visible: root.similar.length > 0
                        spacing: 14
                        Plain { text: qsTr("Benzer sanatçılar:"); color: palette.placeholderText }
                        Repeater {
                            model: root.similar.slice(0, 8)
                            Link { required property var modelData; text: modelData.name; onActivated: root.w.go("artist", { id: modelData.id }) }
                        }
                    }
                    RowLayout {
                        Layout.topMargin: 8
                        spacing: 8
                        Heart { item: root.artist; kind: "artist" }
                        Button { text: qsTr("Diskografiyi çal"); icon.name: "media-playback-start"; highlighted: true; enabled: Sync.canControl && root.artist !== null; onClicked: artistMenu.act.playAll(false) }
                        Button { text: qsTr("Sanatçı radyosu"); icon.name: "media-playlist-shuffle"; enabled: Sync.canControl; onClicked: root.radio() }
                        Button { text: "⋯"; implicitWidth: implicitHeight; enabled: root.artist !== null; onClicked: artistMenu.popup(); ToolTip.visible: hovered; ToolTip.text: qsTr("Diğer"); ToolTip.delay: 500 }
                    }
                }
            }

            component Heading: Plain {
                Layout.topMargin: 16
                font.pointSize: Qt.application.font.pointSize * 1.35; font.weight: Font.Bold
            }
            Heading { text: qsTr("Popüler"); visible: root.topSongs.length > 0 }
            Repeater {
                model: root.topSongs.slice(0, 5)
                SongRow { required property var modelData; song: modelData; list: root.topSongs; Layout.fillWidth: true }
            }

            Heading { text: qsTr("Albümler"); visible: !!(root.artist && root.artist.album) }
            Flow {
                Layout.fillWidth: true
                spacing: 24
                Repeater {
                    model: root.artist ? root.artist.album || [] : []
                    AlbumCard { required property var modelData; album: modelData }
                }
            }

            Heading { text: qsTr("Benzer sanatçılar"); visible: root.similar.length > 0 }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: root.similar
                    Button { required property var modelData; text: modelData.name; onClicked: root.w.go("artist", { id: modelData.id }) }
                }
            }
        }
    }
    Loading { loading: root.loading; failed: root.failed; onRetry: root.load() }
}
