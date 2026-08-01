local h = require("tests.harness")
local ledger = require("shinsa.ledger")
local ui = require("shinsa.ui")

h.test("queue renders coverage, concerns, and high-risk unseen items", function()
  ui._reset()
  ledger._reset()
  local items = {
    {
      id = "api",
      path = "src/api.lua",
      line = 4,
      signature = "pub fn api()",
      kind = "fn",
      risk = "high",
      score = 9,
      reasons = { "contract", "public" },
    },
  }
  ledger.reconcile("repo\0base", items)
  ui.show({ panel = { side = "left", width = 50 } }, {
    root = vim.fn.getcwd(),
    base = "main",
    items = items,
  }, {})
  local lines = vim.api.nvim_buf_get_lines(ui._state().bufnr, 0, -1, false)
  h.contains(lines[1], "Shinsa")
  h.contains(lines[2], "0/1 reviewed")
  h.contains(lines[2], "1 high-risk unseen")
  h.contains(lines[5], "src/api.lua:4")

  ledger.toggle_reviewed("api")
  ledger.set_concern("api", "Verify callers")
  ui.render()
  lines = vim.api.nvim_buf_get_lines(ui._state().bufnr, 0, -1, false)
  h.contains(lines[2], "1/1 reviewed")
  h.contains(lines[2], "1 concerns")
  ui._reset()
end)

h.test("queue renders an analysis failure instead of staying on loading", function()
  ui._reset()
  local refreshed = false
  ui.loading({ panel = { side = "left", width = 50 } }, "Finding merge base", {
    refresh = function()
      refreshed = true
    end,
  })
  ui.failure({ panel = { side = "left", width = 50 } }, "git merge-base failed")
  local lines = vim.api.nvim_buf_get_lines(ui._state().bufnr, 0, -1, false)
  h.eq("Analysis failed", lines[3])
  h.contains(lines[5], "git merge-base failed")
  for _, mapping in ipairs(vim.api.nvim_buf_get_keymap(ui._state().bufnr, "n")) do
    if mapping.lhs == "r" then
      mapping.callback()
    end
  end
  h.eq(true, refreshed)
  ui._reset()
end)

h.test("start opens the highest-ranked unseen item regardless of cursor position", function()
  ui._reset()
  ledger._reset()
  local root = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  vim.fn.writefile({ "return 'high'" }, root .. "/high.lua")
  vim.fn.writefile({ "return 'low'" }, root .. "/low.lua")
  local function review_item(id, path, score, risk)
    return {
      id = id,
      path = path,
      line = 1,
      signature = path,
      kind = "file",
      risk = risk,
      score = score,
      reasons = { "file-level" },
    }
  end
  local items = {
    review_item("high", "high.lua", 9, "high"),
    review_item("low", "low.lua", 2, "low"),
  }
  ledger.reconcile("repo\0base", items)
  ui.show({ panel = { side = "left", width = 50 } }, { root = root, base = "main", items = items }, {})
  vim.api.nvim_win_set_cursor(ui._state().winid, { ui._state().item_lines.low, 0 })
  h.eq(true, ui.start())
  h.eq("high.lua", vim.fs.basename(vim.api.nvim_buf_get_name(0)))
  vim.cmd("bwipeout!")
  ui._reset()
  vim.fn.delete(root, "rf")
end)
