import QtQuick
import qs.Commons

// A fixed-width slot. Nerd Font glyphs do not all paint the same width, so a
// bare Text shifts whatever follows it row by row.
Item {
  id: root

  property string glyph: ""
  property color color: Color.muted
  property real size: Style.font.icon
  property real slot: size
  property int align: Text.AlignHCenter

  implicitWidth: slot
  implicitHeight: slot

  Text {
    anchors.fill: parent
    text: root.glyph
    textFormat: Text.PlainText
    font.family: Style.font.family
    font.pixelSize: root.size
    color: root.color
    horizontalAlignment: root.align
    verticalAlignment: Text.AlignVCenter
  }
}
