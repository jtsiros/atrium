import QtQuick
import qs.Ui
import qs.Commons

Item {
  id: root

  property QtObject bar: null
  property string label: ""
  property string readout: ""
  property real value: 0
  property real minimum: 0
  property real maximum: 1
  property real step: 0.05
  property bool integer: false

  signal released(real value)

  implicitHeight: Math.max(slider.implicitHeight, caption.implicitHeight)

  Text {
    id: caption
    anchors.left: parent.left
    anchors.verticalCenter: parent.verticalCenter
    width: Style.space(42)
    text: root.label
    textFormat: Text.PlainText
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    color: Color.muted
    elide: Text.ElideRight
  }

  PanelSlider {
    id: slider
    anchors.left: caption.right
    anchors.right: readoutText.left
    anchors.leftMargin: Style.spacing.controlGap
    anchors.rightMargin: Style.spacing.controlGap
    anchors.verticalCenter: parent.verticalCenter
    bar: root.bar
    value: root.value
    minimum: root.minimum
    maximum: root.maximum
    step: root.step
    integer: root.integer
    onReleased: function(next) { root.released(next) }
  }

  Text {
    id: readoutText
    anchors.right: parent.right
    anchors.verticalCenter: parent.verticalCenter
    width: Style.space(40)
    horizontalAlignment: Text.AlignRight
    text: root.readout
    textFormat: Text.PlainText
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    color: Color.foreground
  }
}
