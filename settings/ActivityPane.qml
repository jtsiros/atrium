import QtQuick
import qs.Ui
import qs.Commons

Item {
  id: root

  property QtObject service: null
  property string filter: "all"

  readonly property var entries: {
    if (!service) return []
    var out = []
    var all = service.logEntries
    for (var i = all.length - 1; i >= 0; i--) {
      if (filter === "problems" && all[i].level === "info") continue
      out.push(all[i])
    }
    return out
  }

  function since(at) {
    var seconds = Math.max(0, Math.round((Date.now() - at) / 1000))
    if (seconds < 60) return seconds + "s ago"
    if (seconds < 3600) return Math.round(seconds / 60) + "m ago"
    return Math.round(seconds / 3600) + "h ago"
  }

  function colorFor(level) {
    if (level === "error") return Color.urgent
    if (level === "warn") return Color.bar.active
    return Color.muted
  }

  Column {
    anchors.fill: parent
    anchors.margins: Style.spacing.panelPadding
    spacing: Style.spacing.xxxl

    Row {
      width: parent.width

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: "Activity"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.heading
        color: Color.foreground
      }

      Item {
        height: 1
        width: Math.max(0, parent.width - x - count.implicitWidth)
      }

      Text {
        id: count
        anchors.verticalCenter: parent.verticalCenter
        text: root.entries.length + " entries"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
        color: Color.muted
      }
    }

    Row {
      spacing: Style.spacing.lg

      Button {
        text: "All"
        bordered: root.filter === "all"
        onClicked: root.filter = "all"
      }

      Button {
        text: "Issues"
        bordered: root.filter === "problems"
        onClicked: root.filter = "problems"
      }

      Button {
        text: "Clear"
        onClicked: root.service.clearLog()
      }
    }

    Text {
      width: parent.width
      visible: root.entries.length === 0
      text: root.filter === "problems" ? "No issues." : "Nothing recorded yet."
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
      color: Color.muted
    }

    ListView {
      id: list
      width: parent.width
      height: root.height - y - Style.spacing.panelPadding * 2
      clip: true
      spacing: Style.spacing.xs
      model: root.entries

      delegate: Row {
        id: entry
        required property var modelData

        width: list.width
        spacing: Style.spacing.xl

        Rectangle {
          width: Math.max(1, Style.space(2))
          height: line.implicitHeight
          color: root.colorFor(entry.modelData.level)
        }

        Text {
          width: Style.space(64)
          text: root.since(entry.modelData.at)
          textFormat: Text.PlainText
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          color: Color.muted
        }

        Text {
          id: line
          width: Math.max(0, parent.width - x - Style.spacing.xl)
          text: entry.modelData.entityId !== ""
            ? entry.modelData.text + "  (" + entry.modelData.entityId + ")"
            : entry.modelData.text
          textFormat: Text.PlainText
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          color: entry.modelData.level === "info" ? Color.foreground : root.colorFor(entry.modelData.level)
          wrapMode: Text.WordWrap
        }
      }
    }
  }
}
