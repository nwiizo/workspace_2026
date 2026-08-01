local h = require("tests.harness")
local integrations = require("shinsa.integrations")

h.test("built-in diff reuses its named buffer when opened repeatedly", function()
  local initial_tabs = vim.fn.tabpagenr("$")
  local result = {
    root = vim.fn.getcwd(),
    base = "main",
    merge_base = string.rep("a", 40),
    diff = table.concat({
      "diff --git a/file.lua b/file.lua",
      "--- a/file.lua",
      "+++ b/file.lua",
      "@@ -1 +1 @@",
      "-return 1",
      "+return 2",
    }, "\n"),
  }
  local item = { path = "file.lua", line = 1, deleted = false }
  h.eq(true, integrations.diff({ handoff = { diff = "builtin" } }, result, item))
  h.eq(true, integrations.diff({ handoff = { diff = "builtin" } }, result, item))
  local buffers = 0
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_get_name(bufnr):match("^shinsa://diff/") then
      buffers = buffers + 1
    end
  end
  h.eq(1, buffers)
  while vim.fn.tabpagenr("$") > initial_tabs do
    vim.cmd("tabclose!")
  end
end)
