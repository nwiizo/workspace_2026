local h = require("tests.harness")
local ledger = require("shinsa.ledger")

local items = {
  { id = "a", path = "src/a.lua", line = 3, signature = "function a()" },
  { id = "b", path = "src/b.lua", line = 8, signature = "function b()" },
}

h.test("ledger preserves matching items across refreshes in one scope", function()
  ledger._reset()
  ledger.reconcile("repo\0base", items)
  h.eq(true, ledger.toggle_reviewed("a"))
  ledger.set_concern("a", "Handle the error path")
  ledger.reconcile("repo\0base", { items[1] })
  h.eq(true, ledger.get("a").reviewed)
  h.eq("Handle the error path", ledger.get("a").concern)
  h.eq(nil, ledger.get("b"))
end)

h.test("ledger resets state when the review scope changes", function()
  ledger._reset()
  ledger.reconcile("repo\0old", items)
  ledger.toggle_reviewed("a")
  ledger.reconcile("repo\0new", items)
  h.eq(false, ledger.get("a").reviewed)
end)

h.test("ledger exports only concerns as agent-ready Markdown", function()
  ledger._reset()
  ledger.reconcile("repo\0base", items)
  ledger.toggle_reviewed("a")
  ledger.set_concern("b", "Check nil input")
  local markdown = ledger.markdown({ base = "main" }, items)
  h.contains(markdown, "## Shinsa follow-ups")
  h.contains(markdown, "Reviewed: 1/2 items")
  h.contains(markdown, "`src/b.lua:8`")
  h.contains(markdown, "Check nil input")
  h.truthy(not markdown:find("src/a.lua", 1, true))
end)

h.test("ledger keeps control characters and backticks from breaking Markdown structure", function()
  ledger._reset()
  local unusual = { { id = "odd", path = "odd\n`file.lua", line = 1, signature = "fn `odd`()" } }
  ledger.reconcile("repo\0base", unusual)
  ledger.set_concern("odd", "Review this")
  local markdown = ledger.markdown({ base = "main\nbranch" }, unusual)
  h.truthy(not markdown:find("odd\n", 1, true))
  h.contains(markdown, "odd 'file.lua")
  h.contains(markdown, "main branch")
end)
