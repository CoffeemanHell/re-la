// The player bar at the bottom: what plays (cover, title, artist, album), the transport and the seek row (a waveform
// over an invisible slider), DJ, the SUB/WAVE heart, volume and the status line. Kirigami spacing and colors.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import shared
import Rela
import "../default" as D

ToolBar {
    id: playerBar
    required property var w
    visible: w.usable
    position: ToolBar.Footer
    implicitHeight: Kirigami.Units.gridUnit * 4 + Kirigami.Units.smallSpacing
    readonly property color dim: Kirigami.Theme.disabledTextColor
    readonly property real small: Kirigami.Theme.smallFont.pointSize

    Rectangle { // a skip the DJ will hold against the song, for a few seconds: take it back
        objectName: "skipUndoPopup"
        parent: playerBar.w.contentItem // above the bar, on the page: KDE's ToolBar takes no clicks outside itself
        visible: !!Player.skipUndo.id
        x: playerBar.leftPadding + playerRow.x + middleColumn.x + transport.x + transport.width / 2 - width / 2 // centred on the transport row
        y: parent.height - height - Kirigami.Units.smallSpacing * 2
        z: 2
        width: undoRow.implicitWidth + Kirigami.Units.largeSpacing * 2; height: undoRow.implicitHeight + Kirigami.Units.largeSpacing
        radius: Kirigami.Units.cornerRadius
        color: Kirigami.Theme.backgroundColor
        border.width: 1; border.color: Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.2)
        RowLayout {
            id: undoRow
            anchors.centerIn: parent
            spacing: Kirigami.Units.smallSpacing
            D.Plain {
                text: qsTr("Atlama eksi sayılacak"); color: playerBar.dim
                font.pointSize: playerBar.small
                ToolTip.visible: undoHover.hovered; ToolTip.text: Player.skipUndo.title || ""
                HoverHandler { id: undoHover }
            }
            Button { text: qsTr("Geri al"); flat: true; onClicked: Player.undoSkip() }
        }
    }

    RowLayout {
        id: playerRow
        anchors.fill: parent
        anchors.leftMargin: Kirigami.Units.largeSpacing; anchors.rightMargin: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing * 2

        RowLayout {
            Layout.preferredWidth: 350; Layout.maximumWidth: 350; Layout.minimumWidth: 240
            spacing: Kirigami.Units.largeSpacing
            D.Cover {
                Layout.preferredWidth: Kirigami.Units.gridUnit * 3 + Kirigami.Units.smallSpacing; Layout.preferredHeight: Layout.preferredWidth
                art: Player.station.id ? "" : Player.current.coverArt || ""
                url: playerBar.w.live ? Subwave.coverUrl : ""
                TapHandler { onTapped: playerBar.w.go("nowplaying") }
                HoverHandler { cursorShape: Qt.PointingHandCursor }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                RowLayout {
                    spacing: 0
                    D.Plain {
                        Layout.fillWidth: true; Layout.maximumWidth: Math.ceil(implicitWidth) // layouts round down; a fractional width would elide the last letter
                        elide: Text.ElideRight; font.weight: Font.DemiBold
                        text: playerBar.w.live && Subwave.now.title ? Subwave.now.title : Player.station.id ? Player.station.name : Player.current.title || qsTr("Çalan bir şey yok")
                    }
                    ToolButton {
                        objectName: "songMenu"
                        visible: !Player.station.id && playerBar.w.hasSong
                        icon.name: "overflow-menu"; display: AbstractButton.IconOnly; text: qsTr("Menü")
                        implicitWidth: Kirigami.Units.iconSizes.medium; implicitHeight: implicitWidth
                        onClicked: playerBar.w.menu("song", Player.current, { queueIndex: Player.index })
                    }
                    Item { Layout.fillWidth: true }
                }
                D.Plain { visible: !!Player.station.id; text: qsTr("● Canlı"); color: Kirigami.Theme.negativeTextColor }
                D.Plain { Layout.fillWidth: true; elide: Text.ElideRight; visible: playerBar.w.live; text: Subwave.now.artist || "" }
                D.Link {
                    Layout.fillWidth: true
                    visible: !Player.station.id
                    text: Player.current.artist || ""
                    active: !!Player.current.artistId
                    onActivated: playerBar.w.go("artist", { id: Player.current.artistId })
                }
                D.Link {
                    Layout.fillWidth: true
                    text: Player.current.album || ""
                    color: playerBar.dim
                    visible: !Player.station.id
                    active: !!Player.current.albumId
                    onActivated: playerBar.w.go("album", { id: Player.current.albumId })
                }
            }
        }

        ColumnLayout {
            id: middleColumn
            Layout.fillWidth: true
            Layout.preferredWidth: 1 // takes what the sides leave, whatever the seek row holds: the volume slider must not move
            Layout.minimumWidth: transport.implicitWidth // never narrower than the buttons: the left column gives way instead
            spacing: 0
            RowLayout {
                id: transport
                objectName: "transport"
                Layout.alignment: Qt.AlignHCenter
                component Tip: ToolButton {
                    property string tip
                    display: AbstractButton.IconOnly; text: tip
                    ToolTip.visible: hovered && tip !== ""; ToolTip.delay: Kirigami.Units.toolTipDelay; ToolTip.text: tip
                }
                Tip {
                    objectName: "shuffle"
                    icon.name: "media-playlist-shuffle"; tip: qsTr("Karıştır")
                    checkable: true; checked: Player.shuffle
                    enabled: Sync.canControl && !Player.station.id
                    onClicked: Player.shuffle = !Player.shuffle
                }
                Tip { icon.name: "media-skip-backward"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.prev() }
                Tip {
                    icon.name: Player.playing ? "media-playback-pause" : "media-playback-start"
                    icon.width: Kirigami.Units.iconSizes.medium; icon.height: Kirigami.Units.iconSizes.medium
                    enabled: Sync.canControl
                    onClicked: Player.toggle()
                }
                Tip { icon.name: "media-skip-forward"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.next() }
                Tip {
                    objectName: "repeat"
                    icon.name: Util.icon(Player.repeat === 2 ? "media-playlist-repeat-song" : "media-playlist-repeat")
                    tip: Player.repeat === 2 ? qsTr("Tekrar (tek şarkı)") : qsTr("Tekrar")
                    checkable: true; checked: Player.repeat > 0
                    enabled: Sync.canControl && !Player.station.id
                    onClicked: Player.repeat = (Player.repeat + 1) % 3
                }
            }
            RowLayout { // SUB/WAVE: how far into the track the station is (read only, there is no seek in a stream)
                readonly property real total: Subwave.now.duration || 0
                readonly property real done: total > 0 ? Math.min(Subwave.elapsedMs / 1000, total) : Subwave.elapsedMs / 1000
                visible: playerBar.w.live
                Layout.preferredHeight: Kirigami.Units.gridUnit + Kirigami.Units.largeSpacing
                D.Plain { text: playerBar.w.fmt(parent.done); color: playerBar.dim; font.features: { "tnum": 1 } }
                ProgressBar { Layout.fillWidth: true; visible: parent.total > 0; from: 0; to: Math.max(1, parent.total); value: parent.done }
                Item { Layout.fillWidth: true; visible: parent.total <= 0 }
                D.Plain { visible: parent.total > 0; text: playerBar.w.fmt(parent.total); color: playerBar.dim; font.features: { "tnum": 1 } }
            }
            Item { visible: !!Player.station.id && !playerBar.w.live; Layout.fillWidth: true; Layout.preferredHeight: Kirigami.Units.gridUnit + Kirigami.Units.largeSpacing } // a stream without a timeline keeps the row's height
            RowLayout {
                visible: !Player.station.id // a live stream has no timeline
                D.Plain { text: playerBar.w.fmt(seek.pressed ? seek.value : Player.position); color: playerBar.dim; font.features: { "tnum": 1 } }
                Item {
                    Layout.fillWidth: true; implicitHeight: Math.max(Kirigami.Units.gridUnit + Kirigami.Units.largeSpacing, seek.implicitHeight)
                    // the waveform shows through an invisible slider, which still takes clicks and drags
                    D.Wave { anchors.fill: parent; anchors.margins: Kirigami.Units.smallSpacing; visible: Player.waveform.length > 0; levels: Player.waveform; rms: Player.waveformRms; value: seek.value / seek.to }
                    Slider {
                        id: seek
                        anchors.fill: parent
                        opacity: Player.waveform.length > 0 ? 0 : 1
                        enabled: Sync.canControl && playerBar.w.hasSong
                        from: 0; to: Math.max(1, Player.duration)
                        onPressedChanged: if (!pressed) Player.seek(value)
                    }
                }
                Binding { target: seek; property: "value"; value: Player.position; when: !seek.pressed; restoreMode: Binding.RestoreNone }
                D.Plain { text: playerBar.w.fmt(Player.duration); color: playerBar.dim; font.features: { "tnum": 1 } }
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 350; Layout.maximumWidth: 350; Layout.minimumWidth: 240 // as wide as the left side: the transport stays centered
            spacing: 0
            RowLayout {
                Layout.alignment: Qt.AlignRight
                ToolButton {
                    text: "DJ"; checkable: true; checked: Player.dj
                    onToggled: Player.dj = checked
                    ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay
                    ToolTip.text: qsTr("Açıkken dil ve tür ağırlıklarıyla seçer; kapalıyken Navidrome'un benzer şarkıları")
                }
                ToolButton { // SUB/WAVE: heart the track on air
                    objectName: "swHeart"
                    visible: playerBar.w.live && Subwave.likeEnabled
                    enabled: !Subwave.liked && Subwave.likeBlockedMs === 0
                    text: Subwave.liked ? "♥" : "♡"
                    font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.3
                    ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay; ToolTip.text: Subwave.liked ? qsTr("Beğendin") : qsTr("Beğen")
                    onClicked: Subwave.like()
                }
                ToolButton {
                    property real last: 100
                    icon.name: Player.volume > 0 ? "audio-volume-high" : "audio-volume-muted"
                    display: AbstractButton.IconOnly; text: qsTr("Sessiz")
                    onClicked: if (Player.volume > 0) { last = Player.volume; Player.volume = 0 } else Player.volume = last
                }
                // stepSize also gives the ticks under it (KDE's Slider draws one per step)
                Slider { objectName: "volume"; Layout.preferredWidth: Kirigami.Units.gridUnit * 6; from: 0; to: 100; stepSize: 5; wheelEnabled: true; value: Player.volume; onMoved: Player.volume = value }
            }
            D.Plain {
                objectName: "status"
                Layout.fillWidth: true; horizontalAlignment: Text.AlignRight; elide: Text.ElideLeft
                text: playerBar.w.notice || Player.status; color: playerBar.dim
                font.pointSize: playerBar.small
            }
        }
    }
}
