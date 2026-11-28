// Signing in to the Navidrome server; with a saved login only the connection status (with retries) and a way out.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Rela

ColumnLayout {
    width: Kirigami.Units.gridUnit * 22
    spacing: Kirigami.Units.largeSpacing
    Kirigami.Heading { text: "re-la"; level: 1 }
    Plain { text: qsTr("Navidrome üzerinden birlikte, kayıpsız dinle."); color: Kirigami.Theme.disabledTextColor; Layout.bottomMargin: Kirigami.Units.largeSpacing }
    Kirigami.FormLayout {
        Layout.fillWidth: true
        visible: !Player.signedIn
        TextField { id: url; objectName: "loginUrl"; Kirigami.FormData.label: qsTr("Sunucu:"); Layout.fillWidth: true; placeholderText: "https://muzik.ornek.com"; text: Player.setting("url") }
        TextField { id: user; objectName: "loginUser"; Kirigami.FormData.label: qsTr("Kullanıcı adı:"); Layout.fillWidth: true; text: Player.setting("user") }
        Kirigami.PasswordField { id: pass; objectName: "loginPass"; Kirigami.FormData.label: qsTr("Şifre:"); Layout.fillWidth: true; onAccepted: connectBtn.clicked() }
    }
    BusyIndicator { Layout.alignment: Qt.AlignHCenter; running: Player.connecting; visible: running }
    Button {
        id: connectBtn
        Layout.fillWidth: true
        highlighted: !Player.signedIn // while connecting this is "Vazgeç": a hanging connect can be abandoned
        text: Player.signedIn ? qsTr("Vazgeç") : qsTr("Bağlan")
        onClicked: Player.signedIn ? Player.logout() : Player.login(url.text, user.text, pass.text)
    }
    Plain {
        text: Player.status
        color: Player.loginFailed ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.disabledTextColor
        wrapMode: Text.WordWrap; Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter
    }
}
