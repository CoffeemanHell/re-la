// The settings window: categories on the left, one page of switches and sliders per category.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import shared
import Rela

Dialog {
    id: d
    property var w // the window
    property string cat: "Genel"
    function show(c) { cat = c || "Genel"; open() }

    component LabeledSlider: ColumnLayout {
        id: ls
        property string label
        property string valueText
        property string hint
        property alias from: s.from
        property alias to: s.to
        property alias stepSize: s.stepSize
        property alias value: s.value
        property alias enabledSlider: s.enabled
        signal moved(real v)
        Layout.fillWidth: true
        spacing: 2
        RowLayout {
            Plain { text: ls.label; Layout.fillWidth: true }
            Plain { text: ls.valueText; color: ls.enabled ? palette.highlight : palette.windowText; font.weight: Font.DemiBold } // disabled: as faint as the label, not fainter
        }
        Slider { id: s; Layout.fillWidth: true; snapMode: Slider.SnapAlways; onMoved: ls.moved(value) }
        Hint { text: ls.hint }
    }
    component ComboRow: RowLayout { // laid out like a SwitchRow, a combo box in place of the switch
        id: cr
        property string label
        property string hint
        property alias box: cb.objectName
        property alias model: cb.model
        property alias currentIndex: cb.currentIndex
        property alias displayText: cb.displayText
        property real boxWidth: cb.implicitWidth
        signal picked(int i)
        Layout.fillWidth: true
        ColumnLayout {
            Layout.fillWidth: true; spacing: 2
            Plain { text: cr.label }
            Hint { text: cr.hint }
        }
        ComboBox { id: cb; Layout.preferredWidth: cr.boxWidth; onActivated: i => cr.picked(i) }
    }
    component Page: ColumnLayout {
        required property string name
        visible: d.cat === name
        Layout.fillWidth: true
        spacing: 10
    }

    property alias stateDialog: stateDialog
    readonly property DjData dj: DjData {}
    DjStateDialog { id: stateDialog; w: d.w }

    objectName: "settings"
    onAboutToShow: { Player.refreshAudioDevices(); d.dj.refreshFeedback() } // not opened: that waits for the style's opening animation
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(760, d.w.width - 40); height: Math.min(560, d.w.height - 40)
    padding: 0
    contentItem: RowLayout {
        spacing: 0
        ListView {
            id: catList
            Layout.preferredWidth: 190; Layout.fillHeight: true
            Layout.topMargin: 16
            readonly property var icons: ({ "Genel": "configure", "Çalma": "audio-volume-high", "DJ": "media-playlist-shuffle",
                                            "Önbellek": "drive-harddisk", "Klavye": "input-keyboard",
                                            "Scrobble": "view-statistics", "Hesap": "user-identity", "Hakkında": "help-about" })
            // the keys stay Turkish (d.cat, objectName, openSettings); qsTr shows them
            model: [QT_TR_NOOP("Genel"), QT_TR_NOOP("Çalma"), "DJ", QT_TR_NOOP("Önbellek"), QT_TR_NOOP("Klavye"), "Scrobble", QT_TR_NOOP("Hesap"), QT_TR_NOOP("Hakkında")]
            delegate: ItemDelegate {
                required property string modelData
                objectName: "cat_" + modelData
                width: ListView.view.width
                text: qsTr(modelData)
                icon.name: Util.icon(catList.icons[modelData])
                spacing: 6 // Breeze's own; Fluent has none here
                highlighted: d.cat === modelData
                onClicked: d.cat = modelData
            }
        }
        ToolSeparator { orientation: Qt.Vertical; Layout.fillHeight: true }
        Flickable {
            id: settingsFlick
            Layout.fillWidth: true; Layout.fillHeight: true
            contentHeight: sc.implicitHeight + 48
            clip: true
            ScrollBar.vertical: ScrollBar {}
            ColumnLayout {
                id: sc
                x: 24; y: 16; width: settingsFlick.width - 48
                spacing: 10
                RowLayout {
                    PageTitle { text: qsTr(d.cat); Layout.fillWidth: true }
                    ToolButton { icon.name: "window-close"; onClicked: d.close() }
                }

                Page {
                    name: "Genel"
                    ComboRow {
                        label: qsTr("Tema")
                        hint: qsTr("Uygulamanın görünümü.")
                        box: "themeBox"
                        model: App.themeNames.map(n => App.themeTitles[n] || n)
                        currentIndex: App.themeNames.indexOf(Player.theme)
                        onPicked: i => Player.theme = App.themeNames[i]
                    }
                    SwitchRow {
                        visible: App.trayAvailable
                        label: qsTr("Sistem tepsisi")
                        hint: qsTr("Tepside simge durur; pencereyi kapatınca müzik durmaz. Çıkış: simgeye sağ tık. Kapalıyken pencereyi kapatmak uygulamadan çıkar.")
                        checked: Player.options.closeToTray === true
                        onToggled: on => Player.setOption("closeToTray", on)
                    }
                    LabeledSlider {
                        label: qsTr("Izgara boyutu")
                        valueText: (Player.options.gridSize || 172) + " px"
                        hint: qsTr("Albüm ve sanatçı kartlarının genişliği.")
                        from: 120; to: 260; stepSize: 10; value: Player.options.gridSize || 172
                        onMoved: v => Player.setOption("gridSize", v)
                    }
                }

                Page {
                    name: "Çalma"
                    ComboRow {
                        readonly property var outputs: Util.outputs()
                        label: qsTr("Ses çıkışı")
                        hint: qsTr("Sesin çalacağı aygıt.")
                        box: "outputBox"
                        boxWidth: 300
                        model: outputs.map(o => o.description)
                        currentIndex: outputs.findIndex(o => o.name === (Player.options.audioDevice || "auto"))
                        displayText: currentIndex < 0 ? qsTr("Bağlı değil") : outputs[currentIndex].description
                        onPicked: i => Player.setOption("audioDevice", outputs[i].name)
                    }
                    LabeledSlider {
                        label: qsTr("Şarkı geçişi (crossfade)")
                        valueText: Player.crossfade ? qsTr("%1 sn").arg(Player.crossfade) : qsTr("Kapalı · gapless")
                        hint: Player.exclusive ? qsTr("Exclusive modda kapalı: ses kartını tek deck tutabilir.") : qsTr("0 = boşluksuz (gapless) ve bit-perfect geçiş.")
                        from: 0; to: 12; stepSize: 1; value: Player.crossfade
                        enabledSlider: !Player.exclusive && Sync.following === ""
                        onMoved: v => Player.crossfade = v
                    }
                    SwitchRow {
                        visible: Qt.platform.os === "windows"
                        label: "WASAPI Exclusive"
                        hint: qsTr("Ses kartını uygulamaya ayırır, Windows mikseri atlanır: bit-perfect.")
                        checked: Player.exclusive
                        onToggled: on => Player.exclusive = on
                    }
                    SwitchRow {
                        label: qsTr("Otomatik devam")
                        hint: qsTr("Kuyruk bitmeden benzer şarkılar eklenir. Nasıl seçileceğini DJ modu belirler.")
                        checked: Player.options.autoContinue === true
                        onToggled: on => Player.setOption("autoContinue", on)
                    }
                }

                Page {
                    name: "DJ"
                    onVisibleChanged: if (visible) d.dj.refreshFeedback()
                    SwitchRow {
                        label: qsTr("DJ modu")
                        hint: qsTr("Kapalıyken Navidrome'un benzer şarkıları olduğu gibi eklenir. Açıkken aşağıdaki ağırlıklarla, geniş bir havuzdan seçilir.")
                        checked: Player.dj
                        onToggled: on => Player.dj = on
                    }
                    component Knob: LabeledSlider {
                        required property var modelData
                        objectName: "knob_" + modelData.key
                        enabled: Player.dj
                        label: modelData.label
                        hint: modelData.hint
                        from: modelData.from; to: modelData.to; stepSize: modelData.step
                        value: Player.options[modelData.key] ?? modelData.from
                        valueText: DjKnobs.text(modelData, Player.options[modelData.key] ?? modelData.from)
                        onMoved: v => Player.setOption(modelData.key, Math.round(v))
                    }
                    Repeater { model: DjKnobs.main; delegate: Knob {} }
                    Switch { id: advanced; objectName: "djAdvanced"; text: qsTr("Gelişmiş ayarlar"); enabled: Player.dj }
                    Repeater { model: DjKnobs.advanced; delegate: Knob { visible: advanced.checked } }
                    RowLayout {
                        Hint { text: qsTr("DJ ayarlarını ilk hâline getirir."); Layout.fillWidth: true }
                        Button { text: qsTr("Varsayılanlara dön"); icon.name: Util.icon("edit-undo"); onClicked: Player.resetDjSettings() }
                    }
                    Plain { text: qsTr("Geri bildirimlerin"); font.weight: Font.DemiBold; Layout.topMargin: 6 }
                    Hint { visible: !d.dj.feedback.length; text: qsTr("Henüz yok. Bir şarkının menüsünden \"Bunun gibi daha çok\" ya da \"Bunu daha az çal\" diyebilirsin.") }
                    Repeater {
                        model: d.dj.feedback
                        delegate: RowLayout {
                            id: fb
                            required property var modelData
                            Layout.fillWidth: true
                            Plain { text: fb.modelData.dir > 0 ? "▲" : "▼"; color: fb.modelData.dir > 0 ? palette.highlight : palette.placeholderText }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 0
                                Plain { text: fb.modelData.title; elide: Text.ElideRight; Layout.fillWidth: true }
                                Hint { text: (fb.modelData.dir > 0 ? qsTr("Daha çok") : qsTr("Daha az")) + (fb.modelData.artist ? " · " + fb.modelData.artist : "") }
                            }
                            Button {
                                objectName: "fbRemove_" + fb.modelData.id
                                text: qsTr("Kaldır")
                                onClicked: d.dj.toggle(fb.modelData, fb.modelData.dir)
                            }
                        }
                    }
                    RowLayout {
                        Hint { text: qsTr("DJ'nin son seçimde tarttığı şarkılar ve puanları."); Layout.fillWidth: true }
                        Button { text: qsTr("DJ durumu"); icon.name: Util.icon("view-statistics"); onClicked: stateDialog.show() }
                    }
                    RowLayout {
                        Hint { text: qsTr("Atlamaları, dinlemeleri ve geri bildirimlerini siler. Kütüphane dizini kalır."); Layout.fillWidth: true }
                        Button { text: qsTr("DJ hafızasını sıfırla"); icon.name: Util.icon("edit-clear-all"); onClicked: { Player.resetDjMemory(); d.dj.feedback = [] } }
                    }
                }

                Page {
                    name: "Önbellek"
                    Hint { text: qsTr("Dinlediğin ve sıradaki şarkılar diske iner. Sunucuya ulaşılamayınca uygulama son kuyrukla açılır, inen şarkılar çalar. Kendin atladığın bir şarkı bir kez çalarken, bir kez de önbellek için iner.") }
                    ComboRow {
                        label: qsTr("Boyut sınırı")
                        hint: qsTr("Dolunca en uzun süredir dinlenmeyen silinir.")
                        box: "cacheBox"
                        model: Util.cacheSizes.map(g => g ? g + " GB" : qsTr("Kapalı"))
                        currentIndex: Util.cacheSizes.indexOf(Player.options.offlineGb || 0)
                        onPicked: i => Player.setOption("offlineGb", Util.cacheSizes[i])
                    }
                    LabeledSlider {
                        label: qsTr("Önden inen")
                        enabled: Player.options.offlineGb > 0
                        valueText: qsTr("%n şarkı", "", Player.options.offlineAhead || 0)
                        hint: qsTr("Kuyrukta sıradaki kaç şarkı diskte hazır olsun.")
                        from: 0; to: 20; stepSize: 1; value: Player.options.offlineAhead || 0
                        onMoved: v => Player.setOption("offlineAhead", v)
                    }
                    RowLayout {
                        Plain { text: qsTr("Diskte: %1").arg(Util.dataSize(Player.cacheBytes)); Layout.fillWidth: true }
                        Button { text: qsTr("Temizle"); icon.name: Util.icon("edit-clear-all"); enabled: Player.cacheBytes > 0; onClicked: Player.clearSongCache() }
                    }
                }

                Page {
                    name: "Klavye"
                    Hint { text: qsTr("Değiştirmek için kısayola bas, sonra yeni tuşlara. Vazgeçmek: Esc.") }
                    Repeater {
                        model: Shortcuts.all
                        RowLayout {
                            id: keyRow
                            required property var modelData
                            Layout.fillWidth: true
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Plain { text: keyRow.modelData.title; Layout.fillWidth: true }
                                Hint { visible: text !== ""; text: cap.problem; color: "#e5484d" }
                            }
                            KeyCapture { id: cap; action: keyRow.modelData.key }
                            Button {
                                objectName: "key_" + keyRow.modelData.key
                                Layout.preferredWidth: 160
                                text: cap.on ? qsTr("Tuşlara bas…") : Shortcuts.label(keyRow.modelData.key)
                                highlighted: cap.on
                                onClicked: cap.start()
                            }
                            ToolButton {
                                objectName: "reset_" + keyRow.modelData.key
                                icon.name: Util.icon("edit-undo")
                                enabled: !!Player.options[keyRow.modelData.key]
                                onClicked: Player.setOption(keyRow.modelData.key, "")
                                ToolTip.visible: hovered; ToolTip.delay: 500; ToolTip.text: qsTr("Standart")
                            }
                        }
                    }
                }

                Page {
                    name: "Scrobble"
                    Hint { text: qsTr("Dinlediklerin Navidrome'a (ve sunucuda bağlıysa Last.fm / ListenBrainz'e) gider.") }
                    SwitchRow {
                        label: qsTr("Dinlenenleri gönder")
                        hint: qsTr("Şarkının yarısı (en fazla 4 dakikası) çalınca bir dinleme sayılır.")
                        checked: Player.options.scrobble === true
                        onToggled: on => Player.setOption("scrobble", on)
                    }
                    SwitchRow {
                        label: qsTr("Şimdi çalıyor bilgisini gönder")
                        hint: qsTr("Şarkı başlayınca sunucuya ne çaldığını bildirir.")
                        checked: Player.options.nowPlaying === true
                        onToggled: on => Player.setOption("nowPlaying", on)
                    }
                }

                Page {
                    name: "Hesap"
                    Hint { text: Player.user + " @ " + Player.setting("url"); elide: Text.ElideMiddle; wrapMode: Text.NoWrap }
                    Button { text: qsTr("Çıkış yap"); icon.name: "system-log-out"; onClicked: { d.close(); Player.logout() } }
                }

                Page {
                    name: "Hakkında"
                    Plain { text: "re-la"; font.weight: Font.DemiBold }
                    Hint { text: qsTr("Yapı tarihi: %1").arg(App.buildDate) }
                }
            }
        }
    }
}
