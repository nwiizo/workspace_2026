local h = require("tests.harness")
local config = require("herdr.config")
local state = require("herdr.state")

local function agent(id, status, workspace, revision)
  return {
    terminal_id = id,
    target = id,
    status = status,
    workspace_id = workspace or "w1",
    tab_id = "t1",
    pane_id = "p1",
    focused = false,
    revision = revision or 1,
    name = id,
    kind = "codex",
    title = "",
  }
end

local function snapshot(agents)
  return {
    agents = agents,
    workspaces = {
      { workspace_id = "w1", label = "one" },
      { workspace_id = "w2", label = "two" },
    },
  }
end

local function reset()
  config._reset()
  config.setup()
  state._reset()
end

h.test("state bootstraps without a transition notification", function()
  reset()
  local transitions = {}
  local events = {}
  state._set_transition_handler(function(event)
    table.insert(transitions, event)
  end)
  state._set_emit(function(pattern)
    table.insert(events, pattern)
  end)
  state._accept(snapshot({ agent("a", "blocked") }))
  h.eq({}, transitions)
  h.eq({ "HerdrUpdated" }, events)
end)

h.test("state emits transitions for changes and newly observed attention states", function()
  reset()
  local transitions = {}
  state._set_transition_handler(function(event)
    table.insert(transitions, event)
  end)
  state._set_emit(function() end)
  state._accept(snapshot({ agent("a", "working", "w1", 1) }))
  state._accept(snapshot({ agent("a", "blocked", "w1", 2), agent("b", "done", "w1", 1) }))
  h.eq(2, #transitions)
  h.eq("working", transitions[1].previous_status)
  h.eq("blocked", transitions[1].status)
  h.eq(nil, transitions[2].previous_status)
  h.eq("done", transitions[2].status)
end)

h.test("state does not notify for newly observed non-attention states", function()
  reset()
  local transitions = {}
  state._set_transition_handler(function(event)
    table.insert(transitions, event)
  end)
  state._set_emit(function() end)
  state._accept(snapshot({ agent("a", "working") }))
  state._accept(snapshot({ agent("a", "working"), agent("b", "idle"), agent("c", "unknown") }))
  h.eq({}, transitions)
end)

h.test("state groups by workspace and sorts by attention priority", function()
  reset()
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
  state._accept(snapshot({
    agent("idle", "idle", "w1"),
    agent("done", "done", "w1"),
    agent("blocked", "blocked", "w1"),
    agent("working", "working", "w2"),
  }))
  local groups = state.grouped_agents()
  h.eq("one", groups[1].label)
  h.eq(
    { "blocked", "done", "idle" },
    vim.tbl_map(function(item)
      return item.name
    end, groups[1].agents)
  )
  h.eq("working", groups[2].agents[1].name)
  h.eq({ blocked = 1, working = 1, done = 1, idle = 1, unknown = 0 }, state.counts())
end)

h.test("state preserves Herdr order when agents have equal priority", function()
  reset()
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
  state._accept(snapshot({
    agent("first", "working", "w1"),
    agent("second", "working", "w1"),
    agent("third", "working", "w1"),
  }))
  h.eq(
    { "first", "second", "third" },
    vim.tbl_map(function(item)
      return item.name
    end, state.grouped_agents()[1].agents)
  )
end)

h.test("state retains the last good snapshot and marks it stale on failure", function()
  reset()
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
  state._accept(snapshot({ agent("a", "working") }))
  state._fail({ kind = "process", message = "offline" })
  h.eq("a", state.agents()[1].name)
  h.truthy(state.get().stale)
  h.eq("offline", state.get().error.message)
end)

h.test("state notifies only once when the Herdr executable is missing", function()
  reset()
  state._set_emit(function() end)
  local original_notify = vim.notify
  local notifications = {}
  vim.notify = function(text)
    table.insert(notifications, text)
  end
  state._fail({ kind = "executable", message = "ENOENT" }, { stop = true })
  state._fail({ kind = "executable", message = "ENOENT" }, { stop = true })
  vim.notify = original_notify
  h.eq(1, #notifications)
  h.contains(notifications[1], "https://herdr.dev/")
end)

h.test("state keeps polling after a transient snapshot failure and recovers", function()
  reset()
  local client = require("herdr.client")
  local original_ensure = client.ensure_server
  local original_snapshot = client.snapshot
  local snapshot_calls = 0
  client.ensure_server = function(callback)
    callback(true)
  end
  client.snapshot = function(callback)
    snapshot_calls = snapshot_calls + 1
    if snapshot_calls == 1 then
      callback(nil, { kind = "process", message = "temporary timeout" })
    else
      callback(snapshot({ agent("recovered", "working") }))
    end
  end

  local timer_starts = 0
  local fake_timer = {
    is_closing = function()
      return false
    end,
    start = function()
      timer_starts = timer_starts + 1
    end,
    stop = function() end,
    close = function() end,
  }
  state._set_timer_factory(function()
    return fake_timer
  end)
  state.start()
  h.truthy(state.get().polling)
  h.truthy(state.get().stale == false)
  h.eq("temporary timeout", state.get().error.message)
  h.eq(1, timer_starts)

  state.refresh()
  h.eq("recovered", state.agents()[1].name)
  h.eq(false, state.get().stale)
  h.truthy(state.get().polling)
  state.stop()
  client.ensure_server = original_ensure
  client.snapshot = original_snapshot
end)

h.test("state ignores callbacks from a reset lifecycle", function()
  reset()
  local client = require("herdr.client")
  local original_ensure = client.ensure_server
  local original_snapshot = client.snapshot
  local ensure_callback
  local snapshot_called = false
  client.ensure_server = function(callback)
    ensure_callback = callback
  end
  client.snapshot = function()
    snapshot_called = true
  end

  state.start()
  state._reset()
  ensure_callback(true)
  h.eq(false, snapshot_called)
  h.eq(false, state.get().polling)
  h.eq(nil, state.get().snapshot)
  client.ensure_server = original_ensure
  client.snapshot = original_snapshot
end)

h.test("state switches polling interval while the board is visible", function()
  reset()
  local client = require("herdr.client")
  local original_ensure = client.ensure_server
  local original_snapshot = client.snapshot
  client.ensure_server = function(callback)
    callback(true)
  end
  client.snapshot = function(callback)
    callback(snapshot({}))
  end

  local starts = {}
  local fake_timer = {
    closing = false,
    is_closing = function(self)
      return self.closing
    end,
    start = function(_, timeout, repeat_interval)
      table.insert(starts, { timeout, repeat_interval })
    end,
    stop = function() end,
    close = function(self)
      self.closing = true
    end,
  }
  state._set_timer_factory(function()
    return fake_timer
  end)
  state.start()
  state.set_board_visible(true)
  h.eq({ { 5000, 5000 }, { 1000, 1000 } }, starts)
  state.stop()
  client.ensure_server = original_ensure
  client.snapshot = original_snapshot
end)
