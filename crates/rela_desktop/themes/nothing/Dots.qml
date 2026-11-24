// Dot-matrix display type for big text.
import QtQuick
import Rela

Text {
    textFormat: Text.PlainText
    color: App.theme.fg
    font.family: App.theme.displayFont
    font.weight: Font.Bold // Doto is variable: Bold = round dots that still read as separate; Black merges them
}
