local h = require("tests.harness")
local parser = require("shinsa.diff_parser")

h.test("diff parser collects files, hunk ranges, and change counts", function()
  local files = parser.parse(table.concat({
    "diff --git a/lua/a.lua b/lua/a.lua",
    "index 1111111..2222222 100644",
    "--- a/lua/a.lua",
    "+++ b/lua/a.lua",
    "@@ -2,2 +2,3 @@",
    " context",
    "+added",
    "diff --git a/old.lua b/old.lua",
    "deleted file mode 100644",
    "--- a/old.lua",
    "+++ /dev/null",
    "@@ -1 +0,0 @@",
    "-gone",
  }, "\n"))
  h.eq(2, #files)
  h.eq("lua/a.lua", files[1].path)
  h.eq(3, files[1].added)
  h.eq(2, files[1].removed)
  h.eq({ old_start = 2, old_count = 2, new_start = 2, new_count = 3 }, files[1].ranges[1])
  h.eq("old.lua", files[2].path)
  h.eq(true, files[2].deleted)
  h.eq(0, files[2].added)
  h.eq(1, files[2].removed)
end)

h.test("diff parser keeps untracked and binary files visible", function()
  local files = parser.parse(table.concat({
    "diff --git a/new file.lua b/new file.lua",
    "new file mode 100644",
    "--- /dev/null",
    "+++ b/new file.lua",
    "@@ -0,0 +1 @@",
    "+return true",
    "diff --git a/logo.png b/logo.png",
    "Binary files a/logo.png and b/logo.png differ",
    "--- a/logo.png",
    "+++ b/logo.png",
  }, "\n"))
  h.eq("new file.lua", files[1].path)
  h.eq(true, files[1].new_file)
  h.eq("logo.png", files[2].path)
  h.eq(true, files[2].binary)
end)

h.test("NUL-safe metadata retains binary, mode-only, and empty changes without hunk headers", function()
  local files = parser.parse(
    table.concat({
      "diff --git a/image.png b/image.png",
      "Binary files a/image.png and b/image.png differ",
      "diff --git a/run.sh b/run.sh",
      "old mode 100644",
      "new mode 100755",
    }, "\n"),
    {
      { path = "image.png", status = "M" },
      { path = "run.sh", status = "M" },
      { path = "empty.lua", status = "A" },
    }
  )
  h.eq(3, #files)
  h.eq("image.png", files[1].path)
  h.eq(true, files[1].binary)
  h.eq("run.sh", files[2].path)
  h.eq(0, #files[2].ranges)
  h.eq("empty.lua", files[3].path)
  h.eq(true, files[3].new_file)
  h.eq(true, files[3].metadata_only)
end)

h.test("metadata allowlist rejects C-quoted patch paths without duplicating files", function()
  local path = "odd\tname.lua"
  local files = parser.parse(
    table.concat({
      'diff --git "a/odd\\tname.lua" "b/odd\\tname.lua"',
      '--- "a/odd\\tname.lua"',
      '+++ "b/odd\\tname.lua"',
      "@@ -1 +1 @@",
      "-return 1",
      "+return 2",
    }, "\n"),
    { { path = path, status = "M" } }
  )
  h.eq(1, #files)
  h.eq(path, files[1].path)
  h.eq(true, files[1].metadata_only)
end)
