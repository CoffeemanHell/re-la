// Kirigami theme: KDE's own look (Kirigami on the Breeze style), the same on Linux and Windows.
// Window, left sidebar (Sidebar.qml), header (back / forward / refresh, search), one page area, player bar
// (PlayerBar.qml) and the login form (LoginPage.qml). What the pages load and do is in themes/shared, as for every theme.
// Pages reach the window as ApplicationWindow.window: the functions below are the same as the Default theme's.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import QtCore
import org.kde.kirigami as Kirigami
import shared
import Rela

Kirigami.ApplicationWindow {
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
    // One page area: Navigation keeps the history; each page replaces the last on the page stack (one column).
    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.None
    pageStack.columnView.columnResizeMode: Kirigami.ColumnView.SingleColumn
    pageStack.visible: usable
    // Kirigami moves the whole content right of a non-modal sidebar, and Qt then puts the header and footer that far
    // left of the window. Keep the content where it is (the bars span the window) and move only the pages.
    contentItem.anchors.leftMargin: 0
    pageStack.anchors.leftMargin: globalDrawer ? globalDrawer.width * globalDrawer.position : 0
    property url lastPageUrl // the page file shown last (tests check it comes from this theme)
    Navigation {
        id: route
        loader: shown; searchField: q
        onLoad: (file, props) => {
            const url = Qt.resolvedUrl(file)
            win.lastPageUrl = url
            const typing = q.activeFocus // the page stack gives a new page the focus: typing a search must go on
            if (win.pageStack.depth) win.pageStack.replace(url, props)
            else win.pageStack.push(url, props)
            if (typing) q.forceActiveFocus()
        }
    }
    QtObject { // what Navigation calls its loader: the page shown
        id: shown
        readonly property Item item: win.pageStack.currentItem
        property string source // Navigation clears it on logout; start() empties the stack then
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
    function start() { route.start(); if (route.at < 0) pageStack.clear() }
    Connections { target: Player; function onStatusChanged() { win.start() } }
    TapHandler {
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onTapped: (point, button) => button === Qt.BackButton ? win.back() : win.forward()
    }

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
    function confirm(text, cb) { confirmDialog.subtitle = text; confirmDialog.cb = cb; confirmDialog.open() }
    Kirigami.PromptDialog {
        id: nameDialog
        property var cb: null
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onOpened: nameField.forceActiveFocus()
        onAccepted: if (nameField.text.trim()) cb(nameField.text.trim())
        TextField { id: nameField; onAccepted: nameDialog.accept() }
    }
    Kirigami.PromptDialog {
        id: confirmDialog
        property var cb: null
        standardButtons: Kirigami.Dialog.Yes | Kirigami.Dialog.No
        onAccepted: cb()
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

    // ---------------------------------------------------------------- sidebar, header, player bar, login
    globalDrawer: Sidebar { w: win }

    header: ToolBar {
        visible: win.usable
        component Tool: ToolButton {
            property string tip
            display: AbstractButton.IconOnly
            text: tip
            ToolTip.visible: hovered; ToolTip.text: tip; ToolTip.delay: Kirigami.Units.toolTipDelay
        }
        RowLayout {
            anchors.fill: parent
            spacing: Kirigami.Units.smallSpacing
            Tool { objectName: "back"; icon.name: "go-previous"; tip: qsTr("Geri"); enabled: win.canBack; onClicked: win.back() }
            Tool { objectName: "forward"; icon.name: "go-next"; tip: qsTr("İleri"); enabled: win.canForward; onClicked: win.forward() }
            Tool { objectName: "refresh"; icon.name: "view-refresh"; tip: qsTr("Yenile"); onClicked: win.reload() }
            Item { Layout.fillWidth: true }
            // listening together: who we follow (click leaves), or how many are online (relay not reachable: hidden)
            ToolButton {
                visible: Sync.available
                Layout.maximumWidth: Kirigami.Units.gridUnit * 14
                text: Sync.following ? qsTr("%1 ile dinliyorsun").arg(Sync.following)
                    : Sync.listeners.length ? qsTr("%n kişi seni dinliyor", "", Sync.listeners.length)
                    : Sync.users.length ? qsTr("%n kişi çevrimiçi", "", Sync.users.length) : qsTr("Yalnız")
                font.weight: Sync.listeners.length ? Font.DemiBold : Font.Normal
                onClicked: win.openTogether()
            }
            Kirigami.SearchField {
                id: q
                objectName: "search"
                Layout.preferredWidth: Kirigami.Units.gridUnit * 16
                placeholderText: qsTr("Şarkı, sanatçı, albüm ara…")
                autoAccept: false
                onTextEdited: searchTimer.restart()
                onAccepted: { searchTimer.stop(); win.search(text) }
            }
            Timer { id: searchTimer; interval: 300; onTriggered: win.search(q.text) }
        }
    }

    footer: PlayerBar { w: win }

    LoginPage {
        anchors.centerIn: parent
        visible: !win.usable
    }

    // ---------------------------------------------------------------- settings and listening together
    property alias settingsDialog: settingsDialog
    property alias togetherDialog: togetherDialog
    function openSettings(c) { settingsDialog.show(c) }
    function openTogether() { togetherDialog.show() }
    SettingsDialog { id: settingsDialog; w: win }
    TogetherDialog { id: togetherDialog; w: win }

    Component.onCompleted: {
        Util.plainTips(ToolTip.toolTip)
        geoReady = true
        start()
        // --settings opens the panel at start (handy while designing a theme with --demo)
        if (Qt.application.arguments.indexOf("--settings") >= 0) openSettings("Genel")
    }
    Shortcut { sequences: Shortcuts.sequences("keyToggle"); onActivated: Player.toggle() }
    Shortcut { sequences: Shortcuts.sequences("keySearch"); onActivated: q.forceActiveFocus() }
    Shortcut { sequences: Shortcuts.sequences("keyNext"); onActivated: Player.next() }
    Shortcut { sequences: Shortcuts.sequences("keyPrev"); onActivated: Player.prev() }
    Shortcut { sequences: Shortcuts.sequences("keyBack"); onActivated: win.back() }
    Shortcut { sequences: Shortcuts.sequences("keyForward"); onActivated: win.forward() }
}
