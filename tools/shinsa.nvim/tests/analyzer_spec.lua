local h = require("tests.harness")
local analyzer = require("shinsa.analyzer")

h.test("analyzer builds a ranked queue from a real Git working tree", function()
  h.with_git_repo({
    ["service.lua"] = { "local function compute(value)", "  return value", "end", "", "return compute" },
  }, function(root)
    vim.fn.writefile({
      "local function compute(value)",
      "  return value + 1",
      "end",
      "",
      "return compute",
    }, root .. "/service.lua")

    local result
    local analyzer_err
    analyzer.run({
      root = root,
      base = "main",
      config = {
        git_command = { "git" },
        include_untracked = true,
        max_file_bytes = 1024 * 1024,
      },
    }, function(value, callback_err)
      result = value
      analyzer_err = callback_err
    end)
    h.truthy(
      vim.wait(5000, function()
        return result ~= nil or analyzer_err ~= nil
      end),
      "analysis timed out"
    )
    h.eq(nil, analyzer_err)
    h.eq(1, #result.changes)
    h.eq(1, result.changes[1].added)
    h.eq(1, result.changes[1].removed)
    h.eq(1, result.changes[1].ranges[1].new_count)
    h.eq(1, result.changes[1].ranges[1].old_count)
    h.truthy(#result.items >= 1)
    h.eq("service.lua", result.items[1].path)
    h.truthy(result.items[1].score >= 1)
  end)
end)

h.test("analyzer never executes configured Git textconv filters", function()
  h.with_git_repo({
    [".gitattributes"] = { "*.lua diff=shinsa-test" },
    ["service.lua"] = { "return 1" },
  }, function(root)
    local marker = root .. "/textconv-ran"
    local script = root .. "/textconv.sh"
    vim.fn.writefile({
      "#!/bin/sh",
      "touch " .. vim.fn.shellescape(marker),
      'cat "$1"',
    }, script)
    local changed, chmod_err = vim.uv.fs_chmod(script, 493)
    h.truthy(changed, chmod_err)
    h.command(root, { "git", "config", "diff.shinsa-test.textconv", script })
    vim.fn.writefile({ "return 2" }, root .. "/service.lua")

    local result
    local analyzer_err
    analyzer.run({
      root = root,
      base = "main",
      config = { git_command = { "git" }, include_untracked = false, max_file_bytes = 1024 * 1024 },
    }, function(value, callback_err)
      result = value
      analyzer_err = callback_err
    end)
    h.truthy(vim.wait(5000, function()
      return result ~= nil or analyzer_err ~= nil
    end))
    h.eq(nil, analyzer_err)
    h.eq(0, vim.uv.fs_stat(marker) and 1 or 0, "textconv filter must not execute")
  end)
end)

h.test("analyzer falls back to metadata when a patch exceeds its memory budget", function()
  h.with_git_repo({ ["large.lua"] = { "return 1" } }, function(root)
    vim.fn.writefile({ "return " .. string.rep("x", 2048) }, root .. "/large.lua")
    local result
    local analyzer_err
    analyzer.run({
      root = root,
      base = "main",
      config = { git_command = { "git" }, include_untracked = false, max_file_bytes = 32 },
    }, function(value, callback_err)
      result = value
      analyzer_err = callback_err
    end)
    h.truthy(vim.wait(5000, function()
      return result ~= nil or analyzer_err ~= nil
    end))
    h.eq(nil, analyzer_err)
    h.eq(1, #result.items)
    h.eq(true, result.items[1].file_level)
    h.eq("diff exceeds analysis limit", result.items[1].fallback_reason)
  end)
end)

h.test("analyzer keeps real binary, mode-only, and empty-file changes in the queue", function()
  h.with_git_repo({
    ["image.bin"] = { "plain text before" },
    ["run.sh"] = { "#!/bin/sh", "exit 0" },
  }, function(root)
    h.command(root, { "git", "config", "core.filemode", "true" })
    local binary = assert(io.open(root .. "/image.bin", "wb"))
    binary:write("changed\0binary")
    binary:close()
    local changed, chmod_err = vim.uv.fs_chmod(root .. "/run.sh", 493)
    h.truthy(changed, chmod_err)
    vim.fn.writefile({}, root .. "/empty.lua")

    local result
    local analyzer_err
    analyzer.run({
      root = root,
      base = "main",
      config = { git_command = { "git" }, include_untracked = true, max_file_bytes = 1024 * 1024 },
    }, function(value, callback_err)
      result = value
      analyzer_err = callback_err
    end)
    h.truthy(vim.wait(5000, function()
      return result ~= nil or analyzer_err ~= nil
    end))
    h.eq(nil, analyzer_err)
    local paths = {}
    for _, item in ipairs(result.items) do
      paths[item.path] = true
    end
    h.eq(true, paths["image.bin"])
    h.eq(true, paths["run.sh"])
    h.eq(true, paths["empty.lua"])
  end)
end)

h.test("untracked binary scanning obeys an aggregate I/O budget", function()
  h.with_git_repo({ ["tracked.lua"] = { "return true" } }, function(root)
    for index = 1, 12 do
      local handle = assert(io.open(string.format("%s/binary-%02d.bin", root, index), "wb"))
      handle:write("\0" .. string.rep("x", 49))
      handle:close()
    end
    local result
    local analyzer_err
    analyzer.run({
      root = root,
      base = "main",
      config = { git_command = { "git" }, include_untracked = true, max_file_bytes = 64 },
    }, function(value, callback_err)
      result = value
      analyzer_err = callback_err
    end)
    h.truthy(vim.wait(5000, function()
      return result ~= nil or analyzer_err ~= nil
    end))
    h.eq(nil, analyzer_err)
    local budget_fallbacks = 0
    for _, item in ipairs(result.items) do
      if item.fallback_reason == "untracked scan budget exceeded" then
        budget_fallbacks = budget_fallbacks + 1
      end
    end
    h.truthy(budget_fallbacks >= 1)
  end)
end)
