// Now playing: big cover over its own blurred copy; queue and similar songs on the right.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Effects
import shared
import Rela

NowPlayingData {
    id: root

    Image {
        id: ambient
        anchors.fill: parent
        source: root.live ? Subwave.coverUrl : root.cur.coverArt ? "image://cover/" + root.cur.coverArt : ""
        sourceSize: Qt.size(300, 300)
        fillMode: Image.PreserveAspectCrop
        visible: false
    }
    MultiEffect {
        anchors.fill: parent; source: ambient
        visible: ambient.status === Image.Ready
        blurEnabled: true; blur: 1.0; blurMax: 64
        saturation: 0.2
    }
    Rectangle { anchors.fill: parent; color: palette.window; opacity: 0.6 }

    Plain {
        objectName: "strip"
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.leftMargin: 32; anchors.rightMargin: 32; anchors.bottomMargin: 10
        elide: Text.ElideRight
        text: root.strip
        color: palette.placeholderText
        font.features: { "tnum": 1 }
        ToolTip.visible: stripHover.hovered && Player.signalPath !== ""; ToolTip.text: Player.signalPath
        HoverHandler { id: stripHover }
    }

    RowLayout {
        anchors.fill: parent; anchors.margins: 32; anchors.bottomMargin: 40
        spacing: 32

        ColumnLayout {
            id: left
            // the cover is centred in the column; every text starts at the cover's left edge and is as wide as the cover
            readonly property real side: Math.max(120, Math.min(root.width * 0.42 - 16, root.height - 220))
            readonly property real edge: Math.max(0, (root.width * 0.42 - side) / 2) // from the set width, not the laid out one: no layout loop
            Layout.preferredWidth: root.width * 0.42; Layout.maximumWidth: root.width * 0.42
            Layout.fillWidth: false; Layout.fillHeight: true
            spacing: 6
            Item { Layout.fillHeight: true }
            Cover {
                objectName: "cover"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: left.side; Layout.preferredHeight: left.side
                art: Player.station.id ? "" : root.cur.coverArt || ""
                url: root.live ? Subwave.coverUrl : ""
                radius: 8
                layer.enabled: true
                layer.effect: MultiEffect { shadowEnabled: true; shadowBlur: 1.0; shadowOpacity: 0.5; shadowVerticalOffset: 8 }
            }
            Item { Layout.preferredHeight: 8 }
            Plain {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.preferredWidth: left.side; Layout.maximumWidth: left.side
                elide: Text.ElideRight; horizontalAlignment: Text.AlignLeft
                text: root.live && root.sw.title ? root.sw.title : Player.station.id ? Player.station.name : root.cur.title || "Çalan bir şey yok"
                font.pointSize: Qt.application.font.pointSize * 1.6; font.weight: Font.Bold
            }
            Plain { Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; visible: !!Player.station.id; text: "● Canlı"; color: "#e5484d" }
            Plain { Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.maximumWidth: left.side; elide: Text.ElideRight; visible: root.live; text: root.sw.artist || "" }
            Plain {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.maximumWidth: left.side; elide: Text.ElideRight
                visible: root.live; color: palette.placeholderText; text: root.sw.album || ""
            }
            Plain {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.maximumWidth: left.side; elide: Text.ElideRight
                visible: root.live && !!root.sw.dj; color: palette.placeholderText
                text: "DJ " + root.sw.dj + (root.sw.show ? " · " + root.sw.show : "") + " · " + (root.sw.listeners || 0) + " dinleyici"
            }
            Plain { Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; visible: root.live && !Subwave.online; text: "Bağlantı yok"; color: "#e5484d" }
            Link {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.maximumWidth: left.side
                visible: !Player.station.id
                text: root.cur.artist || ""
                active: !!root.cur.artistId
                onActivated: root.w.go("artist", {id: root.cur.artistId})
            }
            Link {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge; Layout.maximumWidth: left.side
                text: root.cur.album || ""
                color: palette.placeholderText
                visible: !Player.station.id
                active: !!root.cur.albumId
                onActivated: root.w.go("album", {id: root.cur.albumId})
            }
            RowLayout {
                Layout.alignment: Qt.AlignLeft; Layout.leftMargin: left.edge
                visible: !Player.station.id && !!root.cur.id
                Heart { item: root.cur }
                Stars { item: root.cur }
            }
            Item { Layout.fillHeight: true }
        }

        Frame {
            Layout.fillWidth: true; Layout.fillHeight: true
            // one surface under all three tabs
            background: Rectangle { color: Qt.rgba(palette.base.r, palette.base.g, palette.base.b, 0.55); radius: 8 }
            ColumnLayout {
                anchors.fill: parent
                spacing: 6
                RowLayout {
                    TabBar {
                        id: liveTabs // SUB/WAVE: no queue and no similar songs, the DJ picks
                        visible: root.live
                        TabButton { text: "Sözler"; width: implicitWidth }
                        TabButton { text: "İstek"; width: implicitWidth }
                    }
                    TabBar {
                        id: tabs
                        visible: !root.live
                        TabButton { text: "Sıra  " + Player.queueLength; width: implicitWidth }
                        TabButton { text: "Benzer"; width: implicitWidth }
                        TabButton { text: "Sözler"; width: implicitWidth }
                    }
                    Item { Layout.fillWidth: true }
                    Button {
                        visible: !root.live && tabs.currentIndex === 0 && Player.queueLength > 0
                        flat: true
                        icon.name: "document-save"
                        text: "Kuyruğu playlist olarak kaydet"
                        display: AbstractButton.IconOnly
                        ToolTip.visible: hovered; ToolTip.text: text; ToolTip.delay: 500
                        onClicked: root.w.askName("Kuyruğu playlist olarak kaydet", "", name =>
                            Player.set(root, "createPlaylist", { name: name, songId: Player.queue.map(s => s.id) },
                                       r => root.w.notify(r ? "Playlist oluşturuldu: " + name : "İşlem başarısız")))
                    }
                    Button {
                        visible: !root.live && tabs.currentIndex === 0 && Player.index < Player.queueLength - 1
                        enabled: Sync.canControl
                        flat: true
                        icon.name: Util.icon("edit-clear-all")
                        text: "Sıradakileri temizle"
                        display: AbstractButton.IconOnly
                        ToolTip.visible: hovered; ToolTip.text: text; ToolTip.delay: 500

                        onClicked: Player.clearUpcoming()
                    }
                }
                StackLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    visible: root.live
                    currentIndex: liveTabs.currentIndex
                    Lyrics { live: true; active: root.live && liveTabs.currentIndex === 0 }
                    RequestForm {}
                }
                StackLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    visible: !root.live
                    currentIndex: tabs.currentIndex
                    Item {
                        ListView {
                            id: queueList
                            anchors.fill: parent
                            clip: true; spacing: 2
                            model: Player.queue
                            delegate: SongRow { required property var modelData; song: modelData; inQueue: true }
                            ScrollBar.vertical: Scroll {}
                            currentIndex: Player.index
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            Component.onCompleted: positionViewAtIndex(currentIndex, ListView.Center)
                        }
                        Plain {
                            anchors.centerIn: parent; visible: Player.queueLength === 0
                            text: "Kuyruk boş.\nAlbümlerden ya da aramadan şarkı ekle; Radyo açıksa gerisi kendiliğinden gelir."
                            color: palette.placeholderText; horizontalAlignment: Text.AlignHCenter
                        }
                    }
                    // songs like the current one (getSimilarSongs), fetched when the tab is shown or the song changes
                    SimilarSongs {
                        id: similar
                        songId: root.cur.id || ""
                        shown: tabs.currentIndex === 1
                        ListView {
                            anchors.fill: parent
                            clip: true; spacing: 2
                            model: similar.songs
                            delegate: SongRow { required property var modelData; song: modelData }
                            ScrollBar.vertical: Scroll {}
                        }
                        Plain {
                            anchors.centerIn: parent
                            visible: !similar.loading && !similar.failed && similar.songs.length === 0
                            text: "Benzer şarkı bulunamadı."
                            color: palette.placeholderText
                        }
                        Loading { loading: similar.loading; failed: similar.failed; onRetry: { similar.forId = ""; similar.load() } }
                    }
                    Lyrics { song: root.cur; active: !root.live && tabs.currentIndex === 2 }
                }
            }
        }
    }
}
