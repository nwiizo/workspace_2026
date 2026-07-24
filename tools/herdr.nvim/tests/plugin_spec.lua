local h = require("tests.harness")

h.test("plugin registers the approved user commands", function()
  vim.g.loaded_herdr_nvim = nil
  vim.cmd("runtime plugin/herdr.lua")
  for _, command in ipairs({
    "Herdr",
    "HerdrRefresh",
    "HerdrStart",
    "HerdrAttach",
    "HerdrSend",
    "HerdrSendVisual",
    "HerdrSendFile",
    "HerdrSendDiagnostics",
    "HerdrHealth",
  }) do
    h.eq(2, vim.fn.exists(":" .. command), command .. " should exist")
  end
  local commands = vim.api.nvim_get_commands({})
  h.eq(true, commands.HerdrAttach.bang)
  h.truthy(commands.HerdrSend.range ~= nil)
  h.truthy(commands.HerdrSendVisual.range ~= nil)
end)

h.test("line and visual send commands use separate unambiguous paths", function()
  local herdr = require("herdr")
  local original_range = herdr._send_range
  local original_visual = herdr._send_visual
  local calls = {}
  herdr._send_range = function(target, _, line1, line2)
    table.insert(calls, { kind = "range", target = target, line1 = line1, line2 = line2 })
  end
  herdr._send_visual = function(target, _, line1, line2)
    table.insert(calls, { kind = "visual", target = target, line1 = line1, line2 = line2 })
  end
  vim.cmd("1,1HerdrSend term_1")
  vim.cmd("1,1HerdrSendVisual term_1")
  herdr._send_range = original_range
  herdr._send_visual = original_visual
  h.eq({
    { kind = "range", target = "term_1", line1 = 1, line2 = 1 },
    { kind = "visual", target = "term_1", line1 = 1, line2 = 1 },
  }, calls)
end)

local function agent(id, name, status)
  return {
    terminal_id = id,
    target = id,
    pane_id = id .. ":pane",
    status = status or "working",
    workspace_id = "w1",
    tab_id = "t1",
    focused = false,
    revision = 1,
    name = name,
    kind = "codex",
    title = "",
    cwd = vim.fn.getcwd(),
  }
end

h.test("agent command targets use stable IDs and reject ambiguous names", function()
  local herdr = require("herdr")
  local state = require("herdr.state")
  state._reset()
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
  state._accept({
    agents = { agent("term_1", "worker"), agent("term_2", "worker") },
    workspaces = { { workspace_id = "w1", label = "repo" } },
  })
  h.eq({ "term_1", "term_2" }, herdr._complete_agents())
  h.eq("term_2", herdr._find_agent("term_2").terminal_id)
  local resolved, err = herdr._find_agent("worker")
  h.eq(nil, resolved)
  h.contains(err, "ambiguous")
end)

h.test("statusline is healthy, stale, and empty when Herdr is missing", function()
  local herdr = require("herdr")
  local state = require("herdr.state")
  local config = require("herdr.config")
  herdr._reset()
  state._reset()
  config._reset()
  local original_start = state.start
  local original_stop = state.stop
  state.start = function() end
  state.stop = function() end
  herdr.setup()
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
  state._accept({
    agents = { agent("term_1", "worker", "blocked") },
    workspaces = { { workspace_id = "w1", label = "repo" } },
  })
  h.eq("H !1", herdr.statusline())
  state._fail({ kind = "process", message = "offline" })
  h.eq("H !1 ~", herdr.statusline())
  config.setup({ herdr_cmd = "definitely-not-a-real-herdr-command" })
  h.eq("", herdr.statusline())
  state.start = original_start
  state.stop = original_stop
  herdr._reset()
  state._reset()
  config._reset()
end)

h.test("invalid reconfiguration does not stop a working instance", function()
  local herdr = require("herdr")
  local state = require("herdr.state")
  local config = require("herdr.config")
  herdr._reset()
  state._reset()
  config._reset()
  local stops = 0
  local original_start = state.start
  local original_stop = state.stop
  state.start = function() end
  state.stop = function()
    stops = stops + 1
  end
  herdr.setup({ board = { width = 50 } })
  h.raises("refresh.board_ms", function()
    herdr.setup({ refresh = { board_ms = 0 } })
  end)
  h.eq(0, stops)
  h.eq(50, config.get().board.width)
  state.start = original_start
  state.stop = original_stop
  herdr._reset()
  state._reset()
  config._reset()
end)
