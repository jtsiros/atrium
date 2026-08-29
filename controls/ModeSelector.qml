import QtQuick
import qs.Ui
import qs.Commons

Flow {
  id: root

  property var modes: []
  property string current: ""

  signal picked(string mode)

  spacing: Style.spacing.sm

  Repeater {
    model: root.modes
    delegate: Rectangle {
      required property var modelData

      readonly property bool selected: String(modelData) === root.current

      implicitWidth: caption.implicitWidth + Style.spacing.controlPaddingX * 2
      implicitHeight: Math.round(Style.spacing.controlHeight * 0.72)
      radius: Style.cornerRadius
      color: selected ? Style.selectedFill : Style.normalFill

      Text {
        id: caption
        anchors.centerIn: parent
        text: String(parent.modelData).replace(/_/g, " ")
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        color: parent.selected ? Color.foreground : Color.muted
      }

      MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.picked(String(parent.modelData))
      }
    }
  }
}
