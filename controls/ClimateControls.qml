import QtQuick
import qs.Commons

Column {
  id: root

  property QtObject bar: null
  property QtObject service: null
  property var row: null

  readonly property var attributes: row && row.attributes ? row.attributes : ({})
  readonly property var controls: row && row.controls ? row.controls : []

  function reading(key, fallback) {
    var value = attributes[key]
    // Home Assistant sends null for a value it does not have; Number(null) is 0,
    // which silently collapses a temperature band.
    if (value === undefined || value === null) return fallback
    var parsed = Number(value)
    return isFinite(parsed) ? parsed : fallback
  }

  function has(key) {
    return attributes[key] !== undefined && attributes[key] !== null
  }

  readonly property real minTemp: reading("min_temp", 45)
  readonly property real maxTemp: reading("max_temp", 95)
  readonly property real step: reading("target_temp_step", 1)

  spacing: Style.spacing.xl

  ModeSelector {
    width: parent.width
    visible: root.controls.indexOf("hvacMode") !== -1
    modes: root.attributes.hvac_modes || []
    current: root.row ? root.row.state : ""
    onPicked: function(mode) {
      root.service.act(root.row.entityId, "setHvacMode", { mode: mode })
    }
  }

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("temperature") !== -1
    bar: root.bar
    label: "target"
    minimum: root.minTemp
    maximum: root.maxTemp
    step: root.step
    value: root.reading("temperature", root.minTemp)
    readout: root.has("temperature") ? String(root.reading("temperature", root.minTemp)) : ""
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setTemperature", { temperature: next })
    }
  }

  SliderRow {
    id: lowBand
    width: parent.width
    visible: root.controls.indexOf("temperatureRange") !== -1
    bar: root.bar
    label: "low"
    minimum: root.minTemp
    maximum: root.maxTemp
    step: root.step
    value: root.reading("target_temp_low", root.minTemp)
    readout: root.has("target_temp_low") ? String(root.reading("target_temp_low", root.minTemp)) : ""
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setTemperatureRange", {
        low: next,
        high: Math.max(next, root.reading("target_temp_high", root.maxTemp))
      })
    }
  }

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("temperatureRange") !== -1
    bar: root.bar
    label: "high"
    minimum: root.minTemp
    maximum: root.maxTemp
    step: root.step
    value: root.reading("target_temp_high", root.maxTemp)
    readout: root.has("target_temp_high") ? String(root.reading("target_temp_high", root.maxTemp)) : ""
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setTemperatureRange", {
        low: Math.min(next, root.reading("target_temp_low", root.minTemp)),
        high: next
      })
    }
  }

  ModeSelector {
    width: parent.width
    visible: root.controls.indexOf("fanMode") !== -1
    modes: root.attributes.fan_modes || []
    current: String(root.attributes.fan_mode || "")
    onPicked: function(mode) {
      root.service.act(root.row.entityId, "setFanMode", { mode: mode })
    }
  }

  ModeSelector {
    width: parent.width
    visible: root.controls.indexOf("presetMode") !== -1
    modes: root.attributes.preset_modes || []
    current: String(root.attributes.preset_mode || "")
    onPicked: function(mode) {
      root.service.act(root.row.entityId, "setPresetMode", { mode: mode })
    }
  }
}
