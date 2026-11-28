// Default theme: native look (the system's Qt Quick Controls style, palette and icon theme).
// A theme is this folder: Main.qml (window, toolbar, player bar) and one file per page and dialog.
// It only draws: what the pages load and do is in themes/shared (`import shared`), the same for every App.theme.
// Everything comes from the re-la singletons (Player, Sync, Subwave, App). Pages reach the window as ApplicationWindow.window.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import QtCore
import shared
import Rela

ApplicationWindow {
    id: win
    width: Math.max(minimumWidth, geo.w); height: Math.max(minimumHeight, geo.h); minimumWidth: 900; minimumHeight: 600
    visible: true
    visibility: geo.max ? Window.Maximized : Window.AutomaticVisibility
    // size (and maximized) kept between runs; Wayland gives windows no position, so none is kept
    Settings { id: geo; category: "window"; property int w: 1200; property int h: 760; property bool max: false }
    property bool geoReady: false // set once the saved size is applied (onCompleted), so the startup size isn't saved over it
    onWidthChanged: if (geoReady && visibility === Window.Windowed) geo.w = width
    onHeightChanged: if (geoReady && visibility === Window.Windowed) geo.h = height
    onVisibilityChanged: if (geoReady && (visibility === Window.Windowed || visibility === Window.Maximized)) geo.max = visibility === Window.Maximized
    title: Player.station.id ? Player.station.name + " · re-la" : Player.current.title ? Player.current.title + " · " + Player.current.artist : "re-la"
    color: palette.window

    readonly property bool usable: Player.online || Player.offline // offline: no server, but songs on disk
    readonly property bool hasSong: Player.current.id !== undefined
    readonly property bool live: !!Player.station.id && Subwave.active // a SUB/WAVE station is playing
    function fmt(t) { t = Math.max(0, Math.floor(t || 0)); return Math.floor(t / 60) + ":" + String(t % 60).padStart(2, "0") }
    function tag(s) {
        if (!s || !s.suffix) return ""
        const t = s.suffix.toUpperCase()
        return s.bitDepth ? t + " " + s.bitDepth + "/" + (s.samplingRate / 1000) : t
    }

    // ---------------------------------------------------------------- pages and history
    Navigation {
        id: route
        loader: view; searchField: q
        onLoad: (file, props) => { view.setSource(file, props); fade.restart() }
    }
    property alias history: route.history // [{page, args}]
    property alias at: route.at
    readonly property bool canBack: route.canBack
    readonly property bool canForward: route.canForward
    readonly property string page: route.page
    function go(page, args) { route.go(page, args) }
    function back() { route.back() }
    function forward() { route.forward() }
    function reload() { route.reload() } // fresh from the server
    function search(text) { route.search(text) }
    function start() { route.start() }
    Connections { target: Player; function onStatusChanged() { win.start() } }

    // short messages from pages ("İşlem başarısız"), shown in the player bar's status line for 3 s
    property string notice: ""
    function notify(text) { notice = text; noticeTimer.restart() }
    Timer { id: noticeTimer; interval: 3000; onTriggered: win.notice = "" }
    Connections {
        target: Sync
        function onListenerJoined(name) { win.notify(qsTr("%1 seni dinliyor").arg(name)) }
        function onListenerLeft(name) { win.notify(qsTr("%1 ayrıldı").arg(name)) }
    }
    // a name from the user (new / renamed playlist); cb(name) runs only for a non-empty name
    function askName(title, text, cb) { nameDialog.title = title; nameField.text = text; nameDialog.cb = cb; nameDialog.open() }
    function confirm(text, cb) { confirmDialog.text = text; confirmDialog.cb = cb; confirmDialog.open() }
    Dialog {
        id: nameDialog
        property var cb: null
        anchors.centerIn: Overlay.overlay; modal: true; width: 360
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: nameField.forceActiveFocus()
        onAccepted: if (nameField.text.trim()) cb(nameField.text.trim())
        TextField { id: nameField; anchors.left: parent.left; anchors.right: parent.right; onAccepted: nameDialog.accept() }
    }
    Dialog {
        id: confirmDialog
        property var cb: null
        property alias text: confirmLabel.text
        anchors.centerIn: Overlay.overlay; modal: true
        standardButtons: Dialog.Yes | Dialog.No
        onAccepted: cb()
        Plain { id: confirmLabel }
    }

    // ---------------------------------------------------------------- shared menu, track info, downloads
    // menu(kind, item, opts): the right-click menu of a song / album / artist / playlist at the cursor.
    // opts: queueIndex (a queue row), onRemove (a playlist row: "Playlist'ten çıkar")
    function menu(kind, item, opts) {
        opts = opts || {}
        itemMenu.kind = kind
        itemMenu.item = item
        itemMenu.queueIndex = opts.queueIndex === undefined ? -1 : opts.queueIndex
        itemMenu.removeFn = opts.onRemove || null
        itemMenu.popup()
    }
    function info(id) { infoDialog.show(id) }
    function download(item, kind) {
        saveDialog.item = item
        saveDialog.selectedFile = saveDialog.currentFolder + "/" + Util.fileName(item, kind)
        saveDialog.open()
    }
    ItemMenu { id: itemMenu; w: win }
    InfoDialog { id: infoDialog }
    FileDialog {
        id: saveDialog
        property var item: null
        title: qsTr("İndir")
        fileMode: FileDialog.SaveFile
        currentFolder: StandardPaths.writableLocation(StandardPaths.DownloadLocation)
        onAccepted: Player.download(item.id, selectedFile)
    }

    // ---------------------------------------------------------------- toolbar
    header: ToolBar {
        id: bar
        visible: win.usable
        component Tool: ToolButton {
            property string tip
            display: AbstractButton.IconOnly
            ToolTip.visible: hovered && tip !== ""; ToolTip.text: tip; ToolTip.delay: 500
        }
        component Nav: ToolButton {
            required property string target
            display: win.width >= 1600 ? AbstractButton.TextBesideIcon : AbstractButton.IconOnly
            checked: win.page === target
            onClicked: win.go(target)
            ToolTip.visible: hovered && display === AbstractButton.IconOnly; ToolTip.text: text; ToolTip.delay: 500
        }
        // the page buttons sit at the window's centre; the side groups keep to their half
        readonly property real side: (width - nav.width) / 2 - 12
        RowLayout {
            anchors.left: parent.left; anchors.leftMargin: 6
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Tool { icon.name: "go-home"; tip: qsTr("Ana sayfa"); onClicked: win.go("home") }
            Tool { icon.name: "go-previous"; tip: qsTr("Geri"); enabled: win.canBack; onClicked: win.back() }
            Tool { icon.name: "go-next"; tip: qsTr("İleri"); enabled: win.canForward; onClicked: win.forward() }
            Tool { icon.name: "view-refresh"; tip: qsTr("Yenile"); onClicked: win.reload() }
        }
        Row {
            id: nav
            objectName: "nav"
            anchors.centerIn: parent
            spacing: 2
            Nav { target: "nowplaying"; icon.name: Util.icon("audio-headphones"); text: qsTr("Şimdi çalan") }
            Nav { target: "favorites"; icon.name: "emblem-favorite"; text: qsTr("Favoriler") }
            Nav { target: "albums"; icon.name: Util.icon("media-optical-audio"); text: qsTr("Albümler") }
            Nav { target: "artists"; icon.name: Util.icon("view-media-artist"); text: qsTr("Sanatçılar") }
            Nav { target: "genres"; icon.name: Util.icon("view-media-genre"); text: qsTr("Türler") }
            Nav { target: "playlists"; icon.name: Util.icon("view-media-playlist"); text: qsTr("Playlist'ler") }
            Nav { target: "tracks"; icon.name: Util.icon("view-media-track"); text: qsTr("Tüm parçalar") }
            Nav { target: "radio"; icon.name: Util.icon("radio"); text: qsTr("Radyo") }
        }
        RowLayout {
            anchors.right: parent.right; anchors.rightMargin: 6
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, bar.side)

            spacing: 2
            TextField {
                id: q
                Layout.fillWidth: true; Layout.preferredWidth: 240; Layout.minimumWidth: 80
                placeholderText: qsTr("Şarkı, sanatçı, albüm ara…")
                onTextEdited: searchTimer.restart()
                onAccepted: { searchTimer.stop(); win.search(text) }
            }
            Timer { id: searchTimer; interval: 300; onTriggered: win.search(q.text) }
            // listening together: who we follow (click leaves), or how many are online (relay not reachable: hidden)
            ToolButton {
                visible: Sync.available
                Layout.maximumWidth: 220
                text: Sync.following ? qsTr("%1 ile dinliyorsun").arg(Sync.following)
                    : Sync.listeners.length ? qsTr("%n kişi seni dinliyor", "", Sync.listeners.length)
                    : Sync.users.length ? qsTr("%n kişi çevrimiçi", "", Sync.users.length) : qsTr("Yalnız")
                font.weight: Sync.listeners.length ? Font.DemiBold : Font.Normal
                onClicked: win.openTogether()
            }
            Tool { icon.name: Util.icon("configure"); tip: qsTr("Ayarlar"); onClicked: win.openSettings("Genel") }
        }
    }

    // ---------------------------------------------------------------- player bar
    footer: ToolBar {
        id: playerBar
        visible: win.usable
        implicitHeight: 80
        Rectangle { // a skip the DJ will hold against the song, for a few seconds: take it back
            objectName: "skipUndoPopup"
            parent: win.contentItem // above the bar, on the page: KDE's ToolBar takes no clicks outside itself
            visible: !!Player.skipUndo.id
            x: playerBar.leftPadding + playerRow.x + middleColumn.x + transport.x + transport.width / 2 - width / 2 // centred on the transport row
            y: parent.height - height - 8
            z: 2
            width: undoRow.implicitWidth + 24; height: undoRow.implicitHeight + 12
            radius: 6
            color: palette.window
            border.width: 1; border.color: palette.mid
            RowLayout {
                id: undoRow
                anchors.centerIn: parent
                spacing: 6
                Plain {
                    text: qsTr("Atlama eksi sayılacak"); color: palette.placeholderText
                    font.pointSize: Qt.application.font.pointSize * 0.8
                    ToolTip.visible: undoHover.hovered; ToolTip.text: Player.skipUndo.title || ""
                    HoverHandler { id: undoHover }
                }
                Button { text: qsTr("Geri al"); flat: true; implicitHeight: 24; onClicked: Player.undoSkip() }
            }
        }
        RowLayout {
            id: playerRow
            anchors.fill: parent
            anchors.leftMargin: 10; anchors.rightMargin: 10
            spacing: 16

            RowLayout {
                Layout.preferredWidth: 350; Layout.maximumWidth: 350; Layout.minimumWidth: 240
                spacing: 10
                Cover {
                    Layout.preferredWidth: 56; Layout.preferredHeight: 56
                    art: Player.station.id ? "" : Player.current.coverArt || ""
                    url: win.live ? Subwave.coverUrl : ""
                    TapHandler { onTapped: win.go("nowplaying") }
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    RowLayout {
                        spacing: 0
                        Plain {
                            Layout.fillWidth: true; Layout.maximumWidth: Math.ceil(implicitWidth) // layouts round down; a fractional width would elide the last letter
                            elide: Text.ElideRight; font.weight: Font.DemiBold
                            text: win.live && Subwave.now.title ? Subwave.now.title : Player.station.id ? Player.station.name : Player.current.title || qsTr("Çalan bir şey yok")
                        }
                        ToolButton {
                            objectName: "songMenu"
                            visible: !Player.station.id && win.hasSong
                            text: "⋮"; implicitWidth: 24; implicitHeight: 24
                            onClicked: win.menu("song", Player.current, { queueIndex: Player.index })
                        }
                        Item { Layout.fillWidth: true }
                    }
                    Plain { visible: !!Player.station.id; text: qsTr("● Canlı"); color: "#e5484d" }
                    Plain { Layout.fillWidth: true; elide: Text.ElideRight; visible: win.live; text: Subwave.now.artist || "" }
                    Link {
                        Layout.fillWidth: true
                        visible: !Player.station.id
                        text: Player.current.artist || ""
                        active: !!Player.current.artistId
                        onActivated: win.go("artist", { id: Player.current.artistId })
                    }
                    Link {
                        Layout.fillWidth: true
                        text: Player.current.album || ""
                        color: palette.placeholderText
                        visible: !Player.station.id
                        active: !!Player.current.albumId
                        onActivated: win.go("album", { id: Player.current.albumId })
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
                    ToolButton {
                        objectName: "shuffle"
                        icon.name: "media-playlist-shuffle"
                        checkable: true; checked: Player.shuffle
                        enabled: Sync.canControl && !Player.station.id
                        onClicked: Player.shuffle = !Player.shuffle
                        ToolTip.visible: hovered; ToolTip.delay: 500; ToolTip.text: qsTr("Karıştır")
                    }
                    ToolButton { icon.name: "media-skip-backward"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.prev() }
                    ToolButton {
                        icon.name: Player.playing ? "media-playback-pause" : "media-playback-start"
                        icon.width: 32; icon.height: 32
                        enabled: Sync.canControl
                        onClicked: Player.toggle()
                    }
                    ToolButton { icon.name: "media-skip-forward"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.next() }
                    ToolButton {
                        objectName: "repeat"
                        icon.name: Util.icon(Player.repeat === 2 ? "media-playlist-repeat-song" : "media-playlist-repeat")
                        checkable: true; checked: Player.repeat > 0
                        enabled: Sync.canControl && !Player.station.id
                        onClicked: Player.repeat = (Player.repeat + 1) % 3
                        ToolTip.visible: hovered; ToolTip.delay: 500; ToolTip.text: Player.repeat === 2 ? qsTr("Tekrar (tek şarkı)") : qsTr("Tekrar")
                    }
                }
                RowLayout { // SUB/WAVE: how far into the track the station is (read only, there is no seek in a stream)
                    readonly property real total: Subwave.now.duration || 0
                    readonly property real done: total > 0 ? Math.min(Subwave.elapsedMs / 1000, total) : Subwave.elapsedMs / 1000
                    visible: win.live
                    Layout.preferredHeight: 28
                    Plain { text: win.fmt(parent.done); color: palette.placeholderText; font.features: { "tnum": 1 } }
                    ProgressBar { Layout.fillWidth: true; visible: parent.total > 0; from: 0; to: Math.max(1, parent.total); value: parent.done }
                    Item { Layout.fillWidth: true; visible: parent.total <= 0 }
                    Plain { visible: parent.total > 0; text: win.fmt(parent.total); color: palette.placeholderText; font.features: { "tnum": 1 } }
                }
                Item { visible: !!Player.station.id && !win.live; Layout.fillWidth: true; Layout.preferredHeight: 28 } // a stream without a timeline keeps the row's height
                RowLayout {
                    visible: !Player.station.id // a live stream has no timeline
                    Plain { text: win.fmt(seek.pressed ? seek.value : Player.position); color: palette.placeholderText; font.features: { "tnum": 1 } }
                    Item {
                        Layout.fillWidth: true; implicitHeight: Math.max(28, seek.implicitHeight)
                        // the waveform shows through an invisible slider, which still takes clicks and drags
                        Wave { anchors.fill: parent; anchors.margins: 4; visible: Player.waveform.length > 0; levels: Player.waveform; rms: Player.waveformRms; value: seek.value / seek.to }
                        Slider {
                            id: seek
                            anchors.fill: parent
                            opacity: Player.waveform.length > 0 ? 0 : 1
                            enabled: Sync.canControl && win.hasSong
                            from: 0; to: Math.max(1, Player.duration)
                            onPressedChanged: if (!pressed) Player.seek(value)
                        }
                    }
                    Binding { target: seek; property: "value"; value: Player.position; when: !seek.pressed; restoreMode: Binding.RestoreNone }
                    Plain { text: win.fmt(Player.duration); color: palette.placeholderText; font.features: { "tnum": 1 } }
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
                        ToolTip.visible: hovered; ToolTip.delay: 500
                        ToolTip.text: qsTr("Açıkken dil ve tür ağırlıklarıyla seçer; kapalıyken Navidrome'un benzer şarkıları")
                    }
                    ToolButton { // SUB/WAVE: heart the track on air
                        objectName: "swHeart"
                        visible: win.live && Subwave.likeEnabled
                        enabled: !Subwave.liked && Subwave.likeBlockedMs === 0
                        text: Subwave.liked ? "♥" : "♡"
                        font.pointSize: Qt.application.font.pointSize * 1.3
                        ToolTip.visible: hovered; ToolTip.delay: 500; ToolTip.text: Subwave.liked ? qsTr("Beğendin") : qsTr("Beğen")
                        onClicked: Subwave.like()
                    }
                    ToolButton {
                        property real last: 100
                        icon.name: Player.volume > 0 ? "audio-volume-high" : "audio-volume-muted"
                        onClicked: if (Player.volume > 0) { last = Player.volume; Player.volume = 0 } else Player.volume = last
                    }
                    Slider { objectName: "volume"; Layout.preferredWidth: 100; from: 0; to: 100; stepSize: 5; wheelEnabled: true; value: Player.volume; onMoved: Player.volume = value }
                }
                Plain {
                    objectName: "status"
                    Layout.fillWidth: true; horizontalAlignment: Text.AlignRight; elide: Text.ElideLeft
                    text: win.notice || Player.status; color: palette.placeholderText
                    font.pointSize: Qt.application.font.pointSize * 0.8
                }
            }
        }
    }

    // ---------------------------------------------------------------- pages
    Rectangle {
        anchors.fill: parent
        color: palette.window
        visible: win.usable
        Loader {
            id: view
            anchors.fill: parent
            NumberAnimation on opacity { id: fade; from: 0; to: 1; duration: 120; running: false }
        }
        TapHandler {
            acceptedButtons: Qt.BackButton | Qt.ForwardButton
            onTapped: (point, button) => button === Qt.BackButton ? win.back() : win.forward()
        }
    }

    // ---------------------------------------------------------------- login
    Pane {
        anchors.centerIn: parent
        width: 400
        visible: !win.usable
        padding: 32
        ColumnLayout {
            anchors.fill: parent
            spacing: 12
            Plain { text: "re-la"; font.pointSize: Qt.application.font.pointSize * 2.2; font.weight: Font.Bold }
            Plain { text: qsTr("Navidrome üzerinden birlikte, kayıpsız dinle."); color: palette.placeholderText; Layout.bottomMargin: 10 }
            // saved login: no form, just the connection status (with retries) and a way out
            TextField { id: url; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("Sunucu (https://muzik.ornek.com)"); text: Player.setting("url") }
            TextField { id: user; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("Kullanıcı adı"); text: Player.setting("user") }
            TextField { id: pass; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("Şifre"); echoMode: TextInput.Password; onAccepted: connectBtn.clicked() }
            BusyIndicator { Layout.alignment: Qt.AlignHCenter; running: Player.connecting; visible: running }
            Button {
                id: connectBtn
                Layout.fillWidth: true; Layout.topMargin: 6
                highlighted: !Player.signedIn // while connecting this is "Vazgeç": a hanging connect can be abandoned

                text: Player.signedIn ? qsTr("Vazgeç") : qsTr("Bağlan")
                onClicked: Player.signedIn ? Player.logout() : Player.login(url.text, user.text, pass.text)
            }
            Plain {
                text: Player.status
                color: Player.loginFailed ? "#e5484d" : palette.placeholderText
                wrapMode: Text.WordWrap; Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
            }
        }
    }

    // ---------------------------------------------------------------- settings and listening together (the top bar opens them)
    property alias settingsDialog: settingsDialog
    property alias togetherDialog: togetherDialog
    function openSettings(c) { settingsDialog.show(c) }
    function openTogether() { togetherDialog.show() }
    SettingsDialog { id: settingsDialog; w: win }
    TogetherDialog { id: togetherDialog; w: win }

    // --settings opens the panel at start (handy while designing a theme with --demo)
    // Plain items (Rectangle, Text) take the system palette, controls the style's. They differ under
    // FluentWinUI3 (Windows): light boxes in a dark window. Set the style's colors on the root, so all match.
    function followStyle(item) {
        for (const group of ["active", "inactive", "disabled"])
            for (const role of ["window", "windowText", "base", "alternateBase", "text", "placeholderText", "button", "buttonText",
                                "brightText", "highlight", "highlightedText", "link", "linkVisited", "light", "midlight",
                                "mid", "dark", "shadow", "toolTipBase", "toolTipText", "accent"])
                item.palette[group][role] = Qt.binding(() => win.palette[group][role])
        // FluentWinUI3 leaves alternateBase (hover, cover placeholder) at the system's light value even when dark
        item.palette.alternateBase = Qt.binding(() => {
            const w = win.palette.window, a = win.palette.alternateBase, t = win.palette.windowText
            return (w.hslLightness < 0.5) === (a.hslLightness < 0.5) ? a : Qt.tint(w, Qt.rgba(t.r, t.g, t.b, 0.06))
        })
    }
    Component.onCompleted: {
        if (ToolTip.toolTip.contentItem) ToolTip.toolTip.contentItem.textFormat = Text.PlainText // tips carry song names: never markup
        for (const item of [contentItem, header, footer, Overlay.overlay]) followStyle(item) // header, footer, popups sit beside the content
        geoReady = true
        start()
        if (Qt.application.arguments.indexOf("--settings") >= 0) openSettings("Genel")
    }
    Shortcut { sequences: Shortcuts.sequences("keyToggle"); onActivated: Player.toggle() }
    Shortcut { sequences: Shortcuts.sequences("keySearch"); onActivated: q.forceActiveFocus() }
    Shortcut { sequences: Shortcuts.sequences("keyNext"); onActivated: Player.next() }
    Shortcut { sequences: Shortcuts.sequences("keyPrev"); onActivated: Player.prev() }
    Shortcut { sequences: Shortcuts.sequences("keyBack"); onActivated: win.back() }
    Shortcut { sequences: Shortcuts.sequences("keyForward"); onActivated: win.forward() }
}
