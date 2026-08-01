local M = {}

local initialized = false
local request_id = 0
local last_request
local current_result

local function ensure_setup()
  if not initialized then
    M.setup()
  end
end

local function notify_error(message)
  vim.notify(tostring(message), vim.log.levels.ERROR, { title = "Shinsa" })
end

local function actions(config, result)
  return {
    refresh = M.refresh,
    yank = M.yank,
    diff = function(item)
      require("shinsa.integrations").diff(config, result, item)
    end,
    git = function()
      require("shinsa.integrations").git(config, result)
    end,
  }
end

function M.setup(opts)
  require("shinsa.config").setup(opts)
  initialized = true
  return M
end

function M.analyze(base)
  ensure_setup()
  local config = require("shinsa.config").get()
  local ui = require("shinsa.ui")
  local root
  if ui.is_current() and last_request then
    root = last_request.root
  else
    local err
    root, err = require("shinsa.git").root(0)
    if not root then
      notify_error(err)
      return
    end
  end
  request_id = request_id + 1
  local current_request = request_id
  last_request = { root = root, base = base }
  ui.loading(config, "Resolving review scope", { refresh = M.refresh })
  require("shinsa.analyzer").run({ config = config, root = root, base = base }, function(result, err)
    if current_request ~= request_id then
      return
    end
    if not result then
      ui.failure(config, err)
      notify_error(err)
      return
    end
    current_result = result
    last_request = { root = result.root, base = result.base }
    require("shinsa.ledger").reconcile(result.root .. "\0" .. result.merge_base, result.items)
    ui.show(config, result, actions(config, result))
    pcall(vim.api.nvim_exec_autocmds, "User", {
      pattern = "ShinsaReady",
      data = { root = result.root, base = result.base, item_count = #result.items },
    })
  end, function(stage)
    if current_request == request_id then
      ui.loading(config, stage)
    end
  end)
end

function M.refresh()
  if last_request then
    M.analyze(last_request.base)
  else
    M.analyze()
  end
end

function M.toggle()
  ensure_setup()
  local ui = require("shinsa.ui")
  if ui.is_open() then
    ui.close()
  elseif not ui.open(require("shinsa.config").get()) then
    M.analyze()
  end
end

function M.close()
  require("shinsa.ui").close()
end

function M.reset()
  require("shinsa.ledger").reset()
  require("shinsa.ui").render()
end

function M.yank()
  ensure_setup()
  if not current_result then
    notify_error("no active review")
    return nil
  end
  local markdown, err = require("shinsa.ledger").markdown({ base = current_result.base }, current_result.items)
  if not markdown then
    vim.notify(err, vim.log.levels.INFO, { title = "Shinsa" })
    return nil
  end
  local register = require("shinsa.config").get().clipboard_register
  local ok, set_err = pcall(vim.fn.setreg, register, markdown)
  if not ok then
    notify_error("could not write register " .. register .. ": " .. tostring(set_err))
    return nil
  end
  vim.notify("Yanked review concerns to " .. register, nil, { title = "Shinsa" })
  return markdown
end

function M._reset()
  request_id = request_id + 1
  initialized = false
  last_request = nil
  current_result = nil
  require("shinsa.config")._reset()
  require("shinsa.ledger")._reset()
  require("shinsa.ui")._reset()
end

return M
