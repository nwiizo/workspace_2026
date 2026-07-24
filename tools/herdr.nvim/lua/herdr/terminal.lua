local config = require("herdr.config")
local client = require("herdr.client")

local M = {}

local terminals = {}
local jobstop = vim.fn.jobstop

local function split_width(value)
  if value < 1 then
    return math.max(20, math.floor(vim.o.columns * value))
  end
  return value
end

local function open_split()
  local terminal_config = config.get().terminal
  local modifier = terminal_config.side == "left" and "topleft" or "botright"
  local width = split_width(terminal_config.width)
  vim.cmd(string.format("%s %dvnew", modifier, width))
  local window = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_width(window, width)
  return window, vim.api.nvim_get_current_buf()
end

local function focus_buffer(bufnr)
  local windows = vim.fn.win_findbuf(bufnr)
  if #windows > 0 and vim.api.nvim_win_is_valid(windows[1]) then
    vim.api.nvim_set_current_win(windows[1])
    return windows[1]
  end
  local window = open_split()
  vim.api.nvim_win_set_buf(window, bufnr)
  return window
end

function M.attach(agent, opts)
  opts = opts or {}
  local existing = terminals[agent.terminal_id]
  if existing and vim.api.nvim_buf_is_valid(existing.bufnr) then
    focus_buffer(existing.bufnr)
    if config.get().terminal.auto_insert then
      vim.cmd("startinsert")
    end
    return existing.bufnr
  end

  local _, bufnr = open_split()
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.b[bufnr].herdr_terminal_id = agent.terminal_id
  local job_id, attach_err = client.attach(agent.target or agent.terminal_id, opts.takeover == true, {
    on_exit = function(_, code)
      vim.schedule(function()
        local record = terminals[agent.terminal_id]
        local unexpected = record and record.bufnr == bufnr
        if record and record.bufnr == bufnr then
          terminals[agent.terminal_id] = nil
        end
        if unexpected and code ~= 0 and vim.v.exiting == vim.NIL then
          local output = ""
          if vim.api.nvim_buf_is_valid(bufnr) then
            local line_count = vim.api.nvim_buf_line_count(bufnr)
            output = table.concat(vim.api.nvim_buf_get_lines(bufnr, math.max(0, line_count - 8), -1, false), "\n")
          end
          local lower = output:lower()
          if lower:find("takeover", 1, true) or lower:find("another client", 1, true) then
            vim.notify(
              string.format(
                "Herdr attach for %s is owned by another client; use :HerdrAttach! to take over",
                agent.name
              ),
              vim.log.levels.WARN,
              { title = "Herdr" }
            )
          else
            vim.notify(
              string.format("Herdr attach for %s exited with code %d; run :HerdrHealth", agent.name, code),
              vim.log.levels.WARN,
              { title = "Herdr" }
            )
          end
        end
      end)
    end,
  })
  if not job_id then
    vim.api.nvim_buf_delete(bufnr, { force = true })
    vim.notify(attach_err.message, vim.log.levels.ERROR, { title = "Herdr" })
    return nil
  end
  terminals[agent.terminal_id] = { bufnr = bufnr, job_id = job_id }
  vim.api.nvim_create_autocmd("BufWipeout", {
    buffer = bufnr,
    once = true,
    callback = function()
      local record = terminals[agent.terminal_id]
      if record and record.bufnr == bufnr then
        terminals[agent.terminal_id] = nil
      end
    end,
  })
  if config.get().terminal.auto_insert then
    vim.cmd("startinsert")
  end
  return bufnr
end

function M.cleanup()
  for _, record in pairs(terminals) do
    if type(record.job_id) == "number" and record.job_id > 0 then
      pcall(jobstop, record.job_id)
    end
  end
  terminals = {}
end

function M._set_termopen(value)
  client._set_termopen(value)
end

function M._set_jobstop(value)
  jobstop = value or vim.fn.jobstop
end

function M._terminals()
  return terminals
end

function M._reset()
  for _, record in pairs(terminals) do
    if vim.api.nvim_buf_is_valid(record.bufnr) then
      vim.api.nvim_buf_delete(record.bufnr, { force = true })
    end
  end
  terminals = {}
  client._set_termopen(nil)
  jobstop = vim.fn.jobstop
end

return M
