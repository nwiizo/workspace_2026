# cargo-boundary

`cargo-boundary` detects dependency direction risks in Rust projects that use DDD,
Clean Architecture, or layered architecture.

It is not a one-off lint. It produces stable issue keys for baseline diffing,
scores design risk, exposes CI gates, emits AI-agent repair plans, and declares
blind spots so a clean report is not mistaken for a proof of good design.

## Quick Start

```bash
cargo boundary .
cargo boundary . --all
cargo boundary . --summary
cargo boundary . --json
cargo boundary . --ai
cargo boundary . --check
cargo boundary . --baseline origin/main --check --fail-on=high
cargo boundary . --blind-spots
cargo boundary . --layers
cargo boundary . --jp
```

The binary also works without Cargo's subcommand dispatch:

```bash
cargo-boundary . --json
```

Default human and AI detail lists hide Low severity findings. The header,
score, grade, baseline diff, JSON, and CI gate always use the full issue set.
Use `--all` to show Low details as well. `--json` and `--ai` always include the
blind-spot manifest.

## Issue Types

- `layer-violation`: a source layer depends on a target layer it is not allowed
  to depend on, such as `domain -> infrastructure`.
- `internal-crossing`: a module outside an owner boundary references
  `internal`, `_internal`, or `private` paths.
- `pub-leak`: a `pub` item has no observed references from other analyzed
  modules and may be better as `pub(crate)` or private.
- `forbidden-import`: a configured layer imports a forbidden crate directly,
  such as `domain` importing `sqlx` or `reqwest`.

Issue identity is stable:

```text
(issue_type, source, target)
```

Targets are normalized where possible into crate-relative form, so equivalent
`crate::`, `self::`, and `super::` references do not split baseline keys. That
key is used by `--baseline <GIT_REF>` to report new, resolved, and unchanged
issues.

## Scoring Model

Each grouped issue receives:

```text
score = dependency_depth x occurrence_count x volatility_multiplier
```

`dependency_depth` is the number of layers crossed in the wrong direction. For
non-layer issue types, depth is a fixed structural risk weight.

`occurrence_count` is the number of source locations with the same stable issue
key.

`volatility_multiplier` comes from `git log` over Rust files:

- Low: `1.0`
- Medium: `1.5`
- High: `2.0`
- Unknown: `1.0`

If git is unavailable, volatility becomes a declared blind spot and severity is
based on depth and occurrences only.

Severity thresholds:

- `Critical`: score `>= 8`
- `High`: score `>= 4`
- `Medium`: score `>= 2`
- `Low`: score `< 2`

Project grade is A-F from a 100-point score after severity penalties.

## boundary.toml

Place `boundary.toml` at the analyzed crate root.

```toml
[[layers]]
name = "domain"
rank = 0
paths = ["domain"]

[[layers]]
name = "application"
rank = 1
paths = ["application", "usecase", "usecases"]

[[layers]]
name = "infrastructure"
rank = 2
paths = ["infra", "infrastructure", "adapter", "repository"]

[[layers]]
name = "presentation"
rank = 3
paths = ["presentation", "handler", "api", "controller"]

[[forbidden_imports]]
layer = "domain"
crates = ["sqlx", "reqwest"]
```

Lower rank means inner policy. Dependencies are allowed from higher rank to
lower rank and within the same layer. For example,
`presentation -> application -> domain` is allowed, while
`domain -> infrastructure` is a violation.

You can make the contract explicit:

```toml
[[allow]]
from = "presentation"
to = "application"

[[allow]]
from = "application"
to = "domain"

[[allow]]
from = "infrastructure"
to = "domain"
```

`[[allow]]` rules are additive. They permit extra pairs without disabling the
rank-based rule above.

## Heuristic Layers

If `boundary.toml` is missing, cargo-boundary infers layers from module and
directory names:

- domain: `domain`, `core`, `entity`, `entities`, `model`, `models`
- application: `app`, `application`, `usecase`, `usecases`, `service`,
  `services`
- infrastructure: `infra`, `infrastructure`, `adapter`, `adapters`,
  `repository`, `repositories`, `persistence`, `db`
- presentation: `presentation`, `handler`, `handlers`, `api`, `controller`,
  `controllers`, `route`, `routes`, `web`

This is always declared as a blind spot. Use `--layers` to inspect the inferred
structure. Directory matches are preferred over file-stem/module-name matches,
so paths such as `infrastructure/repository/models.rs` remain classified by the
owning directory. Ambiguous matches are reported as blind-spot notes.

## Blind Spot Policy

Every JSON and AI report includes a blind-spot manifest. Human output includes
run-specific notes, and `--blind-spots` prints the full manifest.

Declared blind spots include:

- macro-expanded code and inactive `cfg` branches
- missing type resolution for re-exports, trait dispatch, and generated names
- approximate `pub-leak`: method calls are collected syntactically, but not
  type-resolved, so a matching bare name can hide an actually unused public item
- runtime coupling such as ordering, timing, protocols, and shared state
- heuristic layer inference when `boundary.toml` is absent
- missing volatility when git history is unavailable

## Suppression

Use a comment on the target line or directly above the item/reference:

```rust
// boundary-allow: layer-violation
use crate::infrastructure::db::Db;

use reqwest::Client; // boundary-allow: forbidden-import

// boundary-allow: pub-leak
pub struct IntentionallyExported;
```

Multiple issue types can be separated by commas, and `all` suppresses every
issue type. Suppression is resolved from Rust CST trivia around the enclosing
item/reference, so comments and string literals are not parsed as code.

## CI and Exit Codes

`--check` evaluates the full issue set, independent of whether Low details are
hidden. The default threshold is `--fail-on=high`; use `--fail-on=low`,
`medium`, or `critical` to change it. Text modes print an explicit line such as:

```text
check: FAIL (fail-on=high, 3 issue(s) at/above threshold)
```

JSON output includes:

```json
"gate": { "passed": false, "fail_on": "high", "failing": 3 }
```

Exit codes:

- `0`: analysis completed and the check gate passed or was not requested
- `1`: runtime error or `--check` gate failure
- `2`: command-line usage error from clap

## File Discovery

Rust files are discovered under the requested path while skipping `.git`,
`target`, and root `.gitignore` matches such as `vendor/` or cache directories.
If no Rust files remain, cargo-boundary prints
`no Rust files found under this path` before the grade.
