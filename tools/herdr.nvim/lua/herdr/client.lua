local config = require("herdr.config")

local M = {}

local valid_status = {
  blocked = true,
  working = true,
  done = true,
  idle = true,
  unknown = true,
}

local runner
local job_starter
local termopen
local defer = vim.defer_fn
local server_ready = false
local server_checking = false
local server_waiters = {}

local function trim(value)
  return (value or ""):gsub("^%s+", ""):gsub("%s+$", "")
end

local function default_runner(argv, opts, callback)
  local ok, spawn_err = pcall(vim.system, argv, { text = true, timeout = opts.timeout_ms }, function(result)
    vim.schedule(function()
      callback(result)
    end)
  end)
  if not ok then
    vim.schedule(function()
      callback({ code = 127, stdout = "", stderr = tostring(spawn_err), spawn_error = true })
    end)
  end
end

local function default_job_starter(argv)
  local ok, result = pcall(vim.fn.jobstart, argv, { detach = true })
  if not ok then
    return nil, tostring(result)
  end
  return result
end

local function get_runner()
  return runner or default_runner
end

local function get_job_starter()
  return job_starter or default_job_starter
end

local function get_termopen()
  return termopen or vim.fn.termopen
end

local function argv(args)
  local result = { config.get().herdr_cmd }
  vim.list_extend(result, args)
  return result
end

local function process_error(result, command)
  local message = trim(result.stderr)
  if message == "" then
    message = trim(result.stdout)
  end
  if message == "" then
    message = string.format("%s exited with code %s", command, tostring(result.code))
  end
  return {
    kind = (result.spawn_error == true or message:find("ENOENT", 1, true) ~= nil) and "executable" or "process",
    code = result.code,
    signal = result.signal,
    message = message,
  }
end

local function run(args, opts, callback)
  opts = opts or {}
  opts.timeout_ms = opts.timeout_ms or config.get().refresh.timeout_ms
  get_runner()(argv(args), opts, function(result)
    if type(result) ~= "table" then
      callback(nil, { kind = "internal", message = "process runner returned no result" })
      return
    end
    if result.code ~= 0 then
      local safe_command = table.concat(vim.list_slice(args, 1, math.min(#args, 2)), " ")
      callback(nil, process_error(result, safe_command))
      return
    end
    callback(result)
  end)
end

local function run_json(args, callback)
  run(args, {}, function(result, err)
    if err then
      callback(nil, err)
      return
    end
    local ok, decoded = pcall(vim.json.decode, result.stdout or "")
    if not ok or type(decoded) ~= "table" then
      callback(nil, { kind = "json", message = "Herdr returned invalid JSON" })
      return
    end
    if decoded.error ~= nil and decoded.error ~= vim.NIL then
      callback(nil, {
        kind = "api",
        message = decoded.error.message or decoded.error.code or "Herdr API error",
        code = decoded.error.code,
      })
      return
    end
    if type(decoded.result) ~= "table" then
      callback(nil, { kind = "json", message = "Herdr response is missing result" })
      return
    end
    callback(decoded.result)
  end)
end

local function required_string(agent, key, index)
  if type(agent[key]) ~= "string" or agent[key] == "" then
    return nil, string.format("agent %d is missing required field %s", index, key)
  end
  return agent[key]
end

local function nullable(value)
  if value == vim.NIL then
    return nil
  end
  return value
end

local function normalize_agent(agent, index)
  if type(agent) ~= "table" then
    return nil, string.format("agent %d is not an object", index)
  end

  local terminal_id, err = required_string(agent, "terminal_id", index)
  if not terminal_id then
    return nil, err
  end
  local workspace_id
  workspace_id, err = required_string(agent, "workspace_id", index)
  if not workspace_id then
    return nil, err
  end
  local tab_id
  tab_id, err = required_string(agent, "tab_id", index)
  if not tab_id then
    return nil, err
  end
  local pane_id
  pane_id, err = required_string(agent, "pane_id", index)
  if not pane_id then
    return nil, err
  end
  if type(agent.agent_status) ~= "string" then
    return nil, string.format("agent %d is missing required field agent_status", index)
  end
  if type(agent.focused) ~= "boolean" then
    return nil, string.format("agent %d is missing required field focused", index)
  end
  if type(agent.revision) ~= "number" then
    return nil, string.format("agent %d is missing required field revision", index)
  end

  local status = valid_status[agent.agent_status] and agent.agent_status or "unknown"
  local name = nullable(agent.name) or nullable(agent.display_agent) or nullable(agent.agent) or terminal_id
  local kind = nullable(agent.agent) or nullable(agent.display_agent) or "agent"
  local title = nullable(agent.title) or nullable(agent.terminal_title_stripped) or nullable(agent.terminal_title) or ""

  return {
    terminal_id = terminal_id,
    workspace_id = workspace_id,
    tab_id = tab_id,
    pane_id = pane_id,
    status = status,
    focused = agent.focused,
    revision = agent.revision,
    name = tostring(name),
    kind = tostring(kind),
    title = tostring(title),
    cwd = nullable(agent.foreground_cwd) or nullable(agent.cwd),
    target = terminal_id,
  }
end

function M._normalize_snapshot(result)
  if type(result) ~= "table" or result.type ~= "session_snapshot" or type(result.snapshot) ~= "table" then
    return nil, { kind = "schema", message = "Herdr response is not a session snapshot" }
  end
  local source = result.snapshot
  if type(source.protocol) ~= "number" then
    return nil, { kind = "schema", message = "Herdr snapshot is missing protocol" }
  end
  if
    type(source.agents) ~= "table"
    or not vim.islist(source.agents)
    or type(source.workspaces) ~= "table"
    or not vim.islist(source.workspaces)
  then
    return nil, { kind = "schema", message = "Herdr snapshot is missing agents or workspaces" }
  end

  local agents = {}
  for index, agent in ipairs(source.agents) do
    local normalized, message = normalize_agent(agent, index)
    if not normalized then
      return nil, { kind = "schema", message = message }
    end
    table.insert(agents, normalized)
  end

  local workspaces = {}
  for index, workspace in ipairs(source.workspaces) do
    if type(workspace) ~= "table" then
      return nil, { kind = "schema", message = string.format("workspace %d is not an object", index) }
    end
    if type(workspace.workspace_id) ~= "string" or workspace.workspace_id == "" then
      return nil,
        { kind = "schema", message = string.format("workspace %d is missing required field workspace_id", index) }
    end
    local label = nullable(workspace.label) or workspace.workspace_id
    table.insert(workspaces, {
      workspace_id = workspace.workspace_id,
      label = tostring(label),
      focused = workspace.focused == true,
    })
  end

  return {
    agents = agents,
    workspaces = workspaces,
    version = source.version,
    protocol = source.protocol,
    focused_workspace_id = source.focused_workspace_id,
  }
end

function M.snapshot(callback)
  run_json({ "api", "snapshot" }, function(result, err)
    if err then
      callback(nil, err)
      return
    end
    local snapshot, normalize_err = M._normalize_snapshot(result)
    callback(snapshot, normalize_err)
  end)
end

function M.start_agent(kind, name, cwd, callback)
  local agent = config.get().agents[kind]
  if not agent then
    callback(nil, { kind = "config", message = "unknown agent kind: " .. tostring(kind) })
    return
  end
  local args = { "agent", "start", name, "--cwd", cwd, "--no-focus", "--" }
  vim.list_extend(args, agent.command)
  run_json(args, callback)
end

function M.send(target, text, callback)
  run({ "pane", "run", target, text }, {}, function(result, err)
    if err then
      callback(nil, err)
      return
    end
    callback({ type = "ok", code = result.code })
  end)
end

function M.attach(target, takeover, opts)
  local args = { "agent", "attach", target }
  if takeover then
    table.insert(args, "--takeover")
  end
  local ok, job_id = pcall(get_termopen(), argv(args), opts or {})
  if not ok then
    return nil, { kind = "process", message = tostring(job_id) }
  end
  if type(job_id) ~= "number" or job_id <= 0 then
    return nil, { kind = "process", message = "failed to start Herdr attach terminal" }
  end
  return job_id
end

function M.integration_status(callback)
  run({ "integration", "status" }, {}, function(result, err)
    callback(result and result.stdout or nil, err)
  end)
end

local function finish_server_check(ok, err)
  server_checking = false
  server_ready = ok == true
  local waiters = server_waiters
  server_waiters = {}
  for _, callback in ipairs(waiters) do
    callback(server_ready, err)
  end
end

local function check_server(callback)
  run({ "status", "server" }, {}, function(result, err)
    if err then
      callback(false, err)
      return
    end
    local output = result.stdout or ""
    local protocol = tonumber(output:match("protocol:%s*(%d+)"))
    if not protocol or not output:match("compatible:%s*yes") then
      callback(false, {
        kind = "protocol",
        message = "Herdr server did not report a compatible protocol; update Herdr and run :checkhealth herdr",
      })
      return
    end
    callback(true)
  end)
end

local function retry_server(index, last_err)
  local delays = config.get().server_retry_ms
  if index > #delays then
    local detail = last_err and last_err.message or "no response"
    finish_server_check(false, {
      kind = "server",
      message = "Herdr server did not become ready: " .. detail,
      cause = last_err,
    })
    return
  end
  defer(function()
    check_server(function(ok, err)
      if ok then
        finish_server_check(true)
      elseif err and (err.kind == "executable" or err.kind == "protocol") then
        finish_server_check(false, err)
      else
        retry_server(index + 1, err)
      end
    end)
  end, delays[index])
end

function M.ensure_server(callback)
  if server_ready then
    callback(true)
    return
  end
  table.insert(server_waiters, callback)
  if server_checking then
    return
  end
  server_checking = true

  check_server(function(ok, err)
    if ok then
      finish_server_check(true)
      return
    end
    if not config.get().auto_start_server then
      finish_server_check(false, err)
      return
    end
    if err and (err.kind == "executable" or err.kind == "protocol") then
      finish_server_check(false, err)
      return
    end
    local job_id, start_err = get_job_starter()(argv({ "server" }))
    if type(job_id) ~= "number" or job_id <= 0 then
      finish_server_check(false, {
        kind = "server",
        message = "failed to start detached Herdr server" .. (start_err and ": " .. start_err or ""),
      })
      return
    end
    retry_server(1, err)
  end)
end

function M.mark_server_unavailable()
  server_ready = false
end

function M._set_runner(value)
  runner = value
end

function M._set_job_starter(value)
  job_starter = value
end

function M._set_termopen(value)
  termopen = value
end

function M._set_defer(value)
  defer = value or vim.defer_fn
end

function M._reset()
  runner = nil
  job_starter = nil
  termopen = nil
  defer = vim.defer_fn
  server_ready = false
  server_checking = false
  server_waiters = {}
end

return M
