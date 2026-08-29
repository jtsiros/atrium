import QtQuick
import Quickshell.Io

QtObject {
  id: root

  property string daemonPath: ""
  property bool started: false
  property string lastError: ""

  signal event(var payload)

  property var pending: []

  function start() {
    if (proc.running || daemonPath === "") return
    proc.command = [daemonPath, "serve"]
    proc.running = true
  }

  function stop() {
    pending = []
    started = false
    proc.running = false
  }

  function send(command) {
    var line = JSON.stringify(command)
    if (!started) {
      var queue = pending.slice()
      queue.push(line)
      pending = queue
      start()
      return
    }
    proc.write(line + "\n")
  }

  function flush() {
    var queue = pending
    pending = []
    for (var i = 0; i < queue.length; i++) proc.write(queue[i] + "\n")
  }

  function receive(line) {
    if (line === "") return
    var payload
    try {
      payload = JSON.parse(line)
    } catch (e) {
      root.lastError = "unreadable line from atriumd"
      return
    }
    if (!payload || typeof payload !== "object") return
    root.event(payload)
  }

  property Process proc: Process {
    id: proc
    stdinEnabled: true
    onStarted: {
      root.started = true
      root.flush()
    }
    onExited: function(code, status) {
      root.started = false
      if (code !== 0) root.lastError = "atriumd exited with status " + code
      restartTimer.restart()
    }
    stdout: SplitParser { onRead: function(line) { root.receive(line) } }
    stderr: SplitParser { onRead: function(line) { root.lastError = line } }
  }

  property Timer restartTimer: Timer {
    id: restartTimer
    interval: 2000
    repeat: false
    onTriggered: root.start()
  }
}
