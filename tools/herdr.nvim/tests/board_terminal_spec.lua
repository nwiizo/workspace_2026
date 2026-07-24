local h = require("tests.harness")
local board = require("herdr.board")
local config = require("herdr.config")
local state = require("herdr.state")
local terminal = require("herdr.terminal")

local function agent(id, status, workspace)
  return {
    terminal_id = id,
    target = id,
    status = status,
    workspace_id = workspace or "w1",
    tab_id = "t1",
    pane_id = "p1",
    focused = false,
    revision = 1,
    name = id,
    kind = "codex",
    title = "task",
  }
end

local function reset()
  board._reset()
  terminal._reset()
  state._reset()
  config._reset()
  config.setup({ terminal = { auto_insert = false } })
  state._set_transition_handler(function() end)
  state._set_emit(function() end)
end

h.test("board renders grouped agents in attention order with local mappings", function()
  reset()
  state._accept({
    agents = { agent("idle", "idle"), agent("blocked", "blocked"), agent("work", "working", "w2") },
    workspaces = { { workspace_id = "w1", label = "one" }, { workspace_id = "w2", label = "two" } },
  })
  board.setup()
  board.open()
  local bufnr = board._buffer()
  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  h.eq("Herdr  !1  *1  ✓0", lines[1])
  h.eq("one", lines[3])
  h.contains(lines[4], "blocked")
  h.contains(lines[5], "idle")
  h.eq(4, vim.api.nvim_win_get_cursor(0)[1])
  h.eq(false, vim.wo.wrap)
  for line, item in pairs(board._line_agents()) do
    h.truthy(vim.fn.strdisplaywidth(lines[line]) <= config.get().board.width, item.terminal_id)
  end
  h.truthy(vim.fn.maparg("a", "n", false, true).buffer == 1)
  local global_enter = vim.tbl_filter(function(mapping)
    return mapping.lhs == "<CR>"
  end, vim.api.nvim_get_keymap("n"))
  h.eq({}, global_enter)
  board._reset()
end)

h.test("board renders stale last-good state", function()
  reset()
  state._accept({ agents = { agent("work", "working") }, workspaces = { { workspace_id = "w1", label = "one" } } })
  state._fail({ kind = "process", message = "offline" })
  board.setup()
  board.open()
  local lines = vim.api.nvim_buf_get_lines(board._buffer(), 0, -1, false)
  h.contains(lines[1], "stale")
  h.contains(table.concat(lines, "\n"), "work")
  h.contains(table.concat(lines, "\n"), "offline")
  h.contains(table.concat(lines, "\n"), "Press r to retry")
  board._reset()
end)

h.test("board renders actionable missing-executable guidance", function()
  reset()
  local original_notify = vim.notify
  vim.notify = function() end
  state._fail({ kind = "executable", message = "Herdr ENOENT" }, { stop = true })
  vim.notify = original_notify
  board.setup()
  board.open()
  local text = table.concat(vim.api.nvim_buf_get_lines(board._buffer(), 0, -1, false), "\n")
  h.contains(text, "https://herdr.dev/")
  h.contains(text, ":checkhealth herdr")
  board._reset()
end)

h.test("board starts agents at the editor root captured before opening", function()
  reset()
  local editor = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(editor, vim.fn.getcwd() .. "/lua/example.lua")
  vim.api.nvim_set_current_buf(editor)
  local expected_root = require("herdr.context").project_root(editor)
  local herdr = require("herdr")
  local original_start = herdr.start
  local captured
  herdr.start = function(_, opts)
    captured = opts.cwd
  end
  board.setup()
  board.open()
  local mapping = vim.fn.maparg("a", "n", false, true)
  mapping.callback()
  h.eq(expected_root, captured)
  herdr.start = original_start
  board._reset()
  if vim.api.nvim_buf_is_valid(editor) then
    vim.api.nvim_buf_delete(editor, { force = true })
  end
end)

h.test("terminal uses argv, explicit takeover, and reuses a live buffer", function()
  reset()
  local calls = {}
  terminal._set_termopen(function(command, _)
    table.insert(calls, command)
    return 17
  end)
  local target = agent("term_1", "idle")
  local first = terminal.attach(target)
  local second = terminal.attach(target)
  h.eq(first, second)
  h.eq(1, #calls)
  h.eq({ "herdr", "agent", "attach", "term_1" }, calls[1])
  vim.api.nvim_buf_delete(first, { force = true })

  local third = terminal.attach(target, { takeover = true })
  h.truthy(third ~= nil)
  h.eq({ "herdr", "agent", "attach", "term_1", "--takeover" }, calls[2])
  terminal._reset()
end)

h.test("terminal cleanup stops attach clients without closing Herdr agents", function()
  reset()
  local commands = {}
  local stopped = {}
  terminal._set_termopen(function(command)
    table.insert(commands, command)
    return 20 + #commands
  end)
  terminal._set_jobstop(function(job_id)
    table.insert(stopped, job_id)
    return 1
  end)
  terminal.attach(agent("term_a", "working"))
  terminal.attach(agent("term_b", "working"))
  terminal.cleanup()
  table.sort(stopped)
  h.eq({ 21, 22 }, stopped)
  h.eq({}, terminal._terminals())
  for _, command in ipairs(commands) do
    h.eq("attach", command[3])
    h.truthy(not vim.tbl_contains(command, "close"))
    h.truthy(not vim.tbl_contains(command, "stop"))
  end
  terminal._reset()
end)
