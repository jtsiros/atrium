import QtQuick
import qs.Ui
import qs.Commons

Item {
  id: root

  property QtObject service: null

  readonly property var shortcuts: [
    { keys: "j  k", what: "Move between rows" },
    { keys: "↑  ↓", what: "Move between rows" },
    { keys: "h  l", what: "Previous or next room" },
    { keys: "←  →", what: "Previous or next room" },
    { keys: "enter", what: "Toggle the selected row" },
    { keys: "space", what: "Toggle the selected row" },
    { keys: "e", what: "Expand the selected row's controls" },
    { keys: "r", what: "Refresh from Home Assistant" },
    { keys: "s", what: "Open these settings" },
    { keys: "tab", what: "Move to the next bar panel" },
    { keys: "esc", what: "Close the panel" }
  ]

  Column {
    anchors.fill: parent
    anchors.margins: Style.spacing.panelPadding
    spacing: Style.spacing.xxxl

    Text {
      text: "Keyboard"
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.heading
      color: Color.foreground
    }

    Text {
      width: parent.width
      text: "With the panel open."
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
      color: Color.muted
      wrapMode: Text.WordWrap
    }

    Column {
      width: parent.width
      spacing: 1

      Repeater {
        model: root.shortcuts

        delegate: Row {
          id: shortcutRow
          required property var modelData

          width: parent.width
          height: Style.space(24)
          spacing: Style.spacing.huge

          Text {
            anchors.verticalCenter: parent.verticalCenter
            width: Style.space(70)
            text: shortcutRow.modelData.keys
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.body
            color: Color.accent
          }

          Text {
            anchors.verticalCenter: parent.verticalCenter
            text: shortcutRow.modelData.what
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.body
            color: Color.foreground
          }
        }
      }
    }

    Text {
      width: parent.width
      text: "From a terminal or a keybind:"
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
      color: Color.muted
      topPadding: Style.spacing.lg
    }

    Column {
      width: parent.width
      spacing: Style.spacing.sm

      Repeater {
        model: [
          "omarchy-shell atrium toggle",
          "omarchy-shell atrium toggleEntity light.desk",
          "omarchy-shell atrium activate scene.movie_night",
          "omarchy-shell atrium rooms",
          "omarchy-shell atrium status"
        ]

        delegate: Text {
          required property string modelData

          text: modelData
          textFormat: Text.PlainText
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          color: Color.muted
        }
      }
    }
  }
}
