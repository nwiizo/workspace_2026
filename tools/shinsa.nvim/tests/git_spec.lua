local h = require("tests.harness")
local git = require("shinsa.git")

h.test("git helpers parse NUL paths and reject traversal", function()
  h.eq({ "one.lua", "dir/two.lua" }, git.nul_paths("one.lua\0dir/two.lua\0"))
  h.eq({
    { path = "one.lua", status = "M" },
    { path = "new name.lua", status = "R" },
  }, git.name_status("M\0one.lua\0R100\0old name.lua\0new name.lua\0"))
  h.eq("/repo/dir/file.lua", git.join("/repo", "dir/file.lua"))
  h.eq(nil, git.join("/repo", "../escape.lua"))
  h.eq(nil, git.join("/repo", "/absolute.lua"))
end)

h.test("auto base detection ignores a stale origin HEAD and uses a valid fallback", function()
  h.with_git_repo({ ["tracked.lua"] = { "return 1" } }, function(root)
    h.command(root, { "git", "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/missing" })
    vim.fn.writefile({ "return 2" }, root .. "/tracked.lua")
    local result
    local snapshot_err
    git.snapshot(
      {
        git_command = { "git" },
        include_untracked = false,
        max_file_bytes = 1024 * 1024,
      },
      root,
      nil,
      function(value, callback_err)
        result = value
        snapshot_err = callback_err
      end
    )
    h.truthy(vim.wait(5000, function()
      return result ~= nil or snapshot_err ~= nil
    end))
    h.eq(nil, snapshot_err)
    h.eq("main", result.base)
  end)
end)

h.test("snapshot compares merge base with tracked and untracked working-tree changes", function()
  h.with_git_repo({ ["tracked.lua"] = { "return 1" } }, function(root)
    vim.fn.writefile({ "return 2" }, root .. "/tracked.lua")
    vim.fn.writefile({ "return 3" }, root .. "/untracked.lua")

    local result
    local snapshot_err
    git.snapshot(
      { git_command = { "git" }, include_untracked = true, max_file_bytes = 1024 * 1024 },
      root,
      "main",
      function(value, callback_err)
        result = value
        snapshot_err = callback_err
      end
    )
    h.truthy(
      vim.wait(5000, function()
        return result ~= nil or snapshot_err ~= nil
      end),
      "snapshot timed out"
    )
    h.eq(nil, snapshot_err)
    h.eq("main", result.base)
    h.contains(result.diff, "tracked.lua")
    h.contains(result.diff, "untracked.lua")
  end)
end)
