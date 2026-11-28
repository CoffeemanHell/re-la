// The settings window: categories on the left, one page of switches and dot bars per category.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import shared
import Rela

Dialog {
    id: d
    property var w // the window
    property string cat: "Genel"
    function show(c) { cat = c || "Genel"; open() }
    // [key, shown]: the key stays Turkish (cat, objectName, openSettings)
    readonly property var cats: [["Genel", qsTr("GENEL")], ["Çalma", qsTr("ÇALMA")], ["DJ", "DJ"], ["Seçim", qsTr("SEÇİM")], ["Atlamalar", qsTr("ATLAMALAR")],
        ["Hafıza", qsTr("HAFIZA")], ["İnce ayar", qsTr("İNCE AYAR")], ["Önbellek", qsTr("ÖNBELLEK")], ["Klavye", qsTr("KLAVYE")], ["Scrobble", "SCROBBLE"],
        ["Hesap", qsTr("HESAP")], ["Hakkında", qsTr("HAKKINDA")]]
    readonly property var djSubs: ["Seçim", "Atlamalar", "Hafıza", "İnce ayar"] // the DJ's pages, listed under DJ

    component Setting: ColumnLayout {
        id: st
        property string label
        property string valueText
        property string hint
        property real value: 0   // 0..1
        property int dots: 20
        signal picked(real v)
        Layout.fillWidth: true
        spacing: 6
        RowLayout {
            Mono { text: st.label; Layout.fillWidth: true }
            Mono { text: st.valueText; color: App.theme.accent }
        }
        DotBar { Layout.fillWidth: true; dots: st.dots; value: st.value; live: true; enabled: st.enabled; onPicked: v => st.picked(v) }
        Hint { text: st.hint }
    }
    // One DJ setting: its name and value (accent when changed) with a reset when changed, then the control its kind asks
    // for. The rows under a switch hide while it is off.
    component DjKnob: ColumnLayout {
        id: kn
        required property var modelData
        readonly property var k: modelData
        readonly property var stored: Player.options[k.key]
        readonly property bool atDefault: DjKnobs.isDefault(k, stored)
        readonly property var cur: stored ?? k.def
        readonly property var choices: k.kind === "preset" ? k.presets : k.kind === "seg" ? Array.from({ length: (k.to - k.from) / k.step + 1 }, (_, i) => k.from + i * k.step) : []
        objectName: "knob_" + k.key
        visible: !k.under || Player.options[k.under] !== false
        enabled: Player.dj
        Layout.fillWidth: true
        spacing: 8
        signal moved(var v)
        onMoved: v => Player.setOption(k.key, v)
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Mono { text: DjKnobs.upper(kn.k.label); Layout.fillWidth: true }
            Mono {
                visible: kn.k.kind !== "switch"
                text: DjKnobs.upper(DjKnobs.text(kn.k, kn.cur))
                color: kn.atDefault ? App.theme.muted : App.theme.accent
            }
            Pill {
                objectName: "reset_" + kn.k.key
                visible: !kn.atDefault
                text: qsTr("VARSAYILAN")
                onClicked: Player.setOption(kn.k.key, kn.k.def)
            }
            Pill { visible: kn.k.kind === "switch"; text: kn.cur !== false ? qsTr("AÇIK") : qsTr("KAPALI"); on: kn.cur !== false; onClicked: kn.moved(kn.cur === false) }
        }
        DotBar {
            visible: kn.k.kind === "slider"
            Layout.fillWidth: true
            dots: (kn.k.to - kn.k.from) / kn.k.step + 1
            value: (kn.cur - kn.k.from) / (kn.k.to - kn.k.from)
            live: true
            onPicked: v => kn.moved(kn.k.from + Math.round(v * (kn.k.to - kn.k.from) / kn.k.step) * kn.k.step)
        }
        Flow { // a choice per pill: segments and the memory presets
            visible: kn.k.kind === "seg" || kn.k.kind === "preset"
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: kn.choices
                Pill {
                    required property var modelData
                    text: DjKnobs.upper(DjKnobs.text(kn.k, modelData))
                    on: kn.cur === modelData
                    onClicked: kn.moved(modelData)
                }
            }
        }
        RowLayout { // a number box: minus and plus
            visible: kn.k.kind === "spin"
            spacing: 12
            Pill { text: "−"; enabled: kn.cur > kn.k.from; onClicked: kn.moved(Math.max(kn.k.from, kn.cur - kn.k.step)) }
            Pill { text: "+"; enabled: kn.cur < kn.k.to; onClicked: kn.moved(Math.min(kn.k.to, kn.cur + kn.k.step)) }
        }
        Hint { text: DjKnobs.upper(kn.k.hint) }
    }
    component Page: ColumnLayout {
        required property string name
        visible: d.cat === name
        Layout.fillWidth: true
        spacing: 16
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
    background: Rectangle { color: App.theme.bg; border.width: 1; border.color: App.theme.line }
    Overlay.modal: Rectangle { color: "#cc000000" }
    contentItem: RowLayout {
        spacing: 0
        ColumnLayout {
            Layout.preferredWidth: 190; Layout.fillHeight: true
            Layout.margins: 24
            spacing: 8
            Dots { text: qsTr("AYARLAR"); font.pixelSize: 24; Layout.bottomMargin: 12 }
            Repeater {
                model: d.cats
                Pill {
                    required property var modelData
                    objectName: "cat_" + modelData[0]
                    Layout.leftMargin: d.djSubs.indexOf(modelData[0]) >= 0 ? 16 : 0
                    text: modelData[1]
                    on: d.cat === modelData[0]
                    onClicked: d.cat = modelData[0]
                }
            }
            Item { Layout.fillHeight: true }
        }
        Rectangle { Layout.fillHeight: true; implicitWidth: 1; color: App.theme.line }
        Flickable {
            id: settingsFlick
            Layout.fillWidth: true; Layout.fillHeight: true
            contentHeight: sc.implicitHeight + 64
            clip: true
            ScrollBar.vertical: Scroll {}
            ColumnLayout {
                id: sc
                x: 32; y: 24; width: settingsFlick.width - 64
                spacing: 16
                RowLayout {
                    Mono { text: (d.cats.find(c => c[0] === d.cat) || ["", ""])[1]; color: App.theme.muted; Layout.fillWidth: true }
                    Pill { text: "×"; onClicked: d.close() }
                }

                Page {
                    name: "Genel"
                    Mono { text: qsTr("TEMA"); color: App.theme.muted }
                    Flow {
                        Layout.fillWidth: true; spacing: 8
                        Repeater {
                            model: App.themeNames
                            Pill { required property string modelData; text: d.w.up(App.themeTitles[modelData] || modelData); on: Player.theme === modelData; onClicked: Player.theme = modelData }
                        }
                    }
                    SwitchRow {
                        visible: App.trayAvailable
                        label: qsTr("SİSTEM TEPSİSİ")
                        hint: qsTr("TEPSİDE SİMGE DURUR; PENCEREYİ KAPATINCA MÜZİK DURMAZ. ÇIKIŞ: SİMGEYE SAĞ TIK. KAPALIYKEN PENCEREYİ KAPATMAK UYGULAMADAN ÇIKAR.")
                        checked: Player.options.closeToTray === true
                        onToggled: on => Player.setOption("closeToTray", on)
                    }
                    Setting {
                        label: qsTr("IZGARA BOYUTU")
                        valueText: (Player.options.gridSize || 172) + " PX"
                        hint: qsTr("ALBÜM VE SANATÇI KARTLARININ GENİŞLİĞİ.")
                        dots: 15; value: ((Player.options.gridSize || 172) - 120) / 140
                        onPicked: v => Player.setOption("gridSize", 120 + Math.round(v * 14) * 10)
                    }
                }

                Page {
                    name: "Çalma"
                    Mono { text: qsTr("SES ÇIKIŞI"); color: App.theme.muted }
                    Flow {
                        id: outputs
                        Layout.fillWidth: true; spacing: 8
                        Repeater {
                            model: Util.outputs()
                            Pill {
                                required property var modelData
                                objectName: "output_" + modelData.name
                                width: Math.min(implicitWidth, outputs.width)
                                text: modelData.name === "auto" ? qsTr("SİSTEM VARSAYILANI") : d.w.up(modelData.description)
                                on: (Player.options.audioDevice || "auto") === modelData.name
                                onClicked: Player.setOption("audioDevice", modelData.name)
                            }
                        }
                    }
                    Setting {
                        label: qsTr("ŞARKI GEÇİŞİ (CROSSFADE)")
                        valueText: Player.crossfade ? qsTr("%1 SN").arg(Player.crossfade) : qsTr("KAPALI · GAPLESS")
                        hint: Player.exclusive ? qsTr("EXCLUSIVE MODDA KAPALI: SES KARTINI TEK DECK TUTABİLİR.") : qsTr("0 = BOŞLUKSUZ (GAPLESS) VE BIT-PERFECT GEÇİŞ.")
                        dots: 13; value: Player.crossfade / 12
                        enabled: !Player.exclusive && Sync.following === ""
                        onPicked: v => Player.crossfade = Math.round(v * 12)
                    }
                    SwitchRow {
                        visible: Qt.platform.os === "windows"
                        label: "WASAPI EXCLUSIVE"
                        hint: qsTr("SES KARTINI UYGULAMAYA AYIRIR, WINDOWS MİKSERİ ATLANIR: BIT-PERFECT.")
                        checked: Player.exclusive; danger: true
                        onToggled: on => Player.exclusive = on
                    }
                    SwitchRow {
                        label: qsTr("OTOMATİK DEVAM")
                        hint: qsTr("KUYRUK BİTMEDEN BENZER ŞARKILAR EKLENİR. NASIL SEÇİLECEĞİNİ DJ MODU BELİRLER.")
                        checked: Player.options.autoContinue === true
                        onToggled: on => Player.setOption("autoContinue", on)
                    }
                }

                Page {
                    name: "DJ"
                    onVisibleChanged: if (visible) d.dj.refreshFeedback()
                    SwitchRow {
                        label: qsTr("DJ MODU")
                        hint: qsTr("KAPALIYKEN NAVIDROME'UN BENZER ŞARKILARI OLDUĞU GİBİ EKLENİR. AÇIKKEN AŞAĞIDAKİ AĞIRLIKLARLA, GENİŞ BİR HAVUZDAN SEÇİLİR.")
                        checked: Player.dj
                        onToggled: on => Player.dj = on
                    }
                    Hint { text: qsTr("DJ AYARLARINI İLK HÂLİNE GETİRİR.") }
                    RowLayout {
                        Item { Layout.fillWidth: true }
                        Pill { text: qsTr("VARSAYILANLARA DÖN"); onClicked: Player.resetDjSettings() }
                    }
                    Mono { text: qsTr("GERİ BİLDİRİMLERİN"); Layout.topMargin: 8 }
                    Hint { visible: !d.dj.feedback.length; text: qsTr("HENÜZ YOK. BİR ŞARKININ MENÜSÜNDEN \"BUNUN GİBİ DAHA ÇOK\" YA DA \"BUNU DAHA AZ ÇAL\" DİYEBİLİRSİN.") }
                    Repeater {
                        model: d.dj.feedback
                        delegate: RowLayout {
                            id: fb
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 12
                            Mono { text: fb.modelData.dir > 0 ? "▲" : "▼"; color: fb.modelData.dir > 0 ? App.theme.accent : App.theme.muted }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Mono { text: (fb.modelData.title || "").toUpperCase(); elide: Text.ElideRight; Layout.fillWidth: true }
                                Hint {
                                    readonly property string dir: fb.modelData.dir > 0 ? qsTr("DAHA ÇOK") : qsTr("DAHA AZ")
                                    text: fb.modelData.artist ? qsTr("%1 · %2").arg(dir).arg(fb.modelData.artist.toUpperCase()) : dir
                                }
                            }
                            Pill { objectName: "fbRemove_" + fb.modelData.id; text: qsTr("KALDIR"); onClicked: d.dj.toggle(fb.modelData, fb.modelData.dir) }
                        }
                    }
                    Hint { text: qsTr("DJ'NİN SON SEÇİMDE TARTTIĞI ŞARKILAR VE PUANLARI."); Layout.topMargin: 8 }
                    RowLayout {
                        Item { Layout.fillWidth: true }
                        Pill { text: qsTr("DJ DURUMU"); onClicked: stateDialog.show() }
                    }
                    Hint { text: qsTr("ATLAMALARI, DİNLEMELERİ VE GERİ BİLDİRİMLERİNİ SİLER. KÜTÜPHANE DİZİNİ KALIR.") }
                    RowLayout {
                        Item { Layout.fillWidth: true }
                        Pill { text: qsTr("DJ HAFIZASINI SIFIRLA"); onClicked: { Player.resetDjMemory(); d.dj.feedback = [] } }
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
                    Hint { text: qsTr("DİNLEDİĞİN VE SIRADAKİ ŞARKILAR DİSKE İNER. SUNUCUYA ULAŞILAMAYINCA UYGULAMA SON KUYRUKLA AÇILIR, İNEN ŞARKILAR ÇALAR. KENDİN ATLADIĞIN BİR ŞARKI BİR KEZ ÇALARKEN, BİR KEZ DE ÖNBELLEK İÇİN İNER.") }
                    Mono { text: qsTr("BOYUT SINIRI"); color: App.theme.muted }
                    Flow {
                        Layout.fillWidth: true; spacing: 8
                        Repeater {
                            model: Util.cacheSizes
                            Pill {
                                required property int modelData
                                objectName: "cache_" + modelData
                                text: modelData ? modelData + " GB" : qsTr("KAPALI")
                                on: (Player.options.offlineGb || 0) === modelData
                                onClicked: Player.setOption("offlineGb", modelData)
                            }
                        }
                    }
                    Hint { text: qsTr("DOLUNCA EN UZUN SÜREDİR DİNLENMEYEN SİLİNİR.") }
                    Setting {
                        enabled: Player.options.offlineGb > 0
                        label: qsTr("ÖNDEN İNEN")
                        valueText: qsTr("%n ŞARKI", "", Player.options.offlineAhead || 0)
                        hint: qsTr("KUYRUKTA SIRADAKİ KAÇ ŞARKI DİSKTE HAZIR OLSUN.")
                        dots: 21; value: (Player.options.offlineAhead || 0) / 20
                        onPicked: v => Player.setOption("offlineAhead", Math.round(v * 20))
                    }
                    RowLayout {
                        Mono { text: qsTr("DİSKTE: %1").arg(d.w.up(Util.dataSize(Player.cacheBytes))); Layout.fillWidth: true }
                        Pill { text: qsTr("TEMİZLE"); enabled: Player.cacheBytes > 0; onClicked: Player.clearSongCache() }
                    }
                }

                Page {
                    name: "Klavye"
                    Hint { text: qsTr("DEĞİŞTİRMEK İÇİN KISAYOLA BAS, SONRA YENİ TUŞLARA. VAZGEÇMEK: ESC.") }
                    Repeater {
                        model: Shortcuts.all
                        RowLayout {
                            id: keyRow
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 8
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 4
                                Mono { text: keyRow.modelData.title.toLocaleUpperCase(Qt.locale(Qt.uiLanguage)); Layout.fillWidth: true }
                                Hint { visible: text !== ""; text: cap.problem.toLocaleUpperCase(Qt.locale(Qt.uiLanguage)); color: App.theme.accent }
                            }
                            KeyCapture { id: cap; action: keyRow.modelData.key }
                            Pill {
                                objectName: "key_" + keyRow.modelData.key
                                text: cap.on ? qsTr("TUŞLARA BAS…") : Shortcuts.label(keyRow.modelData.key)
                                on: cap.on
                                onClicked: cap.start()
                            }
                            Pill {
                                objectName: "reset_" + keyRow.modelData.key
                                text: qsTr("STANDART")
                                enabled: !!Player.options[keyRow.modelData.key]
                                onClicked: Player.setOption(keyRow.modelData.key, "")
                            }
                        }
                    }
                }

                Page {
                    name: "Scrobble"
                    Hint { text: qsTr("DİNLEDİKLERİN NAVIDROME'A (VE SUNUCUDA BAĞLIYSA LAST.FM / LISTENBRAINZ'E) GİDER.") }
                    SwitchRow {
                        label: qsTr("DİNLENENLERİ GÖNDER")
                        hint: qsTr("ŞARKININ YARISI (EN FAZLA 4 DAKİKASI) ÇALINCA BİR DİNLEME SAYILIR.")
                        checked: Player.options.scrobble === true
                        onToggled: on => Player.setOption("scrobble", on)
                    }
                    SwitchRow {
                        label: qsTr("ŞİMDİ ÇALIYOR BİLGİSİNİ GÖNDER")
                        hint: qsTr("ŞARKI BAŞLAYINCA SUNUCUYA NE ÇALDIĞINI BİLDİRİR.")
                        checked: Player.options.nowPlaying === true
                        onToggled: on => Player.setOption("nowPlaying", on)
                    }
                }

                Page {
                    name: "Hesap"
                    Mono { text: d.w.up(Player.user) + " @ " + Player.setting("url"); color: App.theme.muted; Layout.fillWidth: true; elide: Text.ElideMiddle }
                    Pill { text: qsTr("ÇIKIŞ YAP"); onClicked: { d.close(); Player.logout() } }
                }

                Page {
                    name: "Hakkında"
                    Dots { text: "NSYNC"; font.pixelSize: 24 }
                    Mono { text: qsTr("YAPI TARİHİ: %1").arg(App.buildDate); color: App.theme.muted }
                }
            }
        }
    }
}
