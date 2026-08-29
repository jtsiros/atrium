import QtQuick
import qs.Ui
import qs.Commons

Item {
  id: root

  property string label: ""
  property bool checked: false

  signal toggled()

  implicitHeight: Style.spacing.controlHeight

  Text {
    anchors.left: parent.left
    anchors.right: control.left
    anchors.rightMargin: Style.spacing.lg
    anchors.verticalCenter: parent.verticalCenter
    text: root.label
    textFormat: Text.PlainText
    font.family: Style.font.family
    font.pixelSize: Style.font.bodySmall
    color: Color.foreground
    elide: Text.ElideRight
  }

  ToggleSwitch {
    id: control
    anchors.right: parent.right
    anchors.verticalCenter: parent.verticalCenter
    checked: root.checked
    onToggled: root.toggled()
  }
}
