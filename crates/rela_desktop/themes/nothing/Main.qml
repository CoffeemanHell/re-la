// Nothing-style theme: monochrome, dot-matrix display type, dot grid, one red accent.
// A theme is this folder: Main.qml (window, top bar, player bar) and one file per page and dialog.
// It only draws: what the pages load and do is in themes/shared (`import shared`), the same for every App.theme.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Dialogs
import QtCore
import shared
import Rela

ApplicationWindow {
    id: win
    width: Math.max(minimumWidth, geo.w); height: Math.max(minimumHeight, geo.h); minimumWidth: 920; minimumHeight: 600
    visible: true
    visibility: geo.max ? Window.Maximized : Window.AutomaticVisibility
    // size (and maximized) kept between runs; Wayland gives windows no position, so none is kept
    Settings { id: geo; category: "window"; property int w: 1200; property int h: 760; property bool max: false }
    property bool geoReady: false // set once the saved size is applied (onCompleted), so the startup size isn't saved over it
    onWidthChanged: if (geoReady && visibility === Window.Windowed) geo.w = width
    onHeightChanged: if (geoReady && visibility === Window.Windowed) geo.h = height
    onVisibilityChanged: if (geoReady && (visibility === Window.Windowed || visibility === Window.Maximized)) geo.max = visibility === Window.Maximized
    title: Player.station.id ? Player.station.name + " · re-la" : Player.current.title ? Player.current.title + " · " + Player.current.artist : "re-la"
    color: App.theme.bg
    font.family: App.theme.font
    font.pixelSize: App.theme.fontSize

    readonly property bool usable: Player.online || Player.offline // offline: no server, but songs on disk
    readonly property bool hasSong: Player.current.id !== undefined
    readonly property bool live: !!Player.station.id && Subwave.active // a SUB/WAVE station is playing
    function fmt(t) { t = Math.max(0, Math.floor(t || 0)); return String(Math.floor(t / 60)).padStart(2, "0") + ":" + String(t % 60).padStart(2, "0") }
    function tag(s) { return s && s.suffix ? s.suffix.toUpperCase() + (s.bitDepth ? " · " + s.bitDepth + "BIT · " + s.samplingRate / 1000 + "KHZ" : "") : "" }
    function up(s) { return (s || "").toUpperCase() }

    // ---------------------------------------------------------------- pages and history (same as the default theme)
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
    component Box: Dialog {
        id: box
        anchors.centerIn: Overlay.overlay; modal: true; padding: 24
        background: Rectangle { color: App.theme.surface; border.width: 1; border.color: App.theme.line }
        header: Mono { text: win.up(box.title); padding: 24; bottomPadding: 0; color: App.theme.muted }
    }
    Box {
        id: nameDialog
        property var cb: null
        width: 380
        onOpened: nameField.forceActiveFocus()
        onAccepted: if (nameField.text.trim()) cb(nameField.text.trim())
        ColumnLayout {
            anchors.left: parent.left; anchors.right: parent.right
            spacing: 16
            Field { id: nameField; Layout.fillWidth: true; onAccepted: nameDialog.accept() }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                Pill { text: qsTr("VAZGEÇ"); onClicked: nameDialog.reject() }
                Pill { text: qsTr("TAMAM"); on: true; onClicked: nameDialog.accept() }
            }
        }
    }
    Box {
        id: confirmDialog
        property var cb: null
        property string text
        ColumnLayout {
            spacing: 16
            Mono { text: win.up(confirmDialog.text) }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                Pill { text: qsTr("HAYIR"); onClicked: confirmDialog.reject() }
                Pill { text: qsTr("EVET"); on: true; danger: true; onClicked: confirmDialog.accept() }
            }
        }
        onAccepted: cb()
    }

    // ---------------------------------------------------------------- shared menu, track info, downloads (same API as default)
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

    // ---------------------------------------------------------------- top bar
    header: Rectangle {
        visible: win.usable
        implicitHeight: 108
        color: App.theme.bg
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: App.theme.line }
        RowLayout {
            anchors.left: parent.left; anchors.right: parent.right; y: 14
            anchors.leftMargin: 36; anchors.rightMargin: 36
            spacing: 8
            Pill { text: qsTr("ANA SAYFA"); onClicked: win.go("home") }
            Pill { text: qsTr("← GERİ"); enabled: win.canBack; onClicked: win.back() }
            Pill { text: qsTr("İLERİ →"); enabled: win.canForward; onClicked: win.forward() }
            Pill { text: qsTr("YENİLE"); onClicked: win.reload() }
            Item { Layout.fillWidth: true }
            Field {
                id: q
                Layout.preferredWidth: 200
                placeholderText: qsTr("ARA")
                onTextEdited: searchTimer.restart()
                onAccepted: { searchTimer.stop(); win.search(text) }
            }
            Timer { id: searchTimer; interval: 300; onTriggered: win.search(q.text) }
            Mono {
                visible: Sync.available
                text: Sync.following ? qsTr("%1 İLE").arg(win.up(Sync.following))
                    : Sync.listeners.length ? qsTr("%n KİŞİ SENİ DİNLİYOR", "", Sync.listeners.length)
                    : Sync.users.length ? qsTr("%n ÇEVRİMİÇİ", "", Sync.users.length) : qsTr("YALNIZ")
                color: Sync.following || Sync.listeners.length ? App.theme.fg : App.theme.muted
                TapHandler { onTapped: win.openTogether() }
                HoverHandler { cursorShape: Qt.PointingHandCursor }
            }
            Pill { visible: Sync.following !== ""; text: qsTr("AYRIL"); onClicked: Sync.unfollow() }
            Pill { text: qsTr("AYARLAR"); onClicked: win.openSettings("Genel") }
        }
        // the pages, on their own row so every label fits at the minimum width
        Row {
            objectName: "nav"
            anchors.horizontalCenter: parent.horizontalCenter; y: 58
            spacing: 8
            Repeater {
                model: [["nowplaying", qsTr("ŞİMDİ ÇALAN")], ["favorites", qsTr("FAVORİLER")], ["albums", qsTr("ALBÜMLER")], ["artists", qsTr("SANATÇILAR")],
                        ["genres", qsTr("TÜRLER")], ["playlists", qsTr("PLAYLIST'LER")], ["tracks", qsTr("TÜM PARÇALAR")], ["radio", qsTr("RADYO")]]
                Pill { required property var modelData; text: modelData[1]; on: win.page === modelData[0]; onClicked: win.go(modelData[0]) }
            }
        }
    }

    // ---------------------------------------------------------------- player bar
    footer: Rectangle {
        id: playerBar
        visible: win.usable
        implicitHeight: 96
        color: App.theme.bg
        z: 1 // the skip undo popup overlaps the page above the bar
        Rectangle { width: parent.width; height: 1; color: App.theme.line }
        Rectangle { // a skip the DJ will hold against the song, for a few seconds: take it back
            objectName: "skipUndoPopup"
            visible: !!Player.skipUndo.id
            x: playerRow.x + middleColumn.x + transport.x + transport.width / 2 - width / 2 // centred on the transport row
            y: -height - 8
            z: 2
            width: undoRow.implicitWidth + 28; height: undoRow.implicitHeight + 14
            color: App.theme.bg
            border.width: 1; border.color: App.theme.line
            RowLayout {
                id: undoRow
                anchors.centerIn: parent
                spacing: 12
                Mono { text: qsTr("ATLAMA EKSİ SAYILACAK"); color: App.theme.muted; font.pixelSize: 9; ToolTip.visible: undoHover.hovered; ToolTip.text: Player.skipUndo.title || ""; HoverHandler { id: undoHover } }
                Pill { text: qsTr("GERİ AL"); implicitHeight: 20; onClicked: Player.undoSkip() }
            }
        }
        RowLayout {
            id: playerRow
            anchors.fill: parent
            anchors.leftMargin: 36; anchors.rightMargin: 36
            spacing: 28

            RowLayout {
                Layout.preferredWidth: 300; Layout.maximumWidth: 300
                spacing: 14
                Rectangle {
                    id: liveDot
                    implicitWidth: 8; implicitHeight: 8; radius: 4
                    color: Player.playing ? App.theme.accent : App.theme.muted
                    SequentialAnimation on opacity {
                        running: Player.playing; loops: Animation.Infinite
                        onRunningChanged: if (!running) liveDot.opacity = 1
                        NumberAnimation { to: 0.15; duration: 650 }
                        NumberAnimation { to: 1; duration: 650 }
                    }
                }
                Cover {
                    Layout.preferredWidth: 56; Layout.preferredHeight: 56
                    art: Player.station.id ? "" : Player.current.coverArt || ""
                    url: win.live ? Subwave.coverUrl : ""
                    TapHandler { onTapped: win.go("nowplaying") }
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3
                    RowLayout {
                        spacing: 6
                        Mono {
                            Layout.fillWidth: true; Layout.maximumWidth: Math.ceil(implicitWidth); font.bold: true
                            text: win.live && Subwave.now.title ? win.up(Subwave.now.title) : Player.station.id ? win.up(Player.station.name) : win.up(Player.current.title) || qsTr("SESSİZLİK")
                        }
                        Mono {
                            objectName: "songMenu"
                            visible: !Player.station.id && win.hasSong
                            text: "⋮"; font.bold: true
                            TapHandler { onTapped: win.menu("song", Player.current, { queueIndex: Player.index }) }
                            HoverHandler { cursorShape: Qt.PointingHandCursor }
                        }
                        Item { Layout.fillWidth: true }
                    }
                    Mono { visible: !!Player.station.id; text: qsTr("● CANLI"); color: App.theme.accent }
                    Mono { Layout.fillWidth: true; visible: win.live; text: win.up(Subwave.now.artist) }
                    Link {
                        Layout.fillWidth: true
                        visible: !Player.station.id
                        text: win.up(Player.current.artist)
                        active: !!Player.current.artistId
                        onActivated: win.go("artist", { id: Player.current.artistId })
                    }
                    Link {
                        Layout.fillWidth: true
                        text: win.up(Player.current.album); color: App.theme.muted
                        visible: !Player.station.id
                        active: !!Player.current.albumId
                        onActivated: win.go("album", { id: Player.current.albumId })
                    }
                }
            }

            ColumnLayout {
                id: middleColumn
                Layout.fillWidth: true
                spacing: 8
                RowLayout {
                    id: transport
                    objectName: "transport"
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 12
                    Pill { objectName: "shuffle"; text: qsTr("KARIŞIK"); on: Player.shuffle; enabled: Sync.canControl && !Player.station.id; onClicked: Player.shuffle = !Player.shuffle }
                    Circle { size: 36; text: "⏮"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.prev() }
                    Circle { size: 44; filled: true; text: Player.playing ? "⏸" : "▶"; enabled: Sync.canControl; onClicked: Player.toggle() }
                    Circle { size: 36; text: "⏭"; enabled: Sync.canControl && !Player.station.id; onClicked: Player.next() }
                    Pill { objectName: "repeat"; text: Player.repeat === 2 ? qsTr("TEK") : qsTr("TEKRAR"); on: Player.repeat > 0; enabled: Sync.canControl && !Player.station.id; onClicked: Player.repeat = (Player.repeat + 1) % 3 }
                }
                RowLayout { // SUB/WAVE: how far into the track the station is (read only, there is no seek in a stream)
                    readonly property real total: Subwave.now.duration || 0
                    readonly property real done: total > 0 ? Math.min(Subwave.elapsedMs / 1000, total) : Subwave.elapsedMs / 1000
                    visible: win.live
                    spacing: 12
                    Mono { text: win.fmt(parent.done); color: App.theme.muted }
                    DotBar {
                        objectName: "swProgress"
                        Layout.fillWidth: true; visible: parent.total > 0
                        enabled: false
                        dots: Math.max(10, Math.floor(width / 10))
                        value: parent.done / Math.max(1, parent.total)
                    }
                    Item { Layout.fillWidth: true; visible: parent.total <= 0 }
                    Mono { visible: parent.total > 0; text: win.fmt(parent.total); color: App.theme.muted }
                }
                Item { visible: !!Player.station.id && !win.live; Layout.fillWidth: true; Layout.preferredHeight: 16 } // a stream without a timeline keeps the row's height
                RowLayout {
                    spacing: 12
                    visible: !Player.station.id // a live stream has no timeline
                    Mono { text: win.fmt(Player.position); color: App.theme.muted }
                    DotBar {
                        Layout.fillWidth: true
                        dots: Math.max(10, Math.floor(width / 10))
                        levels: Player.waveform
                        enabled: Sync.canControl && win.hasSong
                        value: Player.duration ? Player.position / Player.duration : 0
                        onPicked: v => Player.seek(v * Player.duration)
                    }
                    Mono { text: win.fmt(Player.duration); color: App.theme.muted }
                }
            }

            ColumnLayout {
                Layout.preferredWidth: 300; Layout.maximumWidth: 300
                spacing: 8
                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 10
                    Pill { // SUB/WAVE: heart the track on air
                        objectName: "swHeart"
                        visible: win.live && Subwave.likeEnabled
                        enabled: !Subwave.liked && Subwave.likeBlockedMs === 0
                        text: Subwave.liked ? "♥" : "♡"; on: Subwave.liked
                        onClicked: Subwave.like()
                    }
                    Pill { text: "DJ"; on: Player.dj; onClicked: Player.dj = !Player.dj }
                }
                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 10
                    Mono { // click mutes, as the default theme's speaker button
                        id: mute
                        property real last: 100
                        text: Player.volume > 0 ? qsTr("SES") : qsTr("SESSİZ"); color: App.theme.muted
                        TapHandler { onTapped: if (Player.volume > 0) { mute.last = Player.volume; Player.volume = 0 } else Player.volume = mute.last }
                        HoverHandler { cursorShape: Qt.PointingHandCursor }
                    }
                    DotBar {
                        objectName: "volume"
                        Layout.preferredWidth: 130; dots: 16; live: true; value: Player.volume / 100
                        onPicked: v => Player.volume = Math.round(v * 100)
                        WheelHandler { // one notch = 5, as in the default theme
                            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad // KDE's button scroll arrives as a touchpad
                            property int turned: 0
                            onWheel: e => {
                                turned += e.angleDelta.y
                                const n = Math.trunc(turned / 120)
                                turned -= n * 120
                                if (n) Player.volume = Math.max(0, Math.min(100, Player.volume + n * 5))
                            }
                        }
                    }
                    Mono { text: String(Math.round(Player.volume)).padStart(3, "0"); color: App.theme.muted }
                }
                Mono { objectName: "status"; Layout.fillWidth: true; horizontalAlignment: Text.AlignRight; elide: Text.ElideLeft; text: win.up(win.notice || Player.status); color: App.theme.muted; font.pixelSize: 9 }
            }
        }
    }

    // ---------------------------------------------------------------- pages (first child: test grabs rely on it)
    Item {
        anchors.fill: parent
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

    // ---------------------------------------------------------------- dot grid backdrop
    Canvas {
        anchors.fill: parent
        z: -1
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const c = getContext("2d")
            c.reset()
            c.fillStyle = App.theme.grid
            for (let y = 10; y < height; y += 20)
                for (let x = 10; x < width; x += 20)
                    c.fillRect(x, y, 2, 2)
        }
    }

    // ---------------------------------------------------------------- login
    ColumnLayout {
        visible: !win.usable
        anchors.centerIn: parent
        width: 380
        spacing: 18
        Dots { text: "NSYNC"; font.pixelSize: 64; Layout.alignment: Qt.AlignHCenter }
        Mono { text: qsTr("KAYIPSIZ · BİRLİKTE"); color: App.theme.muted; Layout.alignment: Qt.AlignHCenter }
        Item { Layout.preferredHeight: 12 }
        // saved login: no form, just the connection status (with retries) and a way out
        Field { id: url; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("SUNUCU (HTTPS://MUZIK.ORNEK.COM)"); text: Player.setting("url") }
        Field { id: user; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("KULLANICI ADI"); text: Player.setting("user") }
        Field { id: pass; visible: !Player.signedIn; Layout.fillWidth: true; placeholderText: qsTr("ŞİFRE"); echoMode: TextInput.Password; onAccepted: connectBtn.clicked() }
        Item { Layout.preferredHeight: 8 }
        Mono {
            visible: Player.connecting
            Layout.alignment: Qt.AlignHCenter
            text: "● ● ●"; color: App.theme.accent
            SequentialAnimation on opacity {
                running: Player.connecting; loops: Animation.Infinite
                NumberAnimation { to: 0.2; duration: 500 }
                NumberAnimation { to: 1; duration: 500 }
            }
        }
        Pill {
            id: connectBtn
            Layout.alignment: Qt.AlignHCenter
            text: Player.signedIn ? qsTr("VAZGEÇ") : qsTr("BAĞLAN") // while connecting: "VAZGEÇ", a hanging connect can be abandoned
            on: !Player.signedIn
            onClicked: Player.signedIn ? Player.logout() : Player.login(url.text, user.text, pass.text)
        }
        Mono {
            Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap; elide: Text.ElideNone
            text: win.up(Player.status)
            color: Player.loginFailed ? App.theme.accent : App.theme.muted
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
    Component.onCompleted: {
        if (ToolTip.toolTip.contentItem) ToolTip.toolTip.contentItem.textFormat = Text.PlainText // tips carry song names: never markup
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
