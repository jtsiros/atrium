import QtQuick
import qs.Ui
import qs.Commons
import "controls" as Controls
import "Glyphs.js" as Glyphs

Item {
  id: root

  property QtObject bar: null
  property QtObject service: null
  property var row: null
  property bool hasCursor: false
  property bool expanded: false
  property bool pending: false

  // Both taken from the room box's rendered edges, so a row spans exactly the
  // same column as the box above it.
  property real leadingInset: 0
  property real trailingInset: Style.spacing.rowPaddingX

  readonly property var controls: row && row.controls ? row.controls : []
  readonly property bool readOnly: controls.length === 1 && controls[0] === "readOnly"
  readonly property bool switchable: controls.indexOf("toggle") !== -1
    || controls.indexOf("lock") !== -1
  readonly property bool expandable: !readOnly && controls.length > (switchable ? 1 : 0)
  readonly property bool hot: hasCursor || hover.hovered

  signal activated()
  signal toggleExpanded()

  implicitHeight: line.height + (expanded ? expansion.height : 0)

  Rectangle {
    id: line
    width: parent.width
    height: Style.spacing.controlHeight
    radius: Style.cornerRadius
    color: root.hot ? Style.hoverFill : "transparent"
    border.width: root.hot ? 1 : 0
    border.color: Style.hoverBorderColor

    HoverHandler { id: hover }

    MouseArea {
      anchors.fill: parent
      cursorShape: root.readOnly ? Qt.ArrowCursor : Qt.PointingHandCursor
      onClicked: root.activated()
      onDoubleClicked: if (root.expandable) root.toggleExpanded()
    }

    Glyph {
      id: rowIcon
      anchors.left: parent.left
      anchors.leftMargin: root.leadingInset
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.row ? root.row.glyph : ""
      color: root.row && root.row.active ? Color.accent : Color.muted
      opacity: root.row && root.row.unavailable ? 0.45 : 1
    }

    // Anchored to the affordance rather than sized by arithmetic: a long name
    // must elide against whatever is actually to its right, not against a
    // width computed before that has been laid out.
    Text {
      anchors.left: rowIcon.right
      anchors.leftMargin: Style.spacing.xl
      anchors.right: affordance.left
      anchors.rightMargin: Style.spacing.lg
      anchors.verticalCenter: parent.verticalCenter
      text: root.row ? root.row.name : ""
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.body
      color: Color.foreground
      opacity: root.row && root.row.unavailable ? 0.55 : 1
      elide: Text.ElideRight
    }

    Row {
      id: affordance
      anchors.right: parent.right
      anchors.rightMargin: root.trailingInset + Style.font.iconSmall + Style.spacing.lg
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.lg

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: root.row ? root.row.displayState : ""
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
        color: Color.muted
        visible: !root.switchable
      }

      Rectangle {
        anchors.verticalCenter: parent.verticalCenter
        visible: root.switchable
        width: Style.space(26)
        height: Style.space(14)
        radius: Style.cornerRadius
        color: "transparent"
        border.width: 1
        border.color: root.row && root.row.active ? Color.accent : Style.normalBorderColor

        Rectangle {
          width: Style.space(10)
          height: Style.space(10)
          radius: Style.cornerRadius
          anchors.verticalCenter: parent.verticalCenter
          anchors.left: root.row && root.row.active ? undefined : parent.left
          anchors.right: root.row && root.row.active ? parent.right : undefined
          anchors.leftMargin: 1
          anchors.rightMargin: 1
          color: root.row && root.row.active ? Color.accent : Color.muted

          SequentialAnimation on opacity {
            running: root.pending
            loops: Animation.Infinite
            NumberAnimation { to: 0.3; duration: 400 }
            NumberAnimation { to: 1.0; duration: 400 }
          }
        }

        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: root.activated()
        }
      }
    }

    Item {
      id: chevronSlot
      anchors.right: parent.right
      anchors.rightMargin: root.trailingInset
      anchors.verticalCenter: parent.verticalCenter
      width: Style.font.iconSmall
      height: Style.font.iconSmall

      Text {
        anchors.centerIn: parent
        visible: root.expandable
        text: root.expanded ? Glyphs.chevronUp : Glyphs.chevronDown
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.iconSmall
        color: Color.muted
      }

      MouseArea {
        anchors.fill: parent
        anchors.margins: -Style.spacing.sm
        enabled: root.expandable
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggleExpanded()
      }
    }
  }

  Rectangle {
    id: expansion
    anchors.top: line.bottom
    width: parent.width
    height: root.expanded ? expandedControls.implicitHeight + Style.spacing.xxl * 2 : 0
    visible: root.expanded
    color: Style.normalFill
    radius: Style.cornerRadius

    Loader {
      id: expandedControls
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      anchors.margins: Style.spacing.xxl
      anchors.leftMargin: root.leadingInset
      anchors.rightMargin: root.trailingInset
      active: root.expanded
      sourceComponent: {
        if (!root.row) return null
        switch (root.row.domain) {
        case "light": return lightControls
        case "climate": return climateControls
        case "media_player": return mediaControls
        case "cover": return coverControls
        case "fan": return fanControls
        }
        return null
      }
    }
  }

  Component {
    id: lightControls
    Controls.LightControls { bar: root.bar; service: root.service; row: root.row }
  }

  Component {
    id: climateControls
    Controls.ClimateControls { bar: root.bar; service: root.service; row: root.row }
  }

  Component {
    id: mediaControls
    Controls.MediaControls { bar: root.bar; service: root.service; row: root.row }
  }

  Component {
    id: coverControls
    Controls.CoverControls { bar: root.bar; service: root.service; row: root.row }
  }

  Component {
    id: fanControls
    Controls.FanControls { bar: root.bar; service: root.service; row: root.row }
  }
}
