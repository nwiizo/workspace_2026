#!/usr/bin/env bash
# Read results/live/{saves.tsv,stdout.log} written by scripts/live-patch.sh.
# For each save: the first output line whose counter step matches the edit,
# the time from the save (file mtime) to that line, and the longest gap
# between output lines until the next save.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
out="$here/results/live"

awk -F'\t' '
  NR == FNR { save_ms[++n] = $1; label[n] = $2; next }
  {
    # <ms> iter=<i> [live] counter = <c> [extra = <e>]
    ms = $1 + 0; split($0, f, " "); c = f[6] + 0
    line_ms[++m] = ms; counter[m] = c; text[m] = $0
  }
  END {
    for (s = 1; s <= n; s++) {
      start = save_ms[s]; stop = (s < n) ? save_ms[s + 1] : 1e18
      want = ""
      if (label[s] ~ /^step\+=/) { want = substr(label[s], 7) + 0 }
      if (label[s] ~ /:\+=/) { want = substr(label[s], index(label[s], ":+=") + 3) + 0 }
      hit = ""; gap = 0; prev = 0; first_after = 0
      for (i = 2; i <= m; i++) {
        if (line_ms[i] <= start || line_ms[i] >= stop) continue
        if (!first_after) first_after = i
        if (prev && line_ms[i] - prev > gap) gap = line_ms[i] - prev
        prev = line_ms[i]
        if (want != "" && hit == "" && counter[i] - counter[i - 1] == want) hit = i
      }
      printf "%-26s", label[s]
      if (want != "") {
        if (hit != "") printf " applied after %5d ms (counter %d -> %d)", line_ms[hit] - start, counter[hit - 1], counter[hit]
        else printf " not applied before the next save"
      }
      printf "  longest gap %d ms", gap
      if (first_after) printf "  first line: %s", text[first_after]
      printf "\n"
    }
  }
' "$out/saves.tsv" "$out/stdout.log"
