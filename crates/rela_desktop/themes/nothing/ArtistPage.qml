// One artist: picture and bio (getArtistInfo2), play top songs / artist radio, popular songs, albums, similar artists.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
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
        contentHeight: col.implicitHeight + 72
        clip: true
        ScrollBar.vertical: Scroll {}
        ColumnLayout {
            id: col
            x: 36; y: 36; width: parent.width - 72
            spacing: 14

            RowLayout {
                spacing: 28
                Cover {
                    Layout.preferredWidth: 200; Layout.preferredHeight: 200; Layout.alignment: Qt.AlignTop
                    art: root.artist ? root.artist.coverArt || "" : ""
                }
                ColumnLayout {
                    Layout.fillWidth: true; Layout.alignment: Qt.AlignTop
                    spacing: 8
                    Dots {
                        Layout.fillWidth: true
                        text: root.artist ? root.w.up(root.artist.name) : ""
                        font.pixelSize: 44; lineHeight: 0.9
                        wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                    }
                    Mono {
                        id: bioText
                        property bool open: false
                        Layout.fillWidth: true
                        visible: root.bio !== ""
                        text: root.bio
                        textFormat: Text.StyledText // decodes &amp; and friends
                        wrapMode: Text.WordWrap
                        maximumLineCount: open ? 1000 : 4
                        color: App.theme.muted
                        lineHeight: 1.3
                        TapHandler { onTapped: bioText.open = !bioText.open }
                        HoverHandler { cursorShape: bioText.truncated || bioText.open ? Qt.PointingHandCursor : Qt.ArrowCursor }
                    }
                    Flow {
                        Layout.fillWidth: true
                        visible: root.similar.length > 0
                        spacing: 14
                        Mono { text: "BENZER SANATÇILAR:"; color: App.theme.muted }
                        Repeater {
                            model: root.similar.slice(0, 8)
                            Link { required property var modelData; text: root.w.up(modelData.name); onActivated: root.w.go("artist", { id: modelData.id }) }
                        }
                    }
                    RowLayout {
                        Layout.topMargin: 10
                        spacing: 8
                        Heart { item: root.artist; kind: "artist" }
                        Pill { text: "DİSKOGRAFİYİ ÇAL"; on: true; enabled: Sync.canControl && root.artist !== null; onClicked: artistMenu.act.playAll(false) }
                        Pill { text: "SANATÇI RADYOSU"; enabled: Sync.canControl; onClicked: root.radio() }
                        Pill { text: "⋯"; enabled: root.artist !== null; onClicked: artistMenu.popup() }
                    }
                }
            }

            component Heading: Mono { Layout.topMargin: 20; color: App.theme.muted }
            Heading { text: "01 · POPÜLER"; visible: root.topSongs.length > 0 }
            Repeater {
                model: root.topSongs.slice(0, 5)
                SongRow { required property var modelData; song: modelData; list: root.topSongs; Layout.fillWidth: true }
            }

            Heading { text: "02 · ALBÜMLER"; visible: !!(root.artist && root.artist.album) }
            Flow {
                Layout.fillWidth: true
                spacing: 24
                Repeater {
                    model: root.artist ? root.artist.album || [] : []
                    AlbumCard { required property var modelData; album: modelData }
                }
            }

            Heading { text: "03 · BENZER SANATÇILAR"; visible: root.similar.length > 0 }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: root.similar
                    Pill { required property var modelData; text: root.w.up(modelData.name); onClicked: root.w.go("artist", { id: modelData.id }) }
                }
            }
        }
    }
    Loading { loading: root.loading; failed: root.failed; onRetry: root.load() }
}
