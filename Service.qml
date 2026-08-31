import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
  id: root

  property var shell: null
  property var manifest: null
  property var pluginRegistry: null

  readonly property string pluginId: "io.github.bitshiftxr.atrium"

  property string connectionState: "unconfigured"
  property string statusMessage: ""
  property string origin: ""
  property string haVersion: ""
  property bool plaintext: false

  property var tabs: []
  property var areas: []
  property string activeTab: ""

  property var rows: ({})
  property int rowsRevision: 0

  property var favorites: []
  property var pinnedTabs: []
  property bool allowSensitiveIpc: false

  // Doors, garages and alarms are not a light. Every process running as this
  // user can reach the IPC socket, so these stay panel-only until the user
  // turns them on in config.json.
  readonly property var sensitiveDomains: ["lock", "cover", "alarm_control_panel"]

  function isSensitive(entityId) {
    var dot = String(entityId).indexOf(".")
    if (dot === -1) return true
    return sensitiveDomains.indexOf(String(entityId).substring(0, dot)) !== -1
  }
  property bool showFavorites: false
  property bool importedDashboardPrefs: false
  property string baseUrl: ""
  property bool hideEmptyAreas: true
  property bool showUnassigned: false

  // Newest last. Bounded so a chatty instance cannot grow this without limit.
  property var logEntries: []
  readonly property int logLimit: 200

  property string problem: ""
  property string problemLevel: ""

  function appendLog(payload) {
    var next = logEntries.slice()
    next.push({
      level: String(payload.level || "info"),
      at: Number(payload.at) || 0,
      text: String(payload.text || ""),
      entityId: String(payload.entityId || "")
    })
    while (next.length > logLimit) next.shift()
    logEntries = next

    if (payload.level === "warn" || payload.level === "error") {
      problem = String(payload.text || "")
      problemLevel = String(payload.level)
      problemTimer.restart()
    }
  }

  // An action is in flight until Home Assistant reports the new state. Without
  // this, a device that simply never answers looks identical to a missed click.
  property var pending: ({})
  property int pendingRevision: 0
  readonly property int pendingTimeout: 6000

  function markPending(entityId) {
    pending[entityId] = Date.now()
    pendingRevision++
    pendingSweep.start()
  }

  function clearPending(entityId) {
    if (!Object.prototype.hasOwnProperty.call(pending, entityId)) return
    delete pending[entityId]
    pendingRevision++
  }

  function isPending(entityId) {
    pendingRevision
    return Object.prototype.hasOwnProperty.call(pending, entityId)
  }

  function sweepPending() {
    var now = Date.now()
    var stale = []
    for (var id in pending) {
      if (now - pending[id] >= pendingTimeout) stale.push(id)
    }
    for (var i = 0; i < stale.length; i++) {
      var row = rowFor(stale[i])
      delete pending[stale[i]]
      appendLog({
        level: "warn",
        at: now,
        text: (row ? row.name : stale[i]) + " did not respond.",
        entityId: stale[i]
      })
    }
    if (stale.length > 0) pendingRevision++
    if (Object.keys(pending).length === 0) pendingSweep.stop()
  }

  function clearLog() {
    logEntries = []
    problem = ""
  }

  readonly property bool connected: connectionState === "connected"
  readonly property bool needsSetup: connectionState === "unconfigured"
    || connectionState === "needsToken"
  readonly property bool daemonFailed: bridge.failed
  readonly property string daemonFailureText:
    "atriumd is not running — run setup in the Atrium plugin folder to build it."

  readonly property string daemonPath: {
    var override = Quickshell.env("ATRIUM_DAEMON")
    if (override && override.length > 0) return String(override)
    // Qt.resolvedUrl percent-encodes, so a space in the install path would
    // otherwise reach exec() as %20 and never start.
    return decodeURIComponent(String(Qt.resolvedUrl("bin/atriumd")).replace(/^file:\/\//, ""))
  }

  signal opened()
  signal closeRequested()
  signal toggleRequested()

  function openSettings(section) {
    if (!shell) return
    closeRequested()
    shell.summon(pluginId, JSON.stringify({ section: section || "connection" }))
  }

  function rowFor(entityId) {
    // A plain object inherits toString, constructor and friends; without this
    // guard those names pass an existence check and reach act().
    if (!entityId || !Object.prototype.hasOwnProperty.call(rows, entityId)) return null
    return rows[entityId]
  }

  function tabFor(tabId) {
    for (var i = 0; i < tabs.length; i++) {
      if (tabs[i].id === tabId) return tabs[i]
    }
    return null
  }

  readonly property var activeTabEntities: {
    rowsRevision
    var tab = tabFor(activeTab)
    if (!tab && tabs.length > 0) tab = tabs[0]
    if (!tab) return []
    var ids = tab.entityIds || []
    var out = []
    for (var i = 0; i < ids.length; i++) {
      var row = rows[ids[i]]
      if (row) out.push(row)
    }
    return out
  }

  readonly property var favoriteRows: {
    rowsRevision
    var out = []
    for (var i = 0; i < favorites.length; i++) {
      var row = rows[favorites[i]]
      if (row) out.push(row)
    }
    return out
  }

  readonly property int activeCount: {
    rowsRevision
    var count = 0
    var ids = Object.keys(rows)
    for (var i = 0; i < ids.length; i++) {
      if (rows[ids[i]].active) count++
    }
    return count
  }

  function act(entityId, action, data) {
    markPending(entityId)
    bridge.send({ cmd: "action", entityId: entityId, action: action, data: data || {} })
  }

  function toggle(entityId) {
    var row = rowFor(entityId)
    if (!row) return
    var controls = row.controls || []
    if (controls.indexOf("toggle") !== -1) act(entityId, "toggle")
    else if (controls.indexOf("activate") !== -1) act(entityId, "activate")
    else if (controls.indexOf("lock") !== -1) act(entityId, row.active ? "lock" : "unlock")
    else if (controls.indexOf("openClose") !== -1) act(entityId, row.active ? "close" : "open")
  }

  function setUrl(url) { bridge.send({ cmd: "setUrl", url: url }) }
  function setToken(token, url) {
    bridge.send({ cmd: "setToken", token: token, url: url || baseUrl })
  }
  function forgetToken() { bridge.send({ cmd: "forgetToken" }) }
  function reconnect() { bridge.send({ cmd: "connect" }) }
  function refresh() { bridge.send({ cmd: "refresh" }) }
  function importDashboardPrefs() { bridge.send({ cmd: "importDashboardPrefs" }) }

  function areaOrder() {
    var order = []
    for (var i = 0; i < areas.length; i++) order.push(areas[i].areaId)
    return order
  }

  function hiddenAreas() {
    var hidden = []
    for (var i = 0; i < areas.length; i++) {
      if (areas[i].hidden) hidden.push(areas[i].areaId)
    }
    return hidden
  }

  function sendAreaPrefs(order, hidden, hideEmpty, unassigned) {
    var command = {
      cmd: "setAreaPrefs",
      hideEmptyAreas: hideEmpty,
      hideEntitiesWithoutArea: !unassigned
    }
    // Before the first connection there are no areas to derive these from, and
    // sending empty lists would erase a hand-built room order.
    if (areas.length > 0) {
      command.order = order
      command.hidden = hidden
    }
    bridge.send(command)
  }

  function toggleAreaHidden(areaId) {
    var hidden = hiddenAreas()
    var at = hidden.indexOf(areaId)
    if (at === -1) hidden.push(areaId)
    else hidden.splice(at, 1)
    sendAreaPrefs(areaOrder(), hidden, hideEmptyAreas, showUnassigned)
  }

  function moveArea(areaId, delta) {
    var order = areaOrder()
    var from = order.indexOf(areaId)
    var to = from + delta
    if (from === -1 || to < 0 || to >= order.length) return
    order.splice(to, 0, order.splice(from, 1)[0])
    sendAreaPrefs(order, hiddenAreas(), hideEmptyAreas, showUnassigned)
  }

  function setHideEmptyAreas(value) {
    sendAreaPrefs(areaOrder(), hiddenAreas(), value, showUnassigned)
  }

  function setShowUnassigned(value) {
    sendAreaPrefs(areaOrder(), hiddenAreas(), hideEmptyAreas, value)
  }

  function showAllAreas() {
    sendAreaPrefs(areaOrder(), [], false, showUnassigned)
  }

  function isPinnedTab(tabId) {
    return pinnedTabs.indexOf(tabId) !== -1
  }

  function togglePinnedTab(tabId) {
    var next = pinnedTabs.slice()
    var at = next.indexOf(tabId)
    if (at === -1) next.push(tabId)
    else next.splice(at, 1)
    bridge.send({ cmd: "setPinnedTabs", ids: next })
  }

  readonly property var pinnedTabList: {
    var out = []
    for (var i = 0; i < tabs.length; i++) {
      if (isPinnedTab(tabs[i].id)) out.push(tabs[i])
    }
    return out
  }

  readonly property var tabOptions: {
    var out = []
    for (var i = 0; i < tabs.length; i++) {
      out.push({ value: tabs[i].id, label: tabs[i].title })
    }
    return out
  }

  function toggleFavorite(entityId) {
    var next = favorites.slice()
    var at = next.indexOf(entityId)
    if (at === -1) next.push(entityId)
    else next.splice(at, 1)
    setFavorites(next, showFavorites)
  }

  function isFavorite(entityId) {
    return favorites.indexOf(entityId) !== -1
  }

  function setFavorites(ids, show) {
    bridge.send({ cmd: "setFavorites", ids: ids, show: show })
  }

  function selectTab(tabId) {
    if (activeTab === tabId) return
    activeTab = tabId
    bridge.send({ cmd: "setSelectedTab", tab: tabId })
  }

  function applyStatus(payload) {
    connectionState = String(payload.state || "offline")
    statusMessage = String(payload.message || "")
    origin = String(payload.origin || "")
    haVersion = String(payload.haVersion || "")
    plaintext = payload.plaintext === true
  }

  function applyRows(list) {
    var next = {}
    for (var i = 0; i < list.length; i++) next[list[i].entityId] = list[i]
    rows = next
    rowsRevision++
  }

  // Mutating in place and bumping the revision keeps a busy instance off the
  // O(entities) copy that a fresh map on every state change would cost.
  function applyRow(row) {
    clearPending(row.entityId)
    rows[row.entityId] = row
    rowsRevision++
  }

  function dropRow(entityId) {
    delete rows[entityId]
    rowsRevision++
  }

  function applyTabs(list) {
    tabs = list
    if (list.length === 0) {
      activeTab = ""
      return
    }
    if (!tabFor(activeTab)) activeTab = list[0].id
  }

  function handle(payload) {
    switch (payload.ev) {
    case "status": applyStatus(payload); break
    case "tabs": applyTabs(payload.tabs || []); break
    case "areas": areas = payload.areas || []; break
    case "rows": applyRows(payload.rows || []); break
    case "row": applyRow(payload.row); break
    case "dropped": dropRow(payload.entityId); break
    case "config":
      favorites = payload.favorites || []
      pinnedTabs = payload.pinnedTabs || []
      allowSensitiveIpc = payload.allowSensitiveIpc === true
      showFavorites = payload.showFavorites === true
      importedDashboardPrefs = payload.importedDashboardPrefs === true
      baseUrl = String(payload.baseUrl || "")
      hideEmptyAreas = payload.hideEmptyAreas === true
      showUnassigned = payload.hideEntitiesWithoutArea === false
      if (payload.selectedTab && !activeTab) activeTab = String(payload.selectedTab)
      break
    case "log": appendLog(payload); break
    }
  }

  property Timer pendingSweep: Timer {
    id: pendingSweep
    interval: 1000
    repeat: true
    onTriggered: root.sweepPending()
  }

  property Timer problemTimer: Timer {
    id: problemTimer
    interval: 12000
    repeat: false
    onTriggered: root.problem = ""
  }

  property Bridge bridge: Bridge {
    daemonPath: root.daemonPath
    onEvent: function(payload) { root.handle(payload) }
  }

  // Unlike a daemon-reported problem, this one does not resolve on its own, so
  // it outlives problemTimer rather than clearing twelve seconds later.
  property Connections daemonFailure: Connections {
    target: root.bridge
    function onFailedChanged() {
      if (root.bridge.failed) {
        problemTimer.stop()
        root.problem = root.daemonFailureText
        root.problemLevel = "error"
      } else if (root.problem === root.daemonFailureText) {
        root.problem = ""
        root.problemLevel = ""
      }
    }
  }

  property IpcHandler ipc: IpcHandler {
    target: "atrium"

    function status(): string {
      return JSON.stringify({
        state: root.connectionState,
        origin: root.origin,
        haVersion: root.haVersion,
        tabs: root.tabs.length,
        entities: Object.keys(root.rows).length,
        active: root.activeCount
      })
    }

    function toggleEntity(entityId: string): string {
      if (!root.rowFor(entityId)) return "unknown entity " + entityId
      if (root.isSensitive(entityId) && !root.allowSensitiveIpc) {
        return "refused: " + entityId + " is only actionable from the panel."
          + " Set allowSensitiveIpc in ~/.config/atrium/config.json to allow it."
      }
      root.toggle(entityId)
      return "ok"
    }

    function activate(entityId: string): string {
      if (!root.rowFor(entityId)) return "unknown entity " + entityId
      if (root.isSensitive(entityId) && !root.allowSensitiveIpc) {
        return "refused: " + entityId + " is only actionable from the panel."
      }
      root.act(entityId, "activate")
      return "ok"
    }

    function areas(): string {
      var names = []
      for (var i = 0; i < root.tabs.length; i++) names.push(root.tabs[i].title)
      return names.join("\n")
    }

    function settings(): string {
      root.openSettings("connection")
      return "ok"
    }

    function rooms(): string {
      root.openSettings("areas")
      return "ok"
    }

    function open(): string {
      root.opened()
      return "ok"
    }

    function close(): string {
      root.closeRequested()
      return "ok"
    }

    function toggle(): string {
      root.toggleRequested()
      return "ok"
    }

    function refresh(): string {
      root.refresh()
      return "ok"
    }
  }

  Component.onCompleted: bridge.start()
}
