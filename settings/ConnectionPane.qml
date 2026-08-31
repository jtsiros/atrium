import QtQuick
import qs.Ui
import qs.Commons
import "../Glyphs.js" as Glyphs

Item {
  id: root

  property QtObject service: null

  readonly property string state: service ? service.connectionState : "unconfigured"
  readonly property bool connected: service ? service.connected : false

  function applyConnection() {
    if (!service) return
    if (tokenField.text !== "") {
      service.setToken(tokenField.text, urlField.text)
      tokenField.text = ""
      return
    }
    service.setUrl(urlField.text)
  }

  Column {
    anchors.fill: parent
    anchors.margins: Style.spacing.panelPadding
    spacing: Style.spacing.xxxl

    Text {
      text: "Connection"
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.heading
      color: Color.foreground
    }

    Column {
      width: parent.width
      spacing: Style.spacing.sm

      Text {
        text: "HOME ASSISTANT URL"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        color: Color.muted
      }

      TextField {
        id: urlField
        width: parent.width
        text: root.service ? root.service.baseUrl : ""
        placeholderText: "https://homeassistant.local:8123"
        onAccepted: root.service.setUrl(text)
      }

    }

    Column {
      width: parent.width
      spacing: Style.spacing.sm

      Text {
        text: "LONG-LIVED ACCESS TOKEN"
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        color: Color.muted
      }

      TextField {
        id: tokenField
        width: parent.width
        password: true
        placeholderText: root.connected ? "Stored in your keyring" : "Paste a token"
        onAccepted: root.applyConnection()
      }

      Text {
        width: parent.width
        text: "Home Assistant → your profile → Security → Long-lived access tokens."
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        color: Color.muted
        wrapMode: Text.WordWrap
      }
    }

    Rectangle {
      width: parent.width
      height: statusText.implicitHeight + Style.spacing.xl * 2
      color: Style.normalFill
      radius: Style.cornerRadius

      Rectangle {
        width: Math.max(1, Style.space(2))
        height: parent.height
        color: root.connected ? Color.accent : Color.muted
      }

      Text {
        id: statusText
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Style.spacing.xxl
        anchors.rightMargin: Style.spacing.xxl
        anchors.verticalCenter: parent.verticalCenter
        textFormat: Text.PlainText
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
        color: Color.foreground
        wrapMode: Text.WordWrap
        text: {
          if (!root.service) return "Starting"
          if (root.service.daemonFailed) return root.service.daemonFailureText
          if (root.connected) {
            return "Connected · Home Assistant " + root.service.haVersion
              + " · " + root.service.areas.length + " areas"
          }
          if (root.service.statusMessage !== "") return root.service.statusMessage
          switch (root.state) {
          case "unconfigured": return "Set your Home Assistant address to begin."
          case "needsToken": return "Paste an access token for " + root.service.origin + "."
          case "connecting": return "Connecting…"
          case "reconnecting": return "Reconnecting…"
          }
          return "Not connected"
        }
      }
    }

    Text {
      width: parent.width
      visible: root.service ? root.service.plaintext : false
      text: "This address sends your token without encryption. Use https unless this is a network you trust."
      textFormat: Text.PlainText
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
      color: Color.urgent
      wrapMode: Text.WordWrap
    }

    Row {
      spacing: Style.spacing.lg

      Button {
        text: "Connect"
        bordered: true
        onClicked: root.applyConnection()
      }

      Button {
        text: "Reconnect"
        onClicked: root.service.reconnect()
      }

      Button {
        text: "Forget token"
        onClicked: root.service.forgetToken()
      }
    }

  }
}
