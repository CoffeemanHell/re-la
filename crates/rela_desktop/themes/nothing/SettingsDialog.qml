// The settings window: categories on the left, one page of switches and dot bars per category.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rela

Dialog {
    id: d
    property var w // the window
    property string cat: "Genel"
    function show(c) { cat = c || "Genel"; open() }
    readonly property var cats: [["Genel", "GENEL"], ["Çalma", "ÇALMA"], ["DJ", "DJ"], ["Scrobble", "SCROBBLE"], ["Hesap", "HESAP"], ["Hakkında", "HAKKINDA"]]

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
    component Page: ColumnLayout {
        required property string name
        visible: d.cat === name
        Layout.fillWidth: true
        spacing: 16
    }

    objectName: "settings"
    anchors.centerIn: Overlay.overlay
    modal: true
    width: Math.min(760, d.w.width - 40); height: Math.min(560, d.w.height - 40)
    padding: 0
    background: Rectangle { color: App.theme.bg; border.width: 1; border.color: App.theme.line }
    Overlay.modal: Rectangle { color: "#cc000000" }
    contentItem: RowLayout {
        spacing: 0
        ColumnLayout {
            Layout.preferredWidth: 190; Layout.fillHeight: true
            Layout.margins: 24
            spacing: 8
            Dots { text: "AYARLAR"; font.pixelSize: 24; Layout.bottomMargin: 12 }
            Repeater {
                model: d.cats
                Pill {
                    required property var modelData
                    objectName: "cat_" + modelData[0]
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
                    Mono { text: "TEMA"; color: App.theme.muted }
                    Flow {
                        Layout.fillWidth: true; spacing: 8
                        Repeater {
                            model: App.themeNames
                            Pill { required property string modelData; text: d.w.up(App.themeTitles[modelData] || modelData); on: Player.theme === modelData; onClicked: Player.theme = modelData }
                        }
                    }
                    SwitchRow {
                        visible: App.trayAvailable
                        label: "KAPATINCA ARKA PLANDA ÇAL"
                        hint: "PENCEREYİ KAPATINCA MÜZİK DURMAZ, TEPSİ SİMGESİNDEN KONTROL EDİLİR. ÇIKIŞ: SİMGEYE SAĞ TIK."
                        checked: Player.options.closeToTray === true
                        onToggled: on => Player.setOption("closeToTray", on)
                    }
                    Setting {
                        label: "IZGARA BOYUTU"
                        valueText: (Player.options.gridSize || 172) + " PX"
                        hint: "ALBÜM VE SANATÇI KARTLARININ GENİŞLİĞİ."
                        dots: 15; value: ((Player.options.gridSize || 172) - 120) / 140
                        onPicked: v => Player.setOption("gridSize", 120 + Math.round(v * 14) * 10)
                    }
                }

                Page {
                    name: "Çalma"
                    Setting {
                        label: "ŞARKI GEÇİŞİ (CROSSFADE)"
                        valueText: Player.crossfade ? Player.crossfade + " SN" : "KAPALI · GAPLESS"
                        hint: Player.exclusive ? "EXCLUSIVE MODDA KAPALI: SES KARTINI TEK DECK TUTABİLİR." : "0 = BOŞLUKSUZ (GAPLESS) VE BIT-PERFECT GEÇİŞ."
                        dots: 13; value: Player.crossfade / 12
                        enabled: !Player.exclusive && Sync.following === ""
                        onPicked: v => Player.crossfade = Math.round(v * 12)
                    }
                    SwitchRow {
                        visible: Qt.platform.os === "windows"
                        label: "WASAPI EXCLUSIVE"
                        hint: "SES KARTINI UYGULAMAYA AYIRIR, WINDOWS MİKSERİ ATLANIR: BIT-PERFECT."
                        checked: Player.exclusive; danger: true
                        onToggled: on => Player.exclusive = on
                    }
                    SwitchRow {
                        label: "OTOMATİK DEVAM"
                        hint: "KUYRUK BİTMEDEN BENZER ŞARKILAR EKLENİR. NASIL SEÇİLECEĞİNİ DJ MODU BELİRLER."
                        checked: Player.options.autoContinue === true
                        onToggled: on => Player.setOption("autoContinue", on)
                    }
                }

                Page {
                    name: "DJ"
                    SwitchRow {
                        label: "DJ MODU"
                        hint: "KAPALIYKEN NAVIDROME'UN BENZER ŞARKILARI OLDUĞU GİBİ EKLENİR. AÇIKKEN AŞAĞIDAKİ AĞIRLIKLARLA, GENİŞ BİR HAVUZDAN SEÇİLİR."
                        checked: Player.dj
                        onToggled: on => Player.dj = on
                    }
                    Setting {
                        enabled: Player.dj
                        label: "DİL BAĞLILIĞI"
                        valueText: "%" + (Player.options.radioLanguage || 0)
                        hint: "EKLENEN ŞARKILARIN NE KADARI ÇALAN ŞARKIYLA AYNI DİLDE OLSUN. TÜRKÇE BİR ŞARKIDAN BAŞLARSAN %80 → 10 ŞARKININ 8'İ TÜRKÇE."
                        dots: 11; value: (Player.options.radioLanguage || 0) / 100
                        onPicked: v => Player.setOption("radioLanguage", Math.round(v * 10) * 10)
                    }
                    Setting {
                        enabled: Player.dj
                        label: "TÜR BAĞLILIĞI"
                        valueText: "%" + (Player.options.radioGenre || 0)
                        hint: "AYNI TÜRE (GENRE) NE KADAR YAKIN KALINSIN."
                        dots: 11; value: (Player.options.radioGenre || 0) / 100
                        onPicked: v => Player.setOption("radioGenre", Math.round(v * 10) * 10)
                    }
                    Setting {
                        enabled: Player.dj
                        label: "HER SEFERDE EKLENEN"
                        valueText: (Player.options.radioBatch || 10) + " ŞARKI"
                        dots: 6; value: ((Player.options.radioBatch || 10) - 5) / 25
                        onPicked: v => Player.setOption("radioBatch", 5 + Math.round(v * 5) * 5)
                    }
                }

                Page {
                    name: "Scrobble"
                    Hint { text: "DİNLEDİKLERİN NAVIDROME'A (VE SUNUCUDA BAĞLIYSA LAST.FM / LISTENBRAINZ'E) GİDER." }
                    SwitchRow {
                        label: "DİNLENENLERİ GÖNDER"
                        hint: "ŞARKININ YARISI (EN FAZLA 4 DAKİKASI) ÇALINCA BİR DİNLEME SAYILIR."
                        checked: Player.options.scrobble === true
                        onToggled: on => Player.setOption("scrobble", on)
                    }
                    SwitchRow {
                        label: "ŞİMDİ ÇALIYOR BİLGİSİNİ GÖNDER"
                        hint: "ŞARKI BAŞLAYINCA SUNUCUYA NE ÇALDIĞINI BİLDİRİR."
                        checked: Player.options.nowPlaying === true
                        onToggled: on => Player.setOption("nowPlaying", on)
                    }
                }

                Page {
                    name: "Hesap"
                    Mono { text: d.w.up(Player.user) + " @ " + Player.setting("url"); color: App.theme.muted; Layout.fillWidth: true; elide: Text.ElideMiddle }
                    Pill { text: "ÇIKIŞ YAP"; onClicked: { d.close(); Player.logout() } }
                }

                Page {
                    name: "Hakkında"
                    Dots { text: "NSYNC"; font.pixelSize: 24 }
                    Mono { text: "YAPI TARİHİ: " + App.buildDate; color: App.theme.muted }
                }
            }
        }
    }
}
