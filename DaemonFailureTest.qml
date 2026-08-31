import QtQuick
import Quickshell

ShellRoot {
  id: harness

  readonly property string mode: Quickshell.env("ATRIUM_TEST_EXPECT") || "ok"
  readonly property bool expectFailure: mode !== "ok"
  property var service: null
  property int failures: 0

  function check(name, actual, expected) {
    var ok = actual === expected
    if (!ok) harness.failures++
    console.log((ok ? "  pass  " : "  FAIL  ") + name
      + (ok ? "" : "  expected=" + JSON.stringify(expected)
        + "  actual=" + JSON.stringify(actual)))
  }

  function finish() {
    console.log(harness.failures === 0 ? "RESULT pass" : "RESULT FAIL")
    Qt.exit(harness.failures === 0 ? 0 : 1)
  }

  Component.onCompleted: {
    var component = Qt.createComponent(Qt.resolvedUrl("Service.qml"))
    if (component.status !== Component.Ready) {
      console.log("  FAIL  could not load Service.qml: " + component.errorString())
      harness.failures++
      harness.finish()
      return
    }
    harness.service = component.createObject(harness)
  }

  Timer {
    interval: 9000
    running: true
    onTriggered: {
      harness.check("daemonFailed", harness.service.daemonFailed, harness.expectFailure)
      harness.check("problem", harness.service.problem,
        harness.expectFailure ? harness.service.daemonFailureText : "")
      harness.check("problemLevel", harness.service.problemLevel,
        harness.expectFailure ? "error" : "")
      if (harness.mode === "ok") harness.finish()
    }
  }

  // problemTimer clears a daemon-reported problem after 12s, so both remaining
  // checks have to happen on the far side of it. In recovery the runner drops a
  // working binary in partway through this window.
  Timer {
    interval: 22000
    running: harness.mode !== "ok"
    onTriggered: {
      if (harness.mode === "recovery") {
        harness.check("daemonFailed cleared", harness.service.daemonFailed, false)
        harness.check("banner cleared", harness.service.problem, "")
      } else {
        harness.check("problem survives problemTimer",
          harness.service.problem, harness.service.daemonFailureText)
      }
      harness.finish()
    }
  }
}
