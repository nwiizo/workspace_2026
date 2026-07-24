local h = require("tests.harness")
local config = require("herdr.config")
local context = require("herdr.context")

local function buffer(lines, name, filetype)
  local bufnr = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.api.nvim_buf_set_name(bufnr, name or (vim.fn.getcwd() .. "/src/example.lua"))
  vim.bo[bufnr].filetype = filetype or "lua"
  vim.bo[bufnr].modified = false
  return bufnr
end

local function reset(opts)
  config._reset()
  config.setup(opts)
  context._reset()
end

h.test("context formats a bounded line range with path and filetype", function()
  reset()
  local bufnr = buffer({ "local x = 1", "return x" })
  local text, err = context.range(bufnr, 1, 2)
  h.eq(nil, err)
  h.contains(text, "src/example.lua (lines 1-2)")
  h.contains(text, "```lua\nlocal x = 1\nreturn x\n```")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context preserves partial character selections", function()
  reset()
  local bufnr = buffer({ "abcdef", "ghijkl" })
  local text = context.range(bufnr, 1, 2, { mode = "v", start_col = 2, end_col = 3 })
  h.contains(text, "cdef\nghij")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("explicit ranges stay linewise while visual ranges require matching marks", function()
  reset()
  local bufnr = buffer({ "abcdef", "ghijkl" })
  vim.api.nvim_buf_set_mark(bufnr, "<", 1, 2, {})
  vim.api.nvim_buf_set_mark(bufnr, ">", 2, 3, {})

  local linewise = context.range(bufnr, 1, 2)
  h.contains(linewise, "abcdef\nghijkl")

  local visual = context.range_from_marks(bufnr, 1, 2, { mode = "v" })
  h.contains(visual, "cdef\nghij")

  local result, err = context.range_from_marks(bufnr, 1, 1, { mode = "v" })
  h.eq(nil, result)
  h.contains(err, "marks do not match")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context preserves UTF-8 endpoints for character selections", function()
  reset()
  local bufnr = buffer({ "αあい🙂z", "前🙂後" })
  local single = context.range(bufnr, 1, 1, {
    mode = "v",
    start_col = #"α",
    end_col = #"αあ",
  })
  h.contains(single, "\nあい\n```")

  local multiple = context.range(bufnr, 1, 2, {
    mode = "v",
    start_col = #"αあい",
    end_col = #"前",
  })
  h.contains(multiple, "🙂z\n前🙂")
  h.truthy(vim.str_utfindex(multiple, "utf-32", #multiple) > 0)
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context handles block selections containing tabs and wide characters", function()
  reset()
  local bufnr = buffer({ "\tあx", "\tいy" })
  local text = context.range(bufnr, 1, 2, {
    mode = "\22",
    start_col = 1,
    end_col = 1,
  })
  h.contains(text, "あ\nい")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context uses absolute paths for agents in another workspace", function()
  reset()
  local name = vim.fn.getcwd() .. "/src/example.lua"
  local bufnr = buffer({ "return true" }, name)
  local text = context.range(bufnr, 1, 1, { target_cwd = "/another/project" })
  h.contains(text, "@" .. vim.fs.normalize(name))
  text = context.range(bufnr, 1, 1, { target_cwd = vim.fn.getcwd() })
  h.contains(text, "@src/example.lua")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context chooses a safe fence when selected text contains backticks", function()
  reset()
  local bufnr = buffer({ "```", "value" })
  local text = context.range(bufnr, 1, 2)
  h.contains(text, "````lua")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("context rejects oversized ranges instead of truncating", function()
  reset({ context = { max_lines = 1, max_bytes = 64 } })
  local bufnr = buffer({ "one", "two" })
  local text, err = context.range(bufnr, 1, 2)
  h.eq(nil, text)
  h.contains(err, "maximum is 1")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("file context rejects unnamed and modified buffers", function()
  reset()
  local unnamed = vim.api.nvim_create_buf(true, false)
  local text, err = context.file(unnamed)
  h.eq(nil, text)
  h.contains(err, "no file name")

  local modified = buffer({ "changed" })
  vim.api.nvim_buf_set_lines(modified, 0, -1, false, { "new change" })
  text, err = context.file(modified)
  h.eq(nil, text)
  h.contains(err, "unsaved changes")
  vim.api.nvim_buf_delete(unnamed, { force = true })
  vim.api.nvim_buf_delete(modified, { force = true })
end)

h.test("diagnostic context uses one-based positions and stable metadata", function()
  reset()
  local bufnr = buffer({ "bad()" })
  context._set_diagnostic_get(function(target)
    h.eq(bufnr, target)
    return {
      {
        lnum = 0,
        col = 3,
        severity = vim.diagnostic.severity.ERROR,
        source = "lua_ls",
        code = "E1",
        message = "bad\ncall",
      },
    }
  end)
  local text, err = context.diagnostics(bufnr)
  h.eq(nil, err)
  h.contains(text, "- 1:4 ERROR [lua_ls/E1] bad call")
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)

h.test("diagnostic context reports an empty set", function()
  reset()
  local bufnr = buffer({ "ok()" })
  context._set_diagnostic_get(function()
    return {}
  end)
  local text, err = context.diagnostics(bufnr)
  h.eq(nil, text)
  h.eq("buffer has no diagnostics", err)
  vim.api.nvim_buf_delete(bufnr, { force = true })
end)
