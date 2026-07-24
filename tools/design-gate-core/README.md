# design-gate-core

Shared infrastructure for small design diagnostic cargo subcommands.

It intentionally does not contain tool-specific lint logic, AST extraction, CLI structs, or scoring formulas. A tool owns its findings and maps them into the shared primitives where useful.

## Provided API

- `Severity` and `Grade`, with stable `Low < Medium < High < Critical` ordering, severity weights, score-to-grade, and weighted-grade helpers.
- `IssueKey { issue_type, source, target }`, plus stable sort, dedup, and baseline diff helpers.
- Baseline helpers for git repo discovery, relative subpaths, temporary detached worktrees with a `Drop` guard, and a clear error when the analyzed path does not exist at the baseline ref.
- Gate helpers: threshold counting, JSON-ready `GateReport { passed, fail_on, failing }`, and a plain text formatter.
- `BlindSpot` / `BlindSpotManifest` with English/Japanese localized note helpers.
- Output helpers for common output options, localized text, and localized severity labels.
- Rust file walking that excludes `target/` and `.git/` and respects `.gitignore` through `git check-ignore --stdin`.
- Suppression helpers for `// <tool>-allow: <issue_type>` comments using `ra_ap_syntax` CST item boundaries.
- Cargo subcommand argument absorption and shared exit-code constants.

## Starting a New Tool

1. Add the dependency:

   ```toml
   design-gate-core = { path = "../design-gate-core" }
   ```

2. Keep tool-specific issue types local, but convert each finding to a core `IssueKey` for dedup and baseline diffing.

3. Use `Severity` directly for finding severity. Keep the tool's score formula local, then call `grade_from_score` or `grade_for_severities`.

4. Use `rust_files` for source discovery and pass a no-file hook if the tool reports that condition in human output.

5. Resolve suppressions with `apply_suppressions`, injecting the tool prefix and issue-type parser/matcher.

6. For `--baseline`, call `prepare_baseline_worktree(path, git_ref, tool_name)`, analyze `baseline_path()`, then diff current and baseline issue sets with `diff_issue_sets`.

7. For `--check`, count either all current issues or only new baseline issues with `gate_report`, then serialize or print the resulting `GateReport`.
