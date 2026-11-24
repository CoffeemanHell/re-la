// Share link for a song or album (createShare), copied to the clipboard.
import QtQuick
import QtQuick.Controls
import Rela

Button {
    id: b
    property string itemId
    property string label: "Paylaş"
    readonly property var w: ApplicationWindow.window
    text: label
    flat: true
    icon.name: "emblem-shared"
    enabled: itemId !== ""
    onClicked: Player.set(b, "createShare", { id: itemId }, r => {
        const s = r && r.shares && r.shares.share
        const url = s && s.length ? s[0].url : ""
        if (!url) return w.notify("İşlem başarısız")
        Player.copy(url)
        w.notify("Link kopyalandı")
    })
}
