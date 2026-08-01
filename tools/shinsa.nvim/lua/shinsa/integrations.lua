local M = {}

local function exists(name)
  return vim.fn.exists(":" .. name) == 2
end

local function notify_error(message)
  vim.notify(message, vim.log.levels.ERROR, { title = "Shinsa" })
end

local function builtin_diff(result, item)
  local lines = vim.split(result.diff or "", "\n", { plain = true })
  if #lines == 1 and lines[1] == "" then
    notify_error("the current review has no diff text")
    return false
  end
  local name = string.format(
    "shinsa://diff/%s/%s",
    (result.merge_base or "working-tree"):sub(1, 12),
    vim.fn.sha256(item.path):sub(1, 16)
  )
  local bufnr = vim.fn.bufnr(name)
  if bufnr == -1 then
    bufnr = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_buf_set_name(bufnr, name)
  else
    vim.fn.bufload(bufnr)
  end
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].filetype = "diff"
  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false
  vim.cmd("tabnew")
  vim.api.nvim_win_set_buf(0, bufnr)
  vim.wo.wrap = false
  vim.keymap.set("n", "q", "<cmd>tabclose<cr>", { buffer = bufnr, silent = true, desc = "Close review diff" })
  local needle = "b/" .. item.path
  for index, line in ipairs(lines) do
    if line:find(needle, 1, true) then
      vim.api.nvim_win_set_cursor(0, { index, 0 })
      vim.cmd("normal! zz")
      break
    end
  end
  return true
end

local function safe_builtin_diff(result, item)
  local ok, opened = pcall(builtin_diff, result, item)
  if not ok then
    notify_error("built-in diff failed: " .. tostring(opened))
    return false
  end
  return opened
end

local function resolve_diff(config)
  local selected = config.handoff.diff
  if selected ~= "auto" then
    return selected
  end
  if exists("CodeDiff") then
    return "codediff"
  end
  if exists("DiffviewOpen") then
    return "diffview"
  end
  return "builtin"
end

function M.diff(config, result, item)
  local backend = resolve_diff(config)
  if backend == false then
    return false
  end
  if type(backend) == "function" then
    local ok, value = pcall(backend, {
      root = result.root,
      base = result.base,
      merge_base = result.merge_base,
      path = item.path,
      line = item.line,
      item = item,
    })
    if not ok then
      notify_error("custom diff handoff failed: " .. tostring(value))
      return false
    end
    return value ~= false
  end
  if backend == "codediff" then
    if item.deleted then
      return safe_builtin_diff(result, item)
    end
    if not exists("CodeDiff") then
      notify_error(":CodeDiff is unavailable")
      return false
    end
    local ok, err = pcall(vim.api.nvim_cmd, { cmd = "CodeDiff", args = { "file", result.merge_base } }, {})
    if not ok then
      notify_error("CodeDiff failed: " .. tostring(err))
      return false
    end
    return true
  end
  if backend == "diffview" then
    if not exists("DiffviewOpen") then
      notify_error(":DiffviewOpen is unavailable")
      return false
    end
    local ok, err = pcall(vim.api.nvim_cmd, {
      cmd = "DiffviewOpen",
      args = { result.merge_base, "--", item.path },
    }, {})
    if not ok then
      notify_error("Diffview failed: " .. tostring(err))
      return false
    end
    return true
  end
  return safe_builtin_diff(result, item)
end

local function resolve_git(config)
  local selected = config.handoff.git
  if selected ~= "auto" then
    return selected
  end
  if exists("LazyGit") then
    return "lazygit"
  end
  if exists("Neogit") then
    return "neogit"
  end
  return false
end

function M.git(config, result)
  local backend = resolve_git(config)
  if backend == false then
    vim.notify("No LazyGit or Neogit command is available", vim.log.levels.INFO, {
      title = "Shinsa",
    })
    return false
  end
  if type(backend) == "function" then
    local ok, value = pcall(backend, { root = result.root, base = result.base, merge_base = result.merge_base })
    if not ok then
      notify_error("custom Git handoff failed: " .. tostring(value))
      return false
    end
    return value ~= false
  end
  if backend == "lazygit" then
    if not exists("LazyGit") then
      notify_error(":LazyGit is unavailable")
      return false
    end
    local loaded, lazygit = pcall(require, "lazygit")
    if not loaded or type(lazygit.lazygit) ~= "function" then
      notify_error("lazygit.nvim Lua API is unavailable")
      return false
    end
    local ok, err = pcall(lazygit.lazygit, result.root)
    if not ok then
      notify_error("LazyGit failed: " .. tostring(err))
      return false
    end
    return true
  end
  if not exists("Neogit") then
    notify_error(":Neogit is unavailable")
    return false
  end
  local ok, err = pcall(vim.api.nvim_cmd, { cmd = "Neogit", args = { "cwd=" .. result.root } }, {})
  if not ok then
    notify_error("Neogit failed: " .. tostring(err))
    return false
  end
  return true
end

M._resolve_diff = resolve_diff
M._resolve_git = resolve_git

return M
