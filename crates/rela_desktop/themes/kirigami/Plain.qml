// A label that shows its text as it is: names come from the server and from other listeners, and a bare Label would read
// a tag in one as HTML (an <img> there fetches from anywhere).
import QtQuick
import QtQuick.Controls

Label { textFormat: Text.PlainText }
