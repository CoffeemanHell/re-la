// The settings window: categories on the left, one page of switches and sliders per category.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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
    component Page: ColumnLayout {
        required property string name
        visible: d.cat === name
        Layout.fillWidth: true
        spacing: 10
    }

    objectName: "settings"
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
                                            "Scrobble": "view-statistics", "Hesap": "user-identity", "Hakkında": "help-about" })
            model: ["Genel", "Çalma", "DJ", "Scrobble", "Hesap", "Hakkında"]
            delegate: ItemDelegate {
                required property string modelData
                objectName: "cat_" + modelData
                width: ListView.view.width
                text: modelData
                icon.name: catList.icons[modelData]
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
                    PageTitle { text: d.cat; Layout.fillWidth: true }
                    ToolButton { icon.name: "window-close"; onClicked: d.close() }
                }

                Page {
                    name: "Genel"
                    RowLayout { // laid out like a SwitchRow, a combo box in place of the switch
                        Layout.fillWidth: true
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 2
                            Plain { text: "Tema" }
                            Hint { text: "Uygulamanın görünümü." }
                        }
                        ComboBox {
                            objectName: "themeBox"
                            model: App.themeNames.map(n => App.themeTitles[n] || n)
                            currentIndex: App.themeNames.indexOf(Player.theme)
                            onActivated: i => Player.theme = App.themeNames[i]
                        }
                    }
                    SwitchRow {
                        visible: App.trayAvailable
                        label: "Kapatınca arka planda çal"
                        hint: "Pencereyi kapatınca müzik durmaz, tepsi simgesinden kontrol edilir. Çıkış: simgeye sağ tık."
                        checked: Player.options.closeToTray === true
                        onToggled: on => Player.setOption("closeToTray", on)
                    }
                    LabeledSlider {
                        label: "Izgara boyutu"
                        valueText: (Player.options.gridSize || 172) + " px"
                        hint: "Albüm ve sanatçı kartlarının genişliği."
                        from: 120; to: 260; stepSize: 10; value: Player.options.gridSize || 172
                        onMoved: v => Player.setOption("gridSize", v)
                    }
                }

                Page {
                    name: "Çalma"
                    LabeledSlider {
                        label: "Şarkı geçişi (crossfade)"
                        valueText: Player.crossfade ? Player.crossfade + " sn" : "Kapalı · gapless"
                        hint: Player.exclusive ? "Exclusive modda kapalı: ses kartını tek deck tutabilir." : "0 = boşluksuz (gapless) ve bit-perfect geçiş."
                        from: 0; to: 12; stepSize: 1; value: Player.crossfade
                        enabledSlider: !Player.exclusive && Sync.following === ""
                        onMoved: v => Player.crossfade = v
                    }
                    SwitchRow {
                        visible: Qt.platform.os === "windows"
                        label: "WASAPI Exclusive"
                        hint: "Ses kartını uygulamaya ayırır, Windows mikseri atlanır: bit-perfect."
                        checked: Player.exclusive
                        onToggled: on => Player.exclusive = on
                    }
                    SwitchRow {
                        label: "Otomatik devam"
                        hint: "Kuyruk bitmeden benzer şarkılar eklenir. Nasıl seçileceğini DJ modu belirler."
                        checked: Player.options.autoContinue === true
                        onToggled: on => Player.setOption("autoContinue", on)
                    }
                }

                Page {
                    name: "DJ"
                    SwitchRow {
                        label: "DJ modu"
                        hint: "Kapalıyken Navidrome'un benzer şarkıları olduğu gibi eklenir. Açıkken aşağıdaki ağırlıklarla, geniş bir havuzdan seçilir."
                        checked: Player.dj
                        onToggled: on => Player.dj = on
                    }
                    LabeledSlider {
                        label: "Dil bağlılığı"
                        enabled: Player.dj
                        valueText: "%" + (Player.options.radioLanguage || 0)
                        hint: "Eklenen şarkıların ne kadarı çalan şarkıyla aynı dilde olsun. Türkçe bir şarkıdan başlarsan %80 → 10 şarkının 8'i Türkçe."
                        from: 0; to: 100; stepSize: 10; value: Player.options.radioLanguage || 0
                        onMoved: v => Player.setOption("radioLanguage", v)
                    }
                    LabeledSlider {
                        label: "Tür bağlılığı"
                        enabled: Player.dj
                        valueText: "%" + (Player.options.radioGenre || 0)
                        hint: "Aynı türe (genre) ne kadar yakın kalınsın."
                        from: 0; to: 100; stepSize: 10; value: Player.options.radioGenre || 0
                        onMoved: v => Player.setOption("radioGenre", v)
                    }
                    LabeledSlider {
                        label: "Her seferde eklenen"
                        enabled: Player.dj
                        valueText: (Player.options.radioBatch || 10) + " şarkı"
                        from: 5; to: 30; stepSize: 5; value: Player.options.radioBatch || 10
                        onMoved: v => Player.setOption("radioBatch", v)
                    }
                }

                Page {
                    name: "Scrobble"
                    Hint { text: "Dinlediklerin Navidrome'a (ve sunucuda bağlıysa Last.fm / ListenBrainz'e) gider." }
                    SwitchRow {
                        label: "Dinlenenleri gönder"
                        hint: "Şarkının yarısı (en fazla 4 dakikası) çalınca bir dinleme sayılır."
                        checked: Player.options.scrobble === true
                        onToggled: on => Player.setOption("scrobble", on)
                    }
                    SwitchRow {
                        label: "Şimdi çalıyor bilgisini gönder"
                        hint: "Şarkı başlayınca sunucuya ne çaldığını bildirir."
                        checked: Player.options.nowPlaying === true
                        onToggled: on => Player.setOption("nowPlaying", on)
                    }
                }

                Page {
                    name: "Hesap"
                    Hint { text: Player.user + " @ " + Player.setting("url"); elide: Text.ElideMiddle; wrapMode: Text.NoWrap }
                    Button { text: "Çıkış yap"; icon.name: "system-log-out"; onClicked: { d.close(); Player.logout() } }
                }

                Page {
                    name: "Hakkında"
                    Plain { text: "re-la"; font.weight: Font.DemiBold }
                    Hint { text: "Yapı tarihi: " + App.buildDate }
                }
            }
        }
    }
}
