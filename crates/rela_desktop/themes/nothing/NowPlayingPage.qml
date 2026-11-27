// Now playing: big square cover and the title in dot-matrix type; queue and similar songs on the right.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

NowPlayingData {
    id: root
    readonly property DjData djData: DjData {}
    Connections { target: Player; function onDjChanged() { if (Player.dj) djData.refreshFeedback() } }
    Component.onCompleted: if (Player.dj) djData.refreshFeedback()

    Mono {
        objectName: "strip"
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.leftMargin: 36; anchors.rightMargin: 36; anchors.bottomMargin: 12
        text: root.w.up(root.strip)
        color: App.theme.muted
        ToolTip.visible: stripHover.hovered && Player.signalPath !== ""; ToolTip.text: Player.signalPath
        HoverHandler { id: stripHover }
    }

    RowLayout {
        anchors.fill: parent; anchors.margins: 36; anchors.bottomMargin: 44
        spacing: 48

        ColumnLayout {
            // 0.42 as in the default theme: at the smallest window the tab row still fits beside it
            Layout.preferredWidth: root.width * 0.42; Layout.maximumWidth: root.width * 0.42
            Layout.fillWidth: false; Layout.fillHeight: true
            spacing: 12
            Item { Layout.fillHeight: true }
            Cover {
                readonly property real side: Math.max(120, Math.min(root.width * 0.42 - 16, root.height - 300))
                Layout.preferredWidth: side; Layout.preferredHeight: side
                art: Player.station.id ? "" : root.cur.coverArt || ""; mono: false
                url: root.live ? Subwave.coverUrl : ""
            }
            Dots {
                Layout.fillWidth: true
                text: root.live && root.sw.title ? root.w.up(root.sw.title) : Player.station.id ? root.w.up(Player.station.name) : root.w.up(root.cur.title) || qsTr("SESSİZLİK")
                font.pixelSize: 46
                lineHeight: 0.9
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
            Link {
                Layout.maximumWidth: parent.width
                visible: !Player.station.id
                text: root.w.up(root.cur.artist); font.pixelSize: 14
                active: !!root.cur.artistId
                onActivated: root.w.go("artist", { id: root.cur.artistId })
            }
            Link {
                Layout.maximumWidth: parent.width
                visible: !Player.station.id
                text: root.w.up(root.cur.album); color: App.theme.muted
                active: !!root.cur.albumId
                onActivated: root.w.go("album", { id: root.cur.albumId })
            }
            Mono { visible: !!Player.station.id; text: qsTr("● CANLI"); color: App.theme.accent }
            Mono { Layout.maximumWidth: parent.width; visible: root.live; text: root.w.up(root.sw.artist); font.pixelSize: 14 }
            Mono { Layout.maximumWidth: parent.width; visible: root.live; text: root.w.up(root.sw.album); color: App.theme.muted }
            Mono {
                Layout.maximumWidth: parent.width
                visible: root.live && !!root.sw.dj; color: App.theme.muted
                text: root.w.up(qsTr("DJ %1").arg(root.sw.dj) + (root.sw.show ? " · " + root.sw.show : "")) + " · " + qsTr("%n DİNLEYİCİ", "", root.sw.listeners || 0)
            }
            Mono { visible: root.live && !Subwave.online; text: qsTr("BAĞLANTI YOK"); color: App.theme.accent }
            Mono { // why the DJ picked it
                Layout.maximumWidth: parent.width; elide: Text.ElideRight
                visible: !Player.station.id && !!root.cur.djWhy
                text: DjKnobs.upper(root.cur.djWhy); color: App.theme.muted; font.pixelSize: 10
            }
            RowLayout {
                visible: !Player.station.id && !!root.cur.id
                spacing: 12
                Heart { item: root.cur }
                Stars { item: root.cur }
                component Fb: Pill {
                    required property int dir
                    visible: Player.dj
                    on: djData.dirOf(root.cur.id) === dir
                    implicitHeight: 24
                    onClicked: djData.toggle(root.cur, dir)
                }
                Fb { objectName: "djMore"; dir: 1; text: qsTr("▲ DAHA ÇOK") }
                Fb { objectName: "djLess"; dir: -1; text: qsTr("▼ DAHA AZ") }
            }
            Item { Layout.fillHeight: true }
        }

        Item {
            Layout.fillWidth: true; Layout.fillHeight: true
            // one surface under all three tabs
            Rectangle { anchors.fill: parent; color: App.theme.surface; opacity: 0.7; border.width: 1; border.color: App.theme.line }
            ColumnLayout {
                anchors.fill: parent; anchors.margins: 16
                spacing: 14
                RowLayout {
                    id: tabs
                    property int current: 0
                    property int liveCurrent: 0 // SUB/WAVE: no queue and no similar songs, the DJ picks
                    spacing: 8
                    Pill { visible: root.live; text: qsTr("SÖZLER"); on: tabs.liveCurrent === 0; onClicked: tabs.liveCurrent = 0 }
                    Pill { visible: root.live; text: qsTr("İSTEK"); on: tabs.liveCurrent === 1; onClicked: tabs.liveCurrent = 1 }
                    Pill { visible: !root.live; text: qsTr("SIRA %1").arg(String(Player.queueLength).padStart(2, "0")); on: tabs.current === 0; onClicked: tabs.current = 0 }
                    Pill { visible: !root.live; text: qsTr("BENZER"); on: tabs.current === 1; onClicked: tabs.current = 1 }
                    Pill { visible: !root.live; text: qsTr("SÖZLER"); on: tabs.current === 2; onClicked: tabs.current = 2 }
                    Item { Layout.fillWidth: true }
                    Pill {
                        text: "⤓"; visible: !root.live && tabs.current === 0 && Player.queueLength > 0
                        ToolTip.visible: hovered; ToolTip.text: qsTr("Kuyruğu playlist olarak kaydet"); ToolTip.delay: 500
                        onClicked: root.w.askName(qsTr("Kuyruğu playlist olarak kaydet"), "", name =>
                            Player.set(root, "createPlaylist", { name: name, songId: Player.queue.map(s => s.id) },
                                       r => root.w.notify(r ? qsTr("Playlist oluşturuldu: %1").arg(name) : qsTr("İşlem başarısız"))))
                    }
                    Pill {
                        text: "⌫"; visible: !root.live && tabs.current === 0 && Player.index < Player.queueLength - 1; enabled: Sync.canControl; onClicked: Player.clearUpcoming()
                        ToolTip.visible: hovered; ToolTip.text: qsTr("Sıradakileri temizle"); ToolTip.delay: 500
                    }
                }
                StackLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    visible: root.live
                    currentIndex: tabs.liveCurrent
                    Lyrics { live: true; active: root.live && tabs.liveCurrent === 0 }
                    RequestForm {}
                }
                StackLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    visible: !root.live
                    currentIndex: tabs.current
                    Item {
                        ListView {
                            id: queueList
                            anchors.fill: parent
                            clip: true
                            model: Player.queue
                            delegate: SongRow { required property var modelData; song: modelData; inQueue: true }
                            ScrollBar.vertical: Scroll {}
                            currentIndex: Player.index
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            Component.onCompleted: positionViewAtIndex(currentIndex, ListView.Center)
                        }
                        Mono {
                            anchors.centerIn: parent; visible: Player.queueLength === 0
                            horizontalAlignment: Text.AlignHCenter; color: App.theme.muted
                            text: qsTr("KUYRUK BOŞ\nALBÜMLERDEN YA DA ARAMADAN ŞARKI EKLE")
                        }
                    }
                    // songs like the current one (getSimilarSongs), fetched when the tab is shown or the song changes
                    SimilarSongs {
                        id: similar
                        songId: root.cur.id || ""
                        shown: tabs.current === 1
                        ListView {
                            anchors.fill: parent
                            clip: true
                            model: similar.songs
                            delegate: SongRow { required property var modelData; song: modelData }
                            ScrollBar.vertical: Scroll {}
                        }
                        Mono {
                            anchors.centerIn: parent
                            visible: !similar.loading && !similar.failed && similar.songs.length === 0
                            text: qsTr("BENZER ŞARKI BULUNAMADI"); color: App.theme.muted
                        }
                        Loading { loading: similar.loading; failed: similar.failed; onRetry: { similar.forId = ""; similar.load() } }
                    }
                    Lyrics { song: root.cur; active: !root.live && tabs.current === 2 }
                }
            }
        }
    }
}
