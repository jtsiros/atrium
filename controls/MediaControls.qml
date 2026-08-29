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
  readonly property string title: String(attributes.media_title || "")
  readonly property string artist: String(attributes.media_artist || "")

  spacing: Style.spacing.lg

  Text {
    width: parent.width
    visible: root.title !== ""
    text: root.artist !== "" ? root.title + " — " + root.artist : root.title
    textFormat: Text.PlainText
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    color: Color.muted
    elide: Text.ElideRight
  }

  Row {
    visible: root.controls.indexOf("transport") !== -1
    spacing: Style.spacing.lg

    Button {
      iconText: "󰒮"
      onClicked: root.service.act(root.row.entityId, "previous")
    }

    Button {
      iconText: root.row && root.row.state === "playing" ? "󰏤" : "󰐊"
      onClicked: root.service.act(root.row.entityId, "playPause")
    }

    Button {
      iconText: "󰒭"
      onClicked: root.service.act(root.row.entityId, "next")
    }
  }

  SliderRow {
    width: parent.width
    visible: root.controls.indexOf("volume") !== -1
    bar: root.bar
    label: "volume"
    minimum: 0
    maximum: 1
    step: 0.02
    value: root.attributes.volume_level !== undefined ? Number(root.attributes.volume_level) : 0
    readout: Math.round((root.attributes.volume_level || 0) * 100) + "%"
    onReleased: function(next) {
      root.service.act(root.row.entityId, "setVolume", { level: next })
    }
  }
}
