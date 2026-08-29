import QtQuick
import qs.Ui
import qs.Commons

Column {
  id: root

  property QtObject bar: null
  property QtObject service: null
  property var row: null

  readonly property var attributes: row && row.attributes ? row.attributes : ({})
  readonly property var controls: row && row.controls ? row.controls : []

  spacing: Style.spacing.lg

  Row {
    spacing: Style.spacing.lg

    Button {
      visible: root.controls.indexOf("openClose") !== -1
      iconText: "󰜷"
      text: "Open"
      onClicked: root.service.act(root.row.entityId, "open")
    }

    Button {
      visible: root.controls.indexOf("stop") !== -1
      iconText: "󰓛"
      text: "Stop"
      onClicked: root.service.act(root.row.entityId, "stop")
    }

    Button {
      visible: root.controls.indexOf("openClose") !== -1
      iconText: "󰜮"
      text: "Close"
      onClicked: root.service.act(root.row.entityId, "close")
    }
  }

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("position") !== -1
    bar: root.bar
    label: "open"
    minimum: 0
    maximum: 100
    step: 1
    integer: true
    value: root.attributes.current_position !== undefined
      ? Number(root.attributes.current_position) : 0
    readout: Math.round(root.attributes.current_position || 0) + "%"
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setPosition", { position: next })
    }
  }
}
