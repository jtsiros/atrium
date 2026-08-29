import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Ui
import qs.Commons
import "Glyphs.js" as Glyphs
import "settings" as Panes

Item {
  id: root

  property var shell: null
  property var manifest: null

  // Assigned by the shell's panel loader on load. A readonly binding here
  // throws during injection and aborts the rest of it, leaving the overlay
  // unable to open.
  property var service: null

  property bool opened: false
  property string section: "connection"

  readonly property color surface: Color.menu.background
  readonly property color border: Color.popups.border

  function open(payloadJson) {
    var next = "connection"
    try {
      var payload = JSON.parse(payloadJson || "{}")
      if (payload && payload.section) next = String(payload.section)
    } catch (e) {
      next = "connection"
    }
    section = next
    opened = true
    Qt.callLater(function() { keys.forceActiveFocus() })
  }

  function close() { opened = false }

  PanelWindow {
    id: window
    visible: root.opened
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    WlrLayershell.namespace: "atrium-settings"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore

    Rectangle {
      anchors.fill: parent
      color: Color.menu.scrim
    }

    MouseArea {
      anchors.fill: parent
      onClicked: root.close()
    }

    Rectangle {
      id: card
      anchors.centerIn: parent
      width: Math.min(Style.space(860), window.width - Style.gapsOut * 4)
      height: Math.min(Style.space(580), window.height - Style.gapsOut * 4)
      radius: Style.cornerRadius
      color: root.surface
      border.width: Math.max(1, Style.space(2))
      border.color: root.border

      MouseArea { anchors.fill: parent }

      PanelKeyCatcher {
        id: keys
        anchors.fill: parent
        onCloseRequested: root.close()
      }

      Column {
        anchors.fill: parent
        spacing: 0

        Item {
          width: parent.width
          height: Style.space(44)

          Row {
            anchors.left: parent.left
            anchors.leftMargin: Style.spacing.panelPadding
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.spacing.md

            Text {
              anchors.verticalCenter: parent.verticalCenter
              text: Glyphs.homeAssistant
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.iconLarge
              color: Color.accent
            }

            Text {
              anchors.verticalCenter: parent.verticalCenter
              text: "Atrium"
              textFormat: Text.PlainText
              font.family: Style.font.family
              font.pixelSize: Style.font.title
              color: Color.foreground
            }
          }

          Button {
            anchors.right: parent.right
            anchors.rightMargin: Style.spacing.panelPadding
            anchors.verticalCenter: parent.verticalCenter
            iconText: Glyphs.close
            onClicked: root.close()
          }

          PanelSeparator {
            anchors.bottom: parent.bottom
            width: parent.width
          }
        }

        Row {
          width: parent.width
          height: card.height - Style.space(44)
          spacing: 0

          Column {
            id: nav
            width: Style.space(148)
            height: parent.height
            padding: Style.spacing.lg
            spacing: 1

            Repeater {
              model: [
                { id: "connection", label: "Connection", glyph: Glyphs.link },
                { id: "areas", label: "Areas", glyph: Glyphs.areaFallback },
                { id: "entities", label: "Entities", glyph: Glyphs.star },
                { id: "keyboard", label: "Keyboard", glyph: Glyphs.keyboard },
                { id: "activity", label: "Activity", glyph: Glyphs.pulse }
              ]

              delegate: Rectangle {
                id: navItem
                required property var modelData

                readonly property bool selected: modelData.id === root.section

                width: nav.width - Style.spacing.lg * 2
                height: Style.spacing.controlHeight
                radius: Style.cornerRadius
                color: selected ? Style.selectedFill : "transparent"

                Row {
                  anchors.fill: parent
                  anchors.leftMargin: Style.spacing.xxl
                  spacing: Style.spacing.xl

                  Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: navItem.modelData.glyph
                    textFormat: Text.PlainText
                    font.family: Style.font.family
                    font.pixelSize: Style.font.iconSmall
                    color: navItem.selected ? Color.foreground : Color.muted
                  }

                  Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: navItem.modelData.label
                    textFormat: Text.PlainText
                    font.family: Style.font.family
                    font.pixelSize: Style.font.body
                    color: navItem.selected ? Color.foreground : Color.muted
                  }
                }

                MouseArea {
                  anchors.fill: parent
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.section = navItem.modelData.id
                }
              }
            }
          }

          Rectangle {
            width: 1
            height: parent.height
            color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.1)
          }

          Loader {
            width: parent.width - nav.width - 1
            height: parent.height
            sourceComponent: {
              switch (root.section) {
              case "areas": return areasPane
              case "entities": return entitiesPane
              case "keyboard": return keyboardPane
              case "activity": return activityPane
              }
              return connectionPane
            }
          }
        }
      }
    }
  }

  Component {
    id: connectionPane
    Panes.ConnectionPane { service: root.service }
  }

  Component {
    id: areasPane
    Panes.AreasPane { service: root.service }
  }

  Component {
    id: entitiesPane
    Panes.EntitiesPane { service: root.service }
  }

  Component {
    id: keyboardPane
    Panes.KeyboardPane { service: root.service }
  }

  Component {
    id: activityPane
    Panes.ActivityPane { service: root.service }
  }
}
