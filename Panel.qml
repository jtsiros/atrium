import QtQuick
import qs.Ui
import qs.Commons
import "Glyphs.js" as Glyphs

Panel {
  id: root
  moduleName: "io.github.bitshiftxr.atrium"
  manageIpc: false

  readonly property var service: bar?.shell?.serviceFor("io.github.bitshiftxr.atrium")
  readonly property bool connected: service ? service.connected : false
  readonly property bool needsSetup: service ? service.needsSetup : true
  readonly property int activeCount: service ? service.activeCount : 0
  readonly property var tabs: service ? service.tabs : []
  readonly property var entities: service ? service.activeTabEntities : []
  readonly property string activeTab: service ? service.activeTab : ""
  readonly property var pinned: service ? service.pinnedTabList : []
  readonly property string problem: service ? service.problem : ""
  readonly property string problemLevel: service ? service.problemLevel : ""

  // Taken from the room box's own rendered geometry rather than recomputed from
  // tokens: Dropdown owns padding this file cannot see, and the two columns
  // have to agree exactly.
  // One padding for the room box, the pinned chips and every device row, so the
  // whole column sits evenly between the panel's edges.
  readonly property real pad: Style.spacing.rowPaddingX
  readonly property bool currentPinned: service ? service.isPinnedTab(activeTab) : false

  // Anchored to an entity id, not a position: the daemon re-indexes this list
  // whenever an entity appears or drops, and a stale index would put the
  // selection on a different device than the one highlighted.
  property string cursorId: ""
  property bool cursorActive: false
  property string expandedId: ""

  readonly property int cursor: {
    for (var i = 0; i < entities.length; i++) {
      if (entities[i].entityId === cursorId) return i
    }
    return -1
  }

  function moveCursor(delta) {
    if (entities.length === 0) {
      cursorId = ""
      return
    }
    var from = cursor
    var next = from === -1 ? 0 : Math.max(0, Math.min(entities.length - 1, from + delta))
    cursorId = entities[next].entityId
  }

  function moveTab(delta) {
    if (tabs.length === 0) return
    var index = 0
    for (var i = 0; i < tabs.length; i++) {
      if (tabs[i].id === activeTab) index = i
    }
    var next = index + delta
    if (next < 0 || next >= tabs.length) return
    selectTab(tabs[next].id)
  }

  function selectTab(tabId) {
    if (!service) return
    service.selectTab(tabId)
    cursorId = ""
    cursorActive = false
    expandedId = ""
  }

  function activateCursor() {
    if (!service || cursorId === "" || cursor === -1) return
    service.toggle(cursorId)
  }

  function expandCursor() {
    if (cursorId === "" || cursor === -1) return
    expandedId = expandedId === cursorId ? "" : cursorId
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  Connections {
    target: root.service
    // service resolves after this component is created, so the signals cannot
    // be matched at compile time.
    ignoreUnknownSignals: true

    function onOpened() { root.open() }
    function onCloseRequested() { root.close() }
    function onToggleRequested() { root.toggle() }
  }

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    // One glyph, always. Swapping to a different mark changes the button's
    // implicit width, which reflows every widget beside it on the bar each
    // time the connection state moves. Colour carries the state instead.
    text: Glyphs.homeAssistant
    dimmed: root.connected && root.activeCount === 0
    active: !root.connected
    onPressed: root.toggle()
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keys
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(content.implicitHeight, Style.space(600))

    PanelKeyCatcher {
      id: keys
      anchors.fill: parent

      onMoveRequested: function(dx, dy) {
        if (!root.cursorActive) { root.cursorActive = true; return }
        if (dy !== 0) root.moveCursor(dy)
        else if (dx !== 0) root.moveTab(dx)
      }
      onActivateRequested: if (root.cursorActive) root.activateCursor()
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(text) {
        switch (text) {
        case "e": case "E": root.expandCursor(); break
        case "r": case "R": if (root.service) root.service.refresh(); break
        case "s": case "S": if (root.service) root.service.openSettings("connection"); break
        }
      }

      Column {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Style.spacing.lg

        Row {
          x: root.pad
          width: Math.max(0, parent.width - root.pad * 2)
          spacing: Style.spacing.md

          Glyph {
            anchors.verticalCenter: parent.verticalCenter
            glyph: Glyphs.homeAssistant
            color: Color.accent
            size: Style.font.iconLarge
            slot: Style.font.icon
          }

          Text {
            anchors.verticalCenter: parent.verticalCenter
            text: "Atrium"
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.title
            color: Color.foreground
          }

          Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: Style.space(5)
            height: Style.space(5)
            radius: Style.cornerRadius
            color: root.connected ? Color.accent : Color.muted
          }

          Item {
            height: 1
            width: Math.max(0, parent.width - x - tools.implicitWidth - parent.spacing)
          }

          Row {
            id: tools
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.spacing.sm

            Button {
              anchors.verticalCenter: parent.verticalCenter
              iconText: root.currentPinned ? Glyphs.pin : Glyphs.pinOutline
              tooltipText: root.currentPinned ? "Unpin this room" : "Pin this room"
              foreground: root.currentPinned ? Color.accent : Color.foreground
              onClicked: if (root.service) root.service.togglePinnedTab(root.activeTab)
            }

            Button {
              anchors.verticalCenter: parent.verticalCenter
              iconText: Glyphs.refresh
              tooltipText: "Refresh"
              onClicked: if (root.service) root.service.refresh()
            }

            Button {
              anchors.verticalCenter: parent.verticalCenter
              iconText: Glyphs.cog
              tooltipText: "Settings"
              onClicked: if (root.service) root.service.openSettings("connection")
            }
          }
        }

        Item {
          width: parent.width
          height: roomPicker.height

          Dropdown {
            id: roomPicker
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: root.pad
            anchors.rightMargin: root.pad
            label: "Room"
            showLabel: false
            options: root.service ? root.service.tabOptions : []
            onChanged: function(next) { root.selectTab(next) }
          }

          // Dropdown assigns to its own `value` when an item is picked, which
          // would destroy a plain binding and leave the label stuck on the last
          // room chosen here while the chips and the list moved on.
          Binding {
            target: roomPicker
            property: "value"
            value: root.activeTab
          }
        }

        Flow {
          x: root.pad
          width: Math.max(0, parent.width - root.pad * 2)
          visible: root.pinned.length > 0
          spacing: Style.spacing.sm

          Repeater {
            model: root.pinned

            delegate: Rectangle {
              id: pinChip
              required property var modelData

              readonly property bool selected: modelData.id === root.activeTab

              implicitWidth: chipRow.implicitWidth + Style.spacing.lg * 2
              implicitHeight: Math.round(Style.spacing.controlHeight * 0.85)
              radius: Style.cornerRadius
              color: selected ? Style.selectedFill : Style.normalFill

              Row {
                id: chipRow
                anchors.centerIn: parent
                spacing: Style.spacing.sm

                Text {
                  anchors.verticalCenter: parent.verticalCenter
                  text: pinChip.modelData.glyph
                  textFormat: Text.PlainText
                  font.family: Style.font.family
                  font.pixelSize: Style.font.iconSmall
                  color: pinChip.selected ? Color.foreground : Color.muted
                }

                Text {
                  anchors.verticalCenter: parent.verticalCenter
                  text: pinChip.modelData.title
                  textFormat: Text.PlainText
                  font.family: Style.font.family
                  font.pixelSize: Style.font.bodySmall
                  color: pinChip.selected ? Color.foreground : Color.muted
                }
              }

              MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.selectTab(pinChip.modelData.id)
              }
            }
          }
        }

        PanelSeparator {
          x: root.pad
          width: Math.max(0, parent.width - root.pad * 2)
        }

        // Only present while something is actually wrong; it clears itself.
        Rectangle {
          x: root.pad
          width: Math.max(0, parent.width - root.pad * 2)
          height: visible ? problemText.implicitHeight + Style.spacing.lg * 2 : 0
          visible: root.problem !== ""
          color: Style.normalFill
          radius: Style.cornerRadius

          Rectangle {
            width: Math.max(1, Style.space(2))
            height: parent.height
            color: root.problemLevel === "error" ? Color.urgent : Color.bar.active
          }

          Text {
            id: problemText
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Style.spacing.xxl
            anchors.rightMargin: Style.spacing.xxl
            anchors.verticalCenter: parent.verticalCenter
            text: root.problem
            textFormat: Text.PlainText
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            color: Color.foreground
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
          }

          MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: if (root.service) root.service.openSettings("activity")
          }
        }

        Column {
          width: parent.width
          spacing: 1
          visible: root.entities.length > 0

          Repeater {
            model: root.entities

            delegate: EntityRow {
              id: entityRow
              required property var modelData
              required property int index

              width: parent.width
              bar: root.bar
              service: root.service
              row: modelData
              leadingInset: root.pad
              trailingInset: root.pad
              hasCursor: root.cursorActive && modelData.entityId === root.cursorId
              expanded: modelData.entityId === root.expandedId

              onActivated: {
                root.cursorId = entityRow.modelData.entityId
                root.cursorActive = true
                if (root.service) root.service.toggle(entityRow.modelData.entityId)
              }
              onToggleExpanded: {
                root.cursorId = entityRow.modelData.entityId
                root.cursorActive = true
                root.expandedId = root.expandedId === entityRow.modelData.entityId
                  ? "" : entityRow.modelData.entityId
              }
            }
          }
        }

        Text {
          width: parent.width
          visible: root.entities.length === 0
          text: root.needsSetup ? "Not connected." : "Nothing in this room."
          textFormat: Text.PlainText
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          color: Color.muted
          wrapMode: Text.WordWrap
        }

        Button {
          visible: root.needsSetup
          text: "Open settings"
          bordered: true
          onClicked: if (root.service) root.service.openSettings("connection")
        }

      }
    }
  }
}
