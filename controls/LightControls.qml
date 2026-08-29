import QtQuick
import qs.Commons

Column {
  id: root

  property QtObject bar: null
  property QtObject service: null
  property var row: null

  readonly property var attributes: row && row.attributes ? row.attributes : ({})
  readonly property var controls: row && row.controls ? row.controls : []

  readonly property real brightness: attributes.brightness !== undefined
    ? Number(attributes.brightness) : 0
  readonly property real kelvin: attributes.color_temp_kelvin !== undefined
    ? Number(attributes.color_temp_kelvin) : 0
  readonly property real minKelvin: attributes.min_color_temp_kelvin !== undefined
    ? Number(attributes.min_color_temp_kelvin) : 2000
  readonly property real maxKelvin: attributes.max_color_temp_kelvin !== undefined
    ? Number(attributes.max_color_temp_kelvin) : 6500

  readonly property var swatches: [
    { hue: 0, saturation: 85, color: "#ff5f5f" },
    { hue: 28, saturation: 80, color: "#ffa64d" },
    { hue: 48, saturation: 75, color: "#ffe066" },
    { hue: 120, saturation: 55, color: "#7ed07e" },
    { hue: 210, saturation: 70, color: "#5fb0ff" },
    { hue: 270, saturation: 55, color: "#b98cff" },
    { hue: 0, saturation: 0, color: "#ffffff" }
  ]

  spacing: Style.spacing.xl

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("brightness") !== -1
    bar: root.bar
    label: "bright"
    minimum: 0
    maximum: 255
    step: 1
    integer: true
    value: root.brightness
    readout: Math.round(root.brightness / 255 * 100) + "%"
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setBrightness", { brightness: next })
    }
  }

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("colorTemp") !== -1
    bar: root.bar
    label: "warmth"
    minimum: root.minKelvin
    maximum: root.maxKelvin
    step: 50
    integer: true
    value: root.kelvin
    readout: Math.round(root.kelvin) + "K"
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setColorTemp", { kelvin: next })
    }
  }

  Row {
    visible: root.controls.indexOf("color") !== -1
    spacing: Style.spacing.md

    Repeater {
      model: root.swatches
      delegate: Rectangle {
        required property var modelData

        width: Style.space(18)
        height: Style.space(18)
        radius: Style.cornerRadius
        color: modelData.color
        border.width: 1
        border.color: Qt.rgba(0, 0, 0, 0.4)

        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: root.service.act(root.row.entityId, "setColor", {
            hue: parent.modelData.hue,
            saturation: parent.modelData.saturation
          })
        }
      }
    }
  }
}
