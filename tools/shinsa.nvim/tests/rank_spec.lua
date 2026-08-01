local h = require("tests.harness")
local rank = require("shinsa.rank")

local function item(overrides)
  return vim.tbl_extend("force", {
    id = "default",
    path = "src/default.lua",
    line = 1,
    end_line = 10,
    kind = "fn",
    name = "default",
    signature = "function default()",
    text = "",
    contract = false,
    added = false,
    deleted = false,
    changed_lines = 1,
    is_test = false,
    exported = false,
    file_level = false,
  }, overrides)
end

h.test("ranking is explainable and follows changed test paths transitively", function()
  local items = rank.apply({
    item({
      id = "api",
      name = "api",
      path = "src/api.lua",
      text = "return helper()",
      contract = true,
      exported = true,
    }),
    item({ id = "helper", name = "helper", path = "src/helper.lua" }),
    item({
      id = "test",
      name = "test_api",
      path = "tests/api_spec.lua",
      text = "assert(api())",
      is_test = true,
    }),
  })
  local by_id = {}
  for _, value in ipairs(items) do
    by_id[value.id] = value
  end
  h.eq("high", by_id.api.risk)
  h.eq(1, by_id.api.test_count)
  h.eq(1, by_id.helper.fan_in)
  h.eq(1, by_id.helper.test_count)
  h.truthy(vim.tbl_contains(by_id.api.reasons, "contract"))
  h.truthy(vim.tbl_contains(by_id.helper.reasons, "fan-in:1"))
end)

h.test("ranking calls missing test paths a risk only when changed tests exist", function()
  local with_test = rank.apply({
    item({ id = "untested", name = "untested", path = "src/untested.lua" }),
    item({ id = "test", name = "test_other", path = "tests/other.lua", is_test = true, text = "other()" }),
  })
  local untested = with_test[1].id == "untested" and with_test[1] or with_test[2]
  h.truthy(vim.tbl_contains(untested.reasons, "no-test-path"))

  local without_tests = rank.apply({ item({ id = "alone", name = "alone" }) })[1]
  h.eq(nil, without_tests.test_count)
  h.truthy(not vim.tbl_contains(without_tests.reasons, "no-test-path"))
end)

h.test("ranking makes deletion-only fallback items visible attention risks", function()
  local ranked = rank.apply({
    item({
      id = "removed",
      kind = "file",
      file_level = true,
      removal = true,
      removed_lines = 12,
      changed_lines = 12,
    }),
  })[1]
  h.eq("medium", ranked.risk)
  h.truthy(vim.tbl_contains(ranked.reasons, "removed:12"))
end)

h.test("test reachability handles long dependency chains without recursion", function()
  local items = {}
  local count = 10000
  for index = 1, count do
    items[index] = item({
      id = "node" .. index,
      name = "node" .. index,
      path = "src/node" .. index .. ".lua",
      text = index < count and ("return node" .. (index + 1) .. "()") or "return true",
      is_test = index == 1,
    })
  end
  rank.apply(items)
  local last
  for _, value in ipairs(items) do
    if value.id == "node" .. count then
      last = value
      break
    end
  end
  h.truthy(last)
  h.eq(1, last.test_count)
end)
