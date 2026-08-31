import QtQuick
import Quickshell.Io

QtObject {
  id: root

  property string daemonPath: ""
  property bool started: false
  property bool failed: false
  property string lastError: ""

  // A run that never says anything is a run that did not work. Spawning
  // succeeds for a binary that then dies on startup, so onStarted alone proves
  // nothing; the first event does.
  property bool sawEvent: false

  signal event(var payload)

  property var pending: []

  function start() {
    if (proc.running || daemonPath === "") return
    proc.command = [daemonPath, "serve"]
    proc.running = true
    sawEvent = false
    startWatchdog.restart()
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
    if (!root.sawEvent) {
      root.sawEvent = true
      root.failed = false
    }
    root.event(payload)
  }

  property Process proc: Process {
    id: proc
    stdinEnabled: true
    onStarted: {
      startWatchdog.stop()
      root.started = true
      root.flush()
    }
    onExited: function(code, status) {
      root.started = false
      if (code !== 0) root.lastError = "atriumd exited with status " + code
      if (!root.sawEvent) root.failed = true
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

  // A binary that is not there emits neither started nor exited — Process just
  // drops running back to false — so onExited never arms restartTimer and
  // nothing else here would ever look again. This timer is both halves: it
  // reports the failure, and it keeps trying, so building the daemon while the
  // shell is up fixes the panel without a reload. Spawning is local and takes
  // milliseconds, so silence this long means it did not start.
  property Timer startWatchdog: Timer {
    id: startWatchdog
    interval: 5000
    repeat: true
    onTriggered: {
      root.failed = true
      root.start()
    }
  }
}
