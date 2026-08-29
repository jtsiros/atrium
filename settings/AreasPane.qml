import QtQuick
import qs.Ui
import qs.Commons
import ".." as Atrium
import "../Glyphs.js" as Glyphs

Item {
  id: root

  property QtObject service: null

  readonly property var areas: service ? service.areas : []
  readonly property var tabs: service ? service.tabs : []

  Column {
    anchors.fill: parent
    anchors.margins: Style.spacing.panelPadding
    spacing: Style.spacing.xxxl

    Row {
      width: parent.width

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: "Areas"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.heading
        color: Color.foreground
      }

      Item {
        height: 1
        width: Math.max(0, parent.width - x - shownCount.implicitWidth)
      }

      Text {
        id: shownCount
        anchors.verticalCenter: parent.verticalCenter
        text: root.tabs.length + " of " + root.areas.length + " shown"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
        color: Color.muted
      }
    }

    Row {
      spacing: Style.spacing.lg

      Button {
        text: "Match Home Assistant"
        iconText: Glyphs.refresh
        bordered: true
        onClicked: root.service.importDashboardPrefs()
      }

      Button {
        text: "Show all"
        onClicked: root.service.showAllAreas()
      }
    }

    Row {
      width: parent.width
      height: root.height - y - Style.spacing.panelPadding * 2
      spacing: Style.spacing.huge

      ListView {
        id: list
        width: parent.width - side.width - parent.spacing
        height: parent.height
        clip: true
        spacing: 1
        model: root.areas

        delegate: Rectangle {
          id: areaRow
          required property var modelData

          width: list.width
          height: Style.spacing.controlHeight
          radius: Style.cornerRadius
          color: hover.hovered ? Style.hoverFill : "transparent"
          opacity: modelData.hidden || modelData.entityCount === 0 ? 0.42 : 1

          HoverHandler { id: hover }

          Row {
            anchors.fill: parent
            anchors.leftMargin: Style.spacing.lg
            spacing: Style.spacing.xl

            Atrium.Glyph {
              anchors.verticalCenter: parent.verticalCenter
              glyph: areaRow.modelData.glyph
            }

            Text {
              anchors.verticalCenter: parent.verticalCenter
              width: parent.width - x - actions.width - parent.spacing * 2
              text: areaRow.modelData.name
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.body
              color: Color.foreground
              elide: Text.ElideRight
            }
          }

          Row {
            id: actions
            anchors.right: parent.right
            anchors.rightMargin: Style.spacing.lg
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.spacing.md

            Text {
              anchors.verticalCenter: parent.verticalCenter
              text: areaRow.modelData.entityCount === 0
                ? "empty" : String(areaRow.modelData.entityCount)
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.caption
              color: Color.muted
            }

            Button {
              iconText: Glyphs.arrowUp
              tooltipText: "Move up"
              onClicked: root.service.moveArea(areaRow.modelData.areaId, -1)
            }

            Button {
              iconText: Glyphs.arrowDown
              tooltipText: "Move down"
              onClicked: root.service.moveArea(areaRow.modelData.areaId, 1)
            }

            Button {
              iconText: root.service.isPinnedTab("area:" + areaRow.modelData.areaId)
                ? Glyphs.pin : Glyphs.pinOutline
              tooltipText: root.service.isPinnedTab("area:" + areaRow.modelData.areaId)
                ? "Unpin" : "Pin"
              foreground: root.service.isPinnedTab("area:" + areaRow.modelData.areaId)
                ? Color.accent : Color.muted
              onClicked: root.service.togglePinnedTab("area:" + areaRow.modelData.areaId)
            }

            Button {
              iconText: areaRow.modelData.hidden ? Glyphs.eyeOff : Glyphs.eye
              tooltipText: areaRow.modelData.hidden ? "Show" : "Hide"
              foreground: areaRow.modelData.hidden ? Color.muted : Color.accent
              onClicked: root.service.toggleAreaHidden(areaRow.modelData.areaId)
            }
          }
        }
      }

      Column {
        id: side
        width: Style.space(218)
        height: parent.height
        spacing: Style.spacing.xxxl

        Column {
          width: parent.width
          spacing: Style.spacing.xl

          Text {
            text: "PANEL TABS"
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            color: Color.muted
          }

          Flow {
            width: parent.width
            spacing: Style.spacing.sm

            Repeater {
              model: root.tabs

              delegate: Rectangle {
                id: previewTab
                required property var modelData
                required property int index

                implicitWidth: previewRow.implicitWidth + Style.spacing.md * 2
                implicitHeight: Style.space(20)
                radius: Style.cornerRadius
                color: index === 0 ? Style.selectedFill : Style.normalFill

                Row {
                  id: previewRow
                  anchors.centerIn: parent
                  spacing: Style.spacing.xs

                  Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: previewTab.modelData.glyph
                    textFormat: Text.PlainText
                    font.family: Style.font.family
                    font.pixelSize: Style.font.caption
                    color: previewTab.index === 0 ? Color.foreground : Color.muted
                  }

                  Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: previewTab.modelData.title
                    textFormat: Text.PlainText
                    font.family: Style.font.family
                    font.pixelSize: Style.font.caption
                    color: previewTab.index === 0 ? Color.foreground : Color.muted
                  }
                }
              }
            }
          }
        }

        Column {
          width: parent.width
          spacing: Style.spacing.xl

          Text {
            text: "OPTIONS"
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            color: Color.muted
          }

          OptionRow {
            width: parent.width
            label: "Hide empty rooms"
            checked: root.service ? root.service.hideEmptyAreas : true
            onToggled: root.service.setHideEmptyAreas(!checked)
          }

          OptionRow {
            width: parent.width
            label: "Show unassigned"
            checked: root.service ? root.service.showUnassigned : false
            onToggled: root.service.setShowUnassigned(!checked)
          }
        }
      }
    }
  }
}
