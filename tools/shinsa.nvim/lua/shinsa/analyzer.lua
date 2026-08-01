local M = {}

function M.run(opts, callback, on_progress)
  local config = assert(opts.config, "config is required")
  local root = assert(opts.root, "root is required")
  on_progress = on_progress or function() end
  require("shinsa.git").snapshot(config, root, opts.base, function(snapshot, err)
    if not snapshot then
      callback(nil, err)
      return
    end
    local changes = require("shinsa.diff_parser").parse(snapshot.diff, snapshot.paths)
    local items = {}
    local index = 1

    local function process_next()
      local change = changes[index]
      if not change then
        snapshot.changes = changes
        local ok, ranked_items = pcall(require("shinsa.rank").apply, items)
        if not ok then
          callback(nil, "could not rank review items: " .. tostring(ranked_items))
          return
        end
        snapshot.items = ranked_items
        callback(snapshot)
        return
      end
      on_progress(string.format("Mapping changed symbols (%d/%d)", index, #changes))
      local ok, extracted = pcall(require("shinsa.symbols").extract, root, change, config)
      if not ok then
        callback(nil, string.format("could not analyze %s: %s", change.path, extracted))
        return
      end
      vim.list_extend(items, extracted)
      index = index + 1
      vim.schedule(process_next)
    end

    process_next()
  end, on_progress)
end

return M
