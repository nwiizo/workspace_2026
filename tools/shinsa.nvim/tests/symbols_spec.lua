local h = require("tests.harness")
local symbols = require("shinsa.symbols")

h.test("symbol header is compact and excludes the body", function()
  local signature = symbols._header("pub fn compute(value: usize) -> usize {\n  value + 1\n}", "rust")
  h.eq("pub fn compute(value: usize) -> usize", signature)
end)

h.test("symbol extraction maps Lua changes or safely falls back", function()
  local root = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  local path = root .. "/service.lua"
  vim.fn.writefile({
    "local function compute(value)",
    "  return value + 1",
    "end",
    "",
    "return compute",
  }, path)
  local items = symbols.extract(root, {
    path = "service.lua",
    ranges = { { new_start = 2, new_count = 1 } },
    added = 1,
    removed = 1,
    new_file = false,
    deleted = false,
    binary = false,
  }, { max_file_bytes = 1024 * 1024 })
  vim.fn.delete(root, "rf")
  h.eq(1, #items)
  if items[1].file_level then
    h.contains(items[1].fallback_reason, "Tree-sitter parser unavailable")
  else
    h.eq("compute", items[1].name)
    h.eq("fn", items[1].kind)
    h.eq(1, items[1].line)
    h.eq(false, items[1].contract)
  end
end)

h.test("unsupported and deleted files become explicit file-level items", function()
  local root = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  vim.fn.writefile({ "data" }, root .. "/asset.unknown_extension")
  local common = {
    ranges = { { new_start = 1, new_count = 1 } },
    added = 1,
    removed = 0,
    new_file = false,
    binary = false,
  }
  local unsupported = vim.tbl_extend("force", common, { path = "asset.unknown_extension", deleted = false })
  local deleted = vim.tbl_extend("force", common, { path = "gone.lua", deleted = true })
  local unsupported_item = symbols.extract(root, unsupported, { max_file_bytes = 1024 })[1]
  local deleted_item = symbols.extract(root, deleted, { max_file_bytes = 1024 })[1]
  vim.fn.delete(root, "rf")
  h.eq(true, unsupported_item.file_level)
  h.eq("unsupported filetype", unsupported_item.fallback_reason)
  h.eq(true, deleted_item.file_level)
  h.eq("deleted file", deleted_item.fallback_reason)
end)

h.test("symbol extraction never follows repository symlinks", function()
  local root = vim.fn.tempname()
  local outside = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  vim.fn.writefile({ "local secret = 'do-not-read'" }, outside)
  local linked, link_err = vim.uv.fs_symlink(outside, root .. "/linked.lua")
  h.truthy(linked, link_err)
  local item = symbols.extract(root, {
    path = "linked.lua",
    ranges = { { new_start = 1, new_count = 1, old_count = 1 } },
    added = 1,
    removed = 1,
    new_file = false,
    deleted = false,
    binary = false,
  }, { max_file_bytes = 1024 })[1]
  vim.fn.delete(root, "rf")
  vim.fn.delete(outside)
  h.eq(true, item.file_level)
  h.eq("symbolic link", item.fallback_reason)
  h.truthy(not item.signature:find("do-not-read", 1, true))
end)

h.test("deletion-only hunks are not attributed to a surviving neighbor", function()
  local root = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  vim.fn.writefile({ "local function keep()", "  return true", "end", "", "return keep" }, root .. "/service.lua")
  local items = symbols.extract(root, {
    path = "service.lua",
    ranges = { { old_start = 1, old_count = 3, new_start = 1, new_count = 0 } },
    added = 0,
    removed = 3,
    new_file = false,
    deleted = false,
    binary = false,
  }, { max_file_bytes = 1024 })
  vim.fn.delete(root, "rf")
  h.eq(1, #items)
  h.eq(true, items[1].file_level)
  h.eq(true, items[1].removal)
  h.eq(3, items[1].removed_lines)
  h.eq("deleted lines", items[1].fallback_reason)
end)

h.test("replacement hunks retain removal risk", function()
  local root = vim.fn.tempname()
  vim.fn.mkdir(root, "p")
  vim.fn.writefile({ "local function compute()", "  return 2", "end", "", "return compute" }, root .. "/service.lua")
  local items = symbols.extract(root, {
    path = "service.lua",
    ranges = { { old_start = 2, old_count = 1, new_start = 2, new_count = 1 } },
    added = 1,
    removed = 1,
    new_file = false,
    deleted = false,
    binary = false,
  }, { max_file_bytes = 1024 })
  vim.fn.delete(root, "rf")
  local removal
  for _, item in ipairs(items) do
    if item.removal then
      removal = item
      break
    end
  end
  h.truthy(removal, "replacement must preserve removal risk")
  h.eq(1, removal.removed_lines)
end)

h.test("TypeScript export ancestors mark the inner declaration public", function()
  local export_node = {
    type = function()
      return "export_statement"
    end,
    parent = function()
      return nil
    end,
  }
  local declaration = {
    parent = function()
      return export_node
    end,
  }
  h.eq(true, symbols._exported(declaration, "function api()", "typescript", "api"))
end)
