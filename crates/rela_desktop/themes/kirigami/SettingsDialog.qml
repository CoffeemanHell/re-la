// The settings window: categories on the left, one page of switches and sliders per category.
pragma ComponentBehavior: Bound
import QtQuick
import org.kde.kirigami as Kirigami
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
            Plain { text: ls.valueText; color: ls.enabled ? Kirigami.Theme.highlightColor : Kirigami.Theme.textColor; font.weight: Font.DemiBold } // disabled: as faint as the label, not fainter
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
    // One DJ setting: its name and value (accent when changed) with a one-value reset, then the control its kind asks
    // for. The rows under a switch hide while it is off.
    component DjKnob: ColumnLayout {
        id: kn
        required property var modelData
        readonly property var k: modelData
        readonly property var stored: Player.options[k.key]
        readonly property bool atDefault: DjKnobs.isDefault(k, stored)
        readonly property var cur: stored ?? k.def
        readonly property var steps: k.kind === "seg" ? Array.from({ length: (k.to - k.from) / k.step + 1 }, (_, i) => k.from + i * k.step) : []
        objectName: "knob_" + k.key
        visible: !k.under || Player.options[k.under] !== false
        enabled: Player.dj
        Layout.fillWidth: true
        spacing: 4
        signal moved(var v)
        onMoved: v => Player.setOption(k.key, v)
        RowLayout {
            Layout.fillWidth: true
            Plain { text: kn.k.label; Layout.fillWidth: true }
            Plain {
                visible: kn.k.kind !== "switch" && kn.k.kind !== "spin" // a number box shows its own value
                text: DjKnobs.text(kn.k, kn.cur)
                color: kn.atDefault ? Kirigami.Theme.disabledTextColor : Kirigami.Theme.highlightColor
                font.weight: Font.DemiBold
                font.features: { "tnum": 1 }
            }
            ToolButton {
                objectName: "reset_" + kn.k.key
                visible: !kn.atDefault
                text: "↺" // a glyph: some icon themes have no edit-undo
                ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay; ToolTip.text: qsTr("Varsayılana dön")
                onClicked: Player.setOption(kn.k.key, kn.k.def)
            }
            Switch { visible: kn.k.kind === "switch"; checked: kn.cur !== false; onToggled: kn.moved(checked) }
        }
        Slider {
            visible: kn.k.kind === "slider"
            Layout.fillWidth: true
            from: kn.k.from; to: kn.k.to; stepSize: kn.k.step; snapMode: Slider.SnapAlways
            value: kn.cur
            onMoved: kn.moved(Math.round(value))
        }
        Flow { // a choice per button: segments and the memory presets
            visible: kn.k.kind === "seg" || kn.k.kind === "preset"
            Layout.fillWidth: true
            spacing: 6
            ButtonGroup { id: choices; exclusive: true }
            Repeater {
                model: kn.k.kind === "preset" ? kn.k.presets : kn.steps
                Button {
                    required property var modelData
                    text: DjKnobs.text(kn.k, modelData)
                    checkable: true
                    checked: kn.cur === modelData
                    ButtonGroup.group: choices
                    onClicked: kn.moved(modelData)
                }
            }
        }
        SpinBox {
            objectName: "spin_" + kn.k.key
            visible: kn.k.kind === "spin"
            Layout.alignment: Qt.AlignLeft
            from: kn.k.from; to: kn.k.to; stepSize: kn.k.step
            editable: true
            value: kn.cur
            textFromValue: v => DjKnobs.text(kn.k, v)
            validator: RegularExpressionValidator { regularExpression: /\d{0,4}\D*/ } // the number with the unit it shows; no digit while retyping it
            valueFromText: t => parseInt(t) || kn.k.from
            onValueModified: kn.moved(value)
        }
        Hint { text: kn.k.hint }
    }

    property alias stateDialog: stateDialog
    readonly property DjData dj: DjData { parent: d.contentItem } // a parent: Player.get answers only such owners
    DjStateDialog { id: stateDialog; w: d.w }

    objectName: "settings"
    onAboutToShow: { Player.refreshAudioDevices(); d.dj.refreshFeedback() } // not opened: that waits for the style's opening animation
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(960, d.w.width - 40); height: Math.min(680, d.w.height - 40)
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
            // the DJ's four pages sit under "DJ", indented and without an icon
            readonly property var subs: ["Seçim", "Atlamalar", "Hafıza", "İnce ayar"]
            // the keys stay Turkish (d.cat, objectName, openSettings); qsTr shows them
            model: [QT_TR_NOOP("Genel"), QT_TR_NOOP("Çalma"), "DJ", QT_TR_NOOP("Seçim"), QT_TR_NOOP("Atlamalar"), QT_TR_NOOP("Hafıza"), QT_TR_NOOP("İnce ayar"),
                    QT_TR_NOOP("Önbellek"), QT_TR_NOOP("Klavye"), "Scrobble", QT_TR_NOOP("Hesap"), QT_TR_NOOP("Hakkında")]
            delegate: ItemDelegate {
                required property string modelData
                readonly property bool sub: catList.subs.indexOf(modelData) >= 0
                objectName: "cat_" + modelData
                width: ListView.view.width
                text: qsTr(modelData)
                icon.name: catList.icons[modelData] ? Util.icon(catList.icons[modelData]) : ""
                leftPadding: sub ? 40 : 12
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
                            Plain { text: fb.modelData.dir > 0 ? "▲" : "▼"; color: fb.modelData.dir > 0 ? Kirigami.Theme.highlightColor : Kirigami.Theme.disabledTextColor }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 0
                                Plain { text: fb.modelData.title; elide: Text.ElideRight; Layout.fillWidth: true }
                                Hint {
                                    readonly property string dir: fb.modelData.dir > 0 ? qsTr("Daha çok") : qsTr("Daha az")
                                    text: fb.modelData.artist ? qsTr("%1 · %2").arg(dir).arg(fb.modelData.artist) : dir
                                }
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
                    name: "Seçim"
                    Repeater { model: DjKnobs.of("pick"); delegate: DjKnob {} }
                }

                Page {
                    name: "Atlamalar"
                    Repeater { model: DjKnobs.of("skips"); delegate: DjKnob {} }
                }

                Page {
                    name: "Hafıza"
                    Repeater { model: DjKnobs.of("memory"); delegate: DjKnob {} }
                }

                Page {
                    name: "İnce ayar"
                    Repeater { model: DjKnobs.of("fine"); delegate: DjKnob {} }
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
                                Hint { visible: text !== ""; text: cap.problem; color: Kirigami.Theme.negativeTextColor }
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
                                ToolTip.visible: hovered; ToolTip.delay: Kirigami.Units.toolTipDelay; ToolTip.text: qsTr("Standart")
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
