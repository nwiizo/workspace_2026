local h = require("tests.harness")
local config = require("herdr.config")
local client = require("herdr.client")

local function reset()
  config._reset()
  client._reset()
end

h.test("config deep merges defaults without mutating caller options", function()
  reset()
  local opts = { board = { width = 52 }, agents = { codex = { command = { "my-codex" } } } }
  local result = config.setup(opts)
  h.eq(52, result.board.width)
  h.eq("left", result.board.side)
  h.eq({ "my-codex" }, result.agents.codex.command)
  h.eq({ "claude" }, result.agents.claude.command)
  result.agents.codex.command[1] = "changed"
  h.eq("my-codex", opts.agents.codex.command[1])
end)

h.test("config rejects invalid values with a precise path", function()
  reset()
  h.raises("refresh.board_ms", function()
    config.setup({ refresh = { board_ms = 0 } })
  end)
  h.raises("terminal.side", function()
    config.setup({ terminal = { side = "center" } })
  end)
  h.raises("agents.codex.command", function()
    config.setup({ agents = { codex = { command = {} } } })
  end)
  h.raises("refresh must be a table", function()
    config.setup({ refresh = false })
  end)
  h.raises("board.markers must be a table", function()
    config.setup({ board = { markers = false } })
  end)
end)

h.test("config publishes only fully validated updates", function()
  reset()
  config.setup({ board = { width = 52 } })
  h.raises("refresh.board_ms", function()
    config.setup({ refresh = { board_ms = 0 } })
  end)
  h.eq(52, config.get().board.width)
  h.eq(1000, config.get().refresh.board_ms)
end)

local function raw_snapshot(agent)
  return {
    type = "session_snapshot",
    snapshot = {
      version = "0.7.4",
      protocol = 16,
      agents = { agent },
      workspaces = { { workspace_id = "w1", label = "repo", focused = true, extra = "ignored" } },
      extra = "ignored",
    },
  }
end

local function raw_agent(overrides)
  return vim.tbl_extend("force", {
    terminal_id = "term_1",
    agent_status = "working",
    workspace_id = "w1",
    tab_id = "w1:t1",
    pane_id = "w1:p1",
    focused = false,
    revision = 3,
    name = "api",
    agent = "codex",
    cwd = "/repo",
    unknown_future_field = true,
  }, overrides or {})
end

h.test("client normalizes documented snapshot fields and ignores unknown fields", function()
  reset()
  config.setup()
  local result, err = client._normalize_snapshot(raw_snapshot(raw_agent()))
  h.eq(nil, err)
  h.eq("term_1", result.agents[1].terminal_id)
  h.eq("working", result.agents[1].status)
  h.eq("api", result.agents[1].name)
  h.eq("repo", result.workspaces[1].label)
end)

h.test("client maps future statuses to unknown but rejects missing required fields", function()
  reset()
  config.setup()
  local future = client._normalize_snapshot(raw_snapshot(raw_agent({ agent_status = "paused" })))
  h.eq("unknown", future.agents[1].status)

  local result, err = client._normalize_snapshot(raw_snapshot(raw_agent({ terminal_id = false })))
  h.eq(nil, result)
  h.contains(err.message, "terminal_id")
end)

h.test("client normalizes nullable display fields before applying fallbacks", function()
  reset()
  config.setup()
  local result = client._normalize_snapshot(raw_snapshot(raw_agent({
    name = vim.NIL,
    display_agent = vim.NIL,
    title = vim.NIL,
    foreground_cwd = vim.NIL,
    cwd = "/repo",
  })))
  h.eq("codex", result.agents[1].name)
  h.eq("", result.agents[1].title)
  h.eq("/repo", result.agents[1].cwd)

  local with_null_label = raw_snapshot(raw_agent())
  with_null_label.snapshot.workspaces[1].label = vim.NIL
  result = client._normalize_snapshot(with_null_label)
  h.eq("w1", result.workspaces[1].label)
end)

h.test("client rejects object-shaped lists and malformed workspaces", function()
  reset()
  config.setup()
  local invalid = raw_snapshot(raw_agent())
  invalid.snapshot.agents = { named = raw_agent() }
  local result, err = client._normalize_snapshot(invalid)
  h.eq(nil, result)
  h.eq("schema", err.kind)

  invalid = raw_snapshot(raw_agent())
  invalid.snapshot.workspaces = { { label = "missing id" } }
  result, err = client._normalize_snapshot(invalid)
  h.eq(nil, result)
  h.contains(err.message, "workspace_id")
end)

h.test("client uses argv arrays for snapshot, start, and send", function()
  reset()
  config.setup()
  local calls = {}
  client._set_runner(function(argv, _, callback)
    table.insert(calls, vim.deepcopy(argv))
    local payload
    if argv[2] == "api" then
      payload = { id = "1", result = raw_snapshot(raw_agent()) }
    else
      payload = { id = "1", result = { type = "ok" } }
    end
    callback({ code = 0, stdout = vim.json.encode(payload), stderr = "" })
  end)

  client.snapshot(function(result)
    h.eq("term_1", result.agents[1].terminal_id)
  end)
  client.start_agent("codex", "worker", "/repo", function(result)
    h.eq("ok", result.type)
  end)
  client.send("term_1", "review `$(unsafe)`\nnext", function(result)
    h.eq("ok", result.type)
  end)

  h.eq({ "herdr", "api", "snapshot" }, calls[1])
  h.eq({ "herdr", "agent", "start", "worker", "--cwd", "/repo", "--no-focus", "--", "codex" }, calls[2])
  h.eq({ "herdr", "pane", "run", "term_1", "review `$(unsafe)`\nnext" }, calls[3])
end)

h.test("client rejects malformed JSON atomically", function()
  reset()
  config.setup()
  client._set_runner(function(_, _, callback)
    callback({ code = 0, stdout = "not-json", stderr = "" })
  end)
  client.snapshot(function(result, err)
    h.eq(nil, result)
    h.eq("json", err.kind)
  end)
end)

h.test("client process errors never echo instruction bodies", function()
  reset()
  config.setup()
  client._set_runner(function(_, _, callback)
    callback({ code = 9, stdout = "", stderr = "" })
  end)
  client.send("term_1", "SECRET PROMPT BODY", function(result, err)
    h.eq(nil, result)
    h.contains(err.message, "pane run")
    h.truthy(not err.message:find("SECRET PROMPT BODY", 1, true))
  end)
end)

h.test("client accepts pane run success with an empty response body", function()
  reset()
  config.setup()
  client._set_runner(function(argv, _, callback)
    h.eq({ "herdr", "pane", "run", "w1:p1", "do the work" }, argv)
    callback({ code = 0, stdout = "", stderr = "" })
  end)
  client.send("w1:p1", "do the work", function(result, err)
    h.eq(nil, err)
    h.eq("ok", result.type)
  end)
end)

h.test("client shares one bounded server start among concurrent callers", function()
  reset()
  config.setup({ server_retry_ms = { 1 } })
  local status_calls = 0
  local starts = 0
  client._set_runner(function(argv, _, callback)
    if argv[2] == "status" then
      status_calls = status_calls + 1
      if status_calls == 1 then
        callback({ code = 1, stdout = "", stderr = "not running" })
      else
        callback({ code = 0, stdout = "status: running\nprotocol: 16\ncompatible: yes\n", stderr = "" })
      end
    end
  end)
  client._set_job_starter(function(argv)
    starts = starts + 1
    h.eq({ "herdr", "server" }, argv)
    return 42
  end)
  client._set_defer(function(callback)
    callback()
  end)

  local results = {}
  client.ensure_server(function(ok)
    table.insert(results, ok)
  end)
  client.ensure_server(function(ok)
    table.insert(results, ok)
  end)
  h.eq(1, starts)
  h.eq({ true, true }, results)
end)

h.test("client reports a missing Herdr executable asynchronously", function()
  reset()
  config.setup({ herdr_cmd = "definitely-not-a-real-herdr-command", auto_start_server = false })
  local result
  client.ensure_server(function(ok, err)
    result = { ok = ok, err = err }
  end)
  h.truthy(vim.wait(1000, function()
    return result ~= nil
  end))
  h.eq(false, result.ok)
  h.eq("executable", result.err.kind)
  h.contains(result.err.message, "ENOENT")
end)

h.test("client rejects a reachable but incompatible Herdr server", function()
  reset()
  config.setup({ auto_start_server = false })
  client._set_runner(function(_, _, callback)
    callback({ code = 0, stdout = "status: running\nprotocol: 99\ncompatible: no\n", stderr = "" })
  end)
  client.ensure_server(function(ok, err)
    h.eq(false, ok)
    h.eq("protocol", err.kind)
  end)
end)
