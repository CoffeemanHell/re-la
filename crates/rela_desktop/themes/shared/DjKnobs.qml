// The DJ's settings as the settings windows show them: option key, label, hint, range and unit. The core
// (rela_core::dj::knobs) reads the same keys and keeps the defaults; `main` comes first, `advanced` under a fold.
pragma Singleton
import QtQuick

QtObject {
    // unit: "%", "days", "s", "min", "songs" or "" (a plain number)
    readonly property var main: [
        { key: "radioLanguage", label: qsTr("Dil bağlılığı"), unit: "%", from: 0, to: 100, step: 10,
          hint: qsTr("Eklenen şarkıların ne kadarı çalan şarkıyla aynı dilde olsun. Türkçe bir şarkıdan başlarsan %80 → 10 şarkının 8'i Türkçe.") },
        { key: "radioGenre", label: qsTr("Benzerlik gücü"), unit: "%", from: 0, to: 100, step: 10,
          hint: qsTr("Çalan müziğe ne kadar yakın kalınsın. Yakın türler (Heavy Metal ~ Hard Rock) de sayılır.") },
        { key: "radioExplore", label: qsTr("Keşif"), unit: "%", from: 0, to: 50, step: 10,
          hint: qsTr("Her partide yeni eklenen ya da az dinlenen şarkılara ayrılan pay.") },
        { key: "radioBatch", label: qsTr("Her seferde eklenen"), unit: "songs", from: 5, to: 30, step: 5, hint: "" },
        { key: "djSkipShare", label: qsTr("Atlama eksi sayılır"), unit: "%", from: 10, to: 60, step: 5,
          hint: qsTr("Şarkının bu kadarı çalmadan atlarsan şarkıya eksi yazılır.") },
        { key: "djDislikeSkips", label: qsTr("Sevilmeyen şarkı: eksi sayısı"), unit: "", from: 1, to: 10, step: 1,
          hint: qsTr("Bu kadar eksiden sonra şarkı sevilmiyor sayılır ve çok seyrek çalar.") },
        { key: "djDislikeDays", label: qsTr("Sevilmeyen şarkı: farklı gün"), unit: "days", from: 1, to: 5, step: 1,
          hint: qsTr("Eksiler en az bu kadar farklı günde gelmiş olmalı: kötü bir akşam yetmez.") },
        { key: "djMemoryDays", label: qsTr("Hafıza süresi"), unit: "days", from: 7, to: 364, step: 7,
          hint: qsTr("Bir eksinin yarıya inmesi için geçen süre.") },
        { key: "djRepeatDays", label: qsTr("Tekrar etmeme"), unit: "days", from: 0, to: 30, step: 1,
          hint: qsTr("Bu kadar gün içinde dinlediğin şarkılar geri planda kalır. 0: kapalı.") },
        { key: "djArtistMax", label: qsTr("Aynı sanatçıdan en fazla"), unit: "songs", from: 1, to: 5, step: 1,
          hint: qsTr("Son 10 şarkı ve sıradakiler içinde.") },
        { key: "djWildcard", label: qsTr("Sürpriz şarkı"), unit: "%", from: 0, to: 30, step: 5,
          hint: qsTr("Her partide puana bakılmadan seçilen şarkıların payı.") }
    ]
    readonly property var advanced: [
        { key: "djSkipMaxSec", label: qsTr("Eksi atlama süresi"), unit: "s", from: 30, to: 600, step: 30,
          hint: qsTr("Bundan sonraki atlamalar eksi sayılmaz.") },
        { key: "djFullShare", label: qsTr("Tam dinlenmiş sayılır"), unit: "%", from: 60, to: 100, step: 5,
          hint: qsTr("Şarkının bu kadarı çaldıysa sonuna kadar dinlenmiş sayılır.") },
        { key: "djSearchSkips", label: qsTr("Arayış: atlama sayısı"), unit: "", from: 2, to: 5, step: 1,
          hint: qsTr("Bu kadar hızlı atlama \"başka bir şey arıyorum\" demek: bu atlamalar eksi sayılmaz, DJ daha çok keşfeder.") },
        { key: "djSearchSec", label: qsTr("Arayış: süre"), unit: "s", from: 30, to: 300, step: 10,
          hint: qsTr("Atlamalar bu süre içinde gelmeli. Tek bir atlama da bu kadar bekler, sonra eksi sayılır.") },
        { key: "djIdleMin", label: qsTr("Oturum sonu"), unit: "min", from: 5, to: 240, step: 5,
          hint: qsTr("Bu kadar dinlemezsen DJ oturumun zevkini unutur.") },
        { key: "djSessionFade", label: qsTr("Oturum zevkinin kalıcılığı"), unit: "%", from: 30, to: 95, step: 5,
          hint: qsTr("Her şarkıdan sonra oturum zevkinin ne kadarı kalır.") },
        { key: "djNewDays", label: qsTr("Yeni sayılma süresi"), unit: "days", from: 0, to: 180, step: 5,
          hint: qsTr("Kütüphaneye bu kadar gün önce eklenen şarkılar yeni sayılır.") },
        { key: "djSimilar", label: qsTr("Benzer şarkı bonusu"), unit: "", from: 0, to: 200, step: 10,
          hint: qsTr("Navidrome'un benzer dediği şarkılara eklenen puan (×100).") },
        { key: "djTaste", label: qsTr("Uzun zevkin ağırlığı"), unit: "", from: 0, to: 200, step: 10,
          hint: qsTr("Çok dinlediğin sanatçı ve türlerin puandaki payı (×100).") },
        { key: "djRandomness", label: qsTr("Seçim rastgeleliği"), unit: "", from: 10, to: 200, step: 10,
          hint: qsTr("Düşük: hep en yüksek puanlılar. Yüksek: puanlar birbirine yaklaşır (×100).") }
    ]
    // capitals for themes that write in them: Turkish i is İ (the UI is English only off a Turkish system)
    function upper(s) {
        s = s || ""
        return Qt.uiLanguage && !Qt.uiLanguage.startsWith("tr") ? s.toUpperCase() : s.replace(/i/g, "İ").toUpperCase()
    }
    // a knob's value as the settings show it
    function text(k, v) {
        if (k.unit === "%") return qsTr("%%1").arg(v) // Turkish puts the sign first, English after
        if (k.unit === "days") return qsTr("%n gün", "", v)
        if (k.unit === "s") return qsTr("%n sn", "", v)
        if (k.unit === "min") return qsTr("%n dk", "", v)
        if (k.unit === "songs") return qsTr("%n şarkı", "", v)
        return String(v)
    }
}
