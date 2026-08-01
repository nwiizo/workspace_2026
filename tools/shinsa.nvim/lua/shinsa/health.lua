local M = {}

function M.check()
  vim.health.start("shinsa.nvim")
  if vim.fn.has("nvim-0.10") == 1 then
    vim.health.ok("Neovim 0.10+")
  else
    vim.health.error("Neovim 0.10+ is required")
  end
  local config = require("shinsa.config").get()
  local git = config.git_command[1]
  if vim.fn.executable(git) == 1 then
    vim.health.ok("Git executable: " .. git)
  else
    vim.health.error("Git executable not found: " .. git)
  end
  local path = vim.api.nvim_buf_get_name(0)
  if path ~= "" then
    local language = require("shinsa.symbols")._language_for(path)
    if language then
      local ok = pcall(vim.treesitter.get_string_parser, "", language)
      if ok then
        vim.health.ok("Tree-sitter parser for current buffer: " .. language)
      else
        vim.health.warn("No Tree-sitter parser for current buffer; file-level fallback will be used")
      end
    end
  end
  if vim.fn.exists(":CodeDiff") == 2 or vim.fn.exists(":DiffviewOpen") == 2 then
    vim.health.ok("Optional precise diff handoff is available")
  else
    vim.health.info("CodeDiff/Diffview not found; built-in unified diff will be used")
  end
end

return M
