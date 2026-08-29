import QtQuick
import qs.Ui
import qs.Commons
import ".." as Atrium
import "../Glyphs.js" as Glyphs

Item {
  id: root

  property QtObject service: null
  property string filter: ""
  property var collapsed: ({})

  readonly property int indent: Style.spacing.lg + Style.font.iconSmall + Style.spacing.md

  function isCollapsed(title) {
    return collapsed[title] === true && filter === ""
  }

  function toggleCollapsed(title) {
    var next = {}
    for (var key in collapsed) next[key] = collapsed[key]
    next[title] = !next[title]
    collapsed = next
  }

  readonly property var groups: {
    if (!service) return []
    service.rowsRevision
    var query = filter.toLowerCase()
    var out = []
    var tabs = service.tabs
    for (var i = 0; i < tabs.length; i++) {
      var rows = []
      var ids = tabs[i].entityIds || []
      for (var j = 0; j < ids.length; j++) {
        var row = service.rowFor(ids[j])
        if (!row) continue
        if (query !== "" && row.name.toLowerCase().indexOf(query) === -1) continue
        rows.push(row)
      }
      if (rows.length > 0) out.push({ title: tabs[i].title, rows: rows })
    }
    return out
  }

  Column {
    anchors.fill: parent
    anchors.margins: Style.spacing.panelPadding
    spacing: Style.spacing.xxxl

    Row {
      width: parent.width

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: "Entities"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.heading
        color: Color.foreground
      }

      Item {
        height: 1
        width: Math.max(0, parent.width - x - counts.implicitWidth)
      }

      Text {
        id: counts
        anchors.verticalCenter: parent.verticalCenter
        text: (root.service ? root.service.favorites.length : 0) + " pinned"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
        color: Color.muted
      }
    }

    Row {
      width: parent.width
      spacing: Style.spacing.huge

      TextField {
        width: parent.width - pinnedToggle.width - parent.spacing
        placeholderText: "Search entities"
        onTextChanged: root.filter = text
      }

      OptionRow {
        id: pinnedToggle
        width: Style.space(218)
        anchors.verticalCenter: parent.verticalCenter
        label: "Show pinned tab"
        checked: root.service ? root.service.showFavorites : false
        onToggled: root.service.setFavorites(root.service.favorites, !checked)
      }
    }

    ListView {
      id: list
      width: parent.width
      height: root.height - y - Style.spacing.panelPadding * 2
      clip: true
      spacing: 0
      model: root.groups

      delegate: Column {
        id: group
        required property var modelData

        width: list.width
        spacing: 0

        Item {
          width: parent.width
          height: Style.spacing.controlHeight

          Row {
            anchors.left: parent.left
            anchors.leftMargin: Style.spacing.lg
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.spacing.md

            Text {
              anchors.verticalCenter: parent.verticalCenter
              text: root.isCollapsed(group.modelData.title) ? Glyphs.chevronRight : Glyphs.chevronDown
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.iconSmall
              color: Color.muted
            }

            Text {
              anchors.verticalCenter: parent.verticalCenter
              text: group.modelData.title + " · " + group.modelData.rows.length
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.caption
              color: Color.muted
            }
          }

          MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.toggleCollapsed(group.modelData.title)
          }
        }

        Repeater {
          model: root.isCollapsed(group.modelData.title) ? [] : group.modelData.rows

          delegate: Rectangle {
            id: entity
            required property var modelData

            readonly property bool pinned: root.service.isFavorite(modelData.entityId)

            width: group.width
            height: Style.space(24)
            radius: Style.cornerRadius
            color: entityHover.hovered ? Style.hoverFill : "transparent"

            HoverHandler { id: entityHover }

            Row {
              anchors.fill: parent
              anchors.leftMargin: root.indent
              spacing: Style.spacing.xl

              Atrium.Glyph {
                anchors.verticalCenter: parent.verticalCenter
                glyph: entity.modelData.glyph
                size: Style.font.iconSmall
              }

              Text {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - x - trailing.width - parent.spacing * 2
                text: entity.modelData.name
                textFormat: Text.PlainText
                font.family: Style.font.family
                font.pixelSize: Style.font.body
                color: Color.foreground
                elide: Text.ElideRight
              }
            }

            Row {
              id: trailing
              anchors.right: parent.right
              anchors.rightMargin: Style.spacing.lg
              anchors.verticalCenter: parent.verticalCenter
              spacing: Style.spacing.lg

              Text {
                anchors.verticalCenter: parent.verticalCenter
                text: entity.modelData.displayState
                textFormat: Text.PlainText
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
                color: Color.muted
              }

              Text {
                anchors.verticalCenter: parent.verticalCenter
                text: entity.pinned ? Glyphs.star : Glyphs.starOutline
                textFormat: Text.PlainText
                font.family: Style.font.family
                font.pixelSize: Style.font.iconSmall
                color: entity.pinned ? Color.bar.active : Color.muted
                opacity: entity.pinned ? 1 : 0.45

                MouseArea {
                  anchors.fill: parent
                  anchors.margins: -Style.spacing.sm
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.service.toggleFavorite(entity.modelData.entityId)
                }
              }
            }
          }
        }
      }
    }
  }
}
