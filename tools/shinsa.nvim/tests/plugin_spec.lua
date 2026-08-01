local h = require("tests.harness")

h.test("plugin registers the public commands", function()
  vim.g.loaded_shinsa_nvim = nil
  vim.cmd("runtime plugin/shinsa.lua")
  for _, command in ipairs({
    "Shinsa",
    "ShinsaRefresh",
    "ShinsaToggle",
    "ShinsaClose",
    "ShinsaReset",
    "ShinsaYank",
    "ShinsaHealth",
  }) do
    h.eq(2, vim.fn.exists(":" .. command), command .. " should exist")
  end
  h.eq("?", vim.api.nvim_get_commands({}).Shinsa.nargs)
end)
