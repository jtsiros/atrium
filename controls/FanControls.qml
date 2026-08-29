import QtQuick
import qs.Commons

Column {
  id: root

  property QtObject bar: null
  property QtObject service: null
  property var row: null

  readonly property var attributes: row && row.attributes ? row.attributes : ({})

  spacing: Style.spacing.lg

  SliderRow {
    width: parent.width
    bar: root.bar
    label: "speed"
    minimum: 0
    maximum: 100
    step: attributes.percentage_step !== undefined ? Number(attributes.percentage_step) : 1
    integer: true
    value: root.attributes.percentage !== undefined ? Number(root.attributes.percentage) : 0
    readout: root.attributes.percentage ? Math.round(root.attributes.percentage) + "%" : "off"
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setSpeed", { percentage: next })
    }
  }
}
