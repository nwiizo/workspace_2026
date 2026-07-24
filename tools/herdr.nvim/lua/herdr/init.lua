local M = {}

local initialized = false
local cleanup_group

local function message(err)
  if type(err) == "table" then
    return err.message or err.kind or vim.inspect(err)
  end
  return tostring(err)
end

local function notify_error(err)
  vim.notify(message(err), vim.log.levels.ERROR, { title = "Herdr" })
end

local function ensure_setup()
  if not initialized then
    M.setup()
  end
end

local function configured_agent_kinds()
  local agents = require("herdr.config").get().agents
  local result = {}
  for _, preferred in ipairs({ "codex", "claude" }) do
    if agents[preferred] then
      table.insert(result, preferred)
    end
  end
  local rest = {}
  for kind in pairs(agents) do
    if kind ~= "codex" and kind ~= "claude" then
      table.insert(rest, kind)
    end
  end
  table.sort(rest)
  vim.list_extend(result, rest)
  return result
end

local function with_agent_kind(kind, callback)
  if kind and kind ~= "" then
    callback(kind)
    return
  end
  local kinds = configured_agent_kinds()
  vim.ui.select(kinds, { prompt = "Start agent:" }, callback)
end

local function find_agent(target)
  local matches = {}
  for _, agent in ipairs(require("herdr.state").agents()) do
    if target == agent.terminal_id then
      return agent
    end
    if target == agent.name then
      table.insert(matches, agent)
    end
  end
  if #matches == 1 then
    return matches[1]
  end
  if #matches > 1 then
    local ids = vim.tbl_map(function(agent)
      return agent.terminal_id
    end, matches)
    return nil, string.format("agent name %q is ambiguous; use a terminal ID: %s", target, table.concat(ids, ", "))
  end
  return nil, string.format("unknown Herdr agent %q; run :HerdrRefresh", target)
end

local function with_target(target, callback)
  local state = require("herdr.state")
  if target and target ~= "" then
    local agent, err = find_agent(target)
    if not agent then
      notify_error(err)
      return
    end
    callback(agent)
    return
  end
  local agents = state.agents()
  if #agents == 0 then
    notify_error("no Herdr agents; run :HerdrStart codex or :HerdrStart claude")
  elseif #agents == 1 then
    callback(agents[1])
  else
    vim.ui.select(agents, {
      prompt = "Herdr agent:",
      format_item = function(agent)
        return string.format("%-12s %-8s %s  [%s]", agent.name, agent.status, agent.cwd or "", agent.terminal_id)
      end,
    }, callback)
  end
end

local function with_server(callback)
  require("herdr.client").ensure_server(function(ok, err)
    if not ok then
      notify_error(err)
      return
    end
    callback()
  end)
end

function M.setup(opts)
  require("herdr.config").setup(opts)
  if initialized then
    require("herdr.state").stop()
  end
  require("herdr.client").mark_server_unavailable()
  require("herdr.board").setup()
  require("herdr.notifier")._reset()
  initialized = true

  cleanup_group = vim.api.nvim_create_augroup("HerdrCleanup", { clear = true })
  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = cleanup_group,
    callback = function()
      require("herdr.state").stop()
      require("herdr.terminal").cleanup()
    end,
  })
  vim.schedule(function()
    if initialized then
      require("herdr.state").start()
    end
  end)
  return M
end

function M.toggle()
  ensure_setup()
  require("herdr.board").toggle()
end

function M.refresh()
  ensure_setup()
  require("herdr.state").refresh({ explicit = true }, function(ok, result)
    if not ok then
      notify_error(result)
    end
  end)
end

function M.start(kind, opts)
  ensure_setup()
  opts = opts or {}
  with_agent_kind(kind, function(selected_kind)
    if not selected_kind then
      return
    end
    if not require("herdr.config").get().agents[selected_kind] then
      notify_error("unknown agent kind: " .. selected_kind)
      return
    end
    local context = require("herdr.context")
    local cwd = opts.cwd or context.project_root(0)
    local project = vim.fn.fnamemodify(cwd, ":t")
    local default_name = string.format("%s-%s", selected_kind, project ~= "" and project or "agent")
    vim.ui.input({ prompt = "Agent name: ", default = opts.name or default_name }, function(name)
      if not name or name == "" then
        return
      end
      with_server(function()
        require("herdr.client").start_agent(selected_kind, name, cwd, function(_, err)
          if err then
            notify_error(err)
            return
          end
          vim.notify(
            string.format("started %s agent %s", selected_kind, name),
            vim.log.levels.INFO,
            { title = "Herdr" }
          )
          require("herdr.state").refresh({ explicit = true })
        end)
      end)
    end)
  end)
end

function M.attach(target, opts)
  ensure_setup()
  opts = opts or {}
  with_target(target, function(agent)
    if not agent then
      return
    end
    with_server(function()
      require("herdr.terminal").attach(agent, opts)
    end)
  end)
end

local function send_to_agent(agent, text)
  if not text or text == "" then
    return
  end
  if #text > require("herdr.config").get().context.max_bytes then
    notify_error("instruction exceeds context.max_bytes")
    return
  end
  with_server(function()
    require("herdr.client").send(agent.pane_id, text, function(_, err)
      if err then
        notify_error(err)
        return
      end
      vim.notify("sent context to " .. agent.name, vim.log.levels.INFO, { title = "Herdr" })
    end)
  end)
end

local function send_text(target, text)
  if not text or text == "" then
    return
  end
  with_target(target, function(agent)
    send_to_agent(agent, text)
  end)
end

local function send_context(target, builder)
  with_target(target, function(agent)
    local text, err = builder(agent)
    if not text then
      local level = err == "buffer has no diagnostics" and vim.log.levels.INFO or vim.log.levels.ERROR
      vim.notify(err, level, { title = "Herdr" })
      return
    end
    send_to_agent(agent, text)
  end)
end

function M.send(target, text)
  ensure_setup()
  if text ~= nil then
    send_text(target, text)
    return
  end
  vim.ui.input({ prompt = "Instruction: " }, function(input)
    send_text(target, input)
  end)
end

function M._send_range(target, bufnr, line1, line2)
  ensure_setup()
  send_context(target, function(agent)
    return require("herdr.context").range(bufnr or 0, line1, line2, {
      target_cwd = agent.cwd or false,
    })
  end)
end

function M._send_visual(target, bufnr, line1, line2)
  ensure_setup()
  send_context(target, function(agent)
    return require("herdr.context").range_from_marks(bufnr or 0, line1, line2, {
      target_cwd = agent.cwd or false,
    })
  end)
end

function M._send_file(target, bufnr)
  ensure_setup()
  send_context(target, function(agent)
    return require("herdr.context").file(bufnr or 0, { target_cwd = agent.cwd or false })
  end)
end

function M._send_diagnostics(target, bufnr)
  ensure_setup()
  send_context(target, function(agent)
    return require("herdr.context").diagnostics(bufnr or 0, { target_cwd = agent.cwd or false })
  end)
end

function M.statusline()
  local ok, result = pcall(function()
    if not initialized then
      return ""
    end
    local configured = require("herdr.config").get()
    if vim.fn.executable(configured.herdr_cmd) ~= 1 then
      return ""
    end
    local status = require("herdr.state").get()
    if not status.snapshot then
      return ""
    end
    local counts = require("herdr.state").counts()
    local parts = { "H" }
    for _, key in ipairs({ "blocked", "working", "done", "idle", "unknown" }) do
      if counts[key] > 0 then
        table.insert(parts, configured.board.markers[key] .. counts[key])
      end
    end
    if status.stale then
      table.insert(parts, "~")
    end
    return table.concat(parts, " ")
  end)
  return ok and result or ""
end

function M._complete_agents()
  local result = {}
  for _, agent in ipairs(require("herdr.state").agents()) do
    table.insert(result, agent.terminal_id)
  end
  return result
end

function M._find_agent(target)
  return find_agent(target)
end

function M._complete_agent_kinds()
  ensure_setup()
  return configured_agent_kinds()
end

function M._reset()
  initialized = false
  if cleanup_group then
    pcall(vim.api.nvim_del_augroup_by_id, cleanup_group)
  end
  cleanup_group = nil
end

return M
