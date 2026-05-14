# kata-eval

Skill evaluation CLI for the Claude Code CLI. Reimplementation in Rust of
[mizchi/waxa](https://github.com/mizchi/skills/tree/main/tools/waxa) (which
itself sits on top of [microsoft/waza](https://github.com/microsoft/waza)).
Reads the same `eval.yaml` + `tasks/*.yaml` schema, adds the
empirical-prompt-tuning policy layer (self-report grader, RED/GREEN/REFACTOR
`iterate` loop with `ledger.yaml`, LLM-as-Judge, model comparison, skill A/B
variant).

> **Status**: 0.1 — schema-compatible with waxa 0.x. Binary is `kata`,
> crate is `kata-eval`.

## Why kata

- waxa/waza give you a clean declarative schema; what they leave to the
  operator is the *judgment policy* (how to iterate, when to stop, how to
  capture the executor's own report on what was ambiguous).
- `kata` wires that policy in: structured `Self-report` block grader,
  cumulative `ledger.yaml`, convergence (2× zero new unclear rules) and
  divergence (3× non-decreasing new-unclear count → rewrite the skill,
  don't keep patching).

### Conceptual background: prompt engineering as inference-time alignment

Traditional alignment (RLHF / DPO / Constitutional AI) updates model
weights — the difference between desired and actual output is converted
to a loss and back-propagated. At **inference time** the weights are
frozen, so no mathematical gradient flows. Yet a well-designed prompt —
explicit criteria, self-critique steps, references, iteration — produces
the same quality improvement as if the weights had been updated.

This is sometimes called "alignment at inference time," and there is a
small but growing body of work formalizing it as **natural-language
gradient descent**:

- [TextGrad](https://arxiv.org/abs/2406.07496) — backpropagation through
  natural-language feedback.
- [DSPy](https://github.com/stanfordnlp/dspy) — programmatic
  optimization of LM pipelines, including prompt parameters.
- [OPRO](https://arxiv.org/abs/2309.03409) — Large Language Models as
  Optimizers.
- self-refine / Reflexion families — iterative self-critique loops.

The intuition: a sentence like *"the previous output failed criterion X"*
is the natural-language analog of a loss term, and the model's resulting
shift in next-token distribution is the natural-language analog of a
weight update. Mechanistically, several papers
([Akyürek et al. 2022](https://arxiv.org/abs/2211.15661),
[von Oswald et al. 2022](https://arxiv.org/abs/2212.07677)) show that
Transformer attention layers *can* implement in-context computations
equivalent to gradient descent — so the prompt-as-optimizer framing has a
plausible mechanistic substrate, even though which specific circuits a
real model recruits remains an open question in mechanistic
interpretability.

kata makes this loop concrete on the Claude Code CLI:

| Natural-language gradient descent | kata implementation |
|------------------------------------|---------------------|
| loss signal                        | executor's `Unclear points` (Issue / Cause) |
| gradient direction                 | executor's `General Fix Rule` |
| differentiable scorer              | `text` / `code` / `llm` graders |
| optimizer step                     | one `iterate` iteration |
| optimization trajectory            | `ledger.yaml` (cumulative `known_rules`) |
| minimum reached                    | `[CONVERGED]` (2× zero new rules) |
| stationary / divergent point       | `[DIVERGENCE-SIGNAL]` (3× non-decreasing → rewrite the skill structure) |

The skill body is the parameter being optimized; the executor's
self-report is the gradient sample; the ledger is the integration of
those samples over the iteration trajectory. Convergence /divergence
detection is the standard "stop when the objective stops moving"
heuristic, except both signal and objective live in natural language.

## Install

```bash
cargo install --path tools/kata-eval
# or, ad-hoc:
cargo run --manifest-path tools/kata-eval/Cargo.toml -- <eval.yaml>
```

Requires the `claude` CLI on `PATH` and authenticated (OAuth or
`ANTHROPIC_API_KEY`). Override the binary location via `CLAUDE_BIN=...`.

## Commands

```bash
# single run (eval-level or .kata.yaml `defaults.model` picks the model)
kata <path/to/eval.yaml> [--task ID] [--model M]

# RED/GREEN/REFACTOR loop with cumulative ledger
kata iterate <path/to/eval.yaml> [--max 5] [--task ID]

# multi-model comparison (objective axes only — no LLM A-vs-B judge)
kata compare <path/to/eval.yaml> --models claude-sonnet-4-6,claude-opus-4-7

# skill A/B variant exploration
kata variant <path/to/eval.yaml> --base skill-current --candidate skill-rewrite
```

Backwards-compat: project config is `.kata.yaml` first, falling back to
`.waxa.yaml` / `.waza.yaml`, so kata coexists with the upstream tools.

## Project layout

```
your-repo/
├── .kata.yaml                       # (or .waxa.yaml / .waza.yaml)
└── evals/
    └── <skill>/
        ├── eval.yaml
        └── tasks/
            └── *.yaml
```

Minimum `.kata.yaml`:

```yaml
paths:
  skills: .
  evals: evals/
  results: results/
defaults:
  model: claude-sonnet-4-6
  timeout: 300
```

## Grader types

| Type          | Description |
|---------------|-------------|
| `text`        | Regex match / not-match against the output. |
| `code`        | rhai expression evaluated with `output` in scope. Accepts waxa-style shims: `len(x)`, `'a' in x`, `'a' not in x`, `'X'` string literals, `.includes(` → `.contains(`, `.length` → `.len`. |
| `self-report` | Structural assertions on the executor's `## Self-report` block: `require_present`, `require_all_phases_ok`, `max_unclear`, `max_retries`. |
| `llm`         | LLM-as-Judge against a free-form `rubric`. Returns `PASS / SCORE / REASON`. Honors `model`, optional `pass_threshold` (default 0.7). |

## Self-report block

`kata` appends a Self-report request to every executor prompt (unless the
task overrides with `expected.require_self_report: false`). The executor
must end its reply with:

```
## Self-report
### Phase trace
- <phase>: OK | stuck | skipped | missing — <reason>
### Unclear points
- Issue: ...
  Cause: ...
  General Fix Rule: ...
### Discretionary fill-ins
- ...
### Retries
<integer>
```

The parser is tolerant: empty sections, missing trailing periods, and `-`,
`—`, `–` between phase name and status all work.

## Iterate

`kata iterate` runs the eval up to `--max N` times, writes
`<eval-dir>/ledger.yaml` (cumulative `known_rules` + per-iteration
breakdown), and halts on:

- **CONVERGED** — 2 consecutive iterations with zero new unclear rules.
- **DIVERGENCE-SIGNAL** — 3 iterations of non-decreasing new-unclear count.
  This is the cue to rewrite the skill structure, not keep patching.

## Example

A ready-to-run example ships in `examples/echo-skill/`. From this repo:

```bash
cd tools/kata-eval/examples/echo-skill
cargo run --manifest-path ../../Cargo.toml -- evals/echo-skill/eval.yaml
```

## Notes on the code grader

The upstream uses JS `new Function(...)` with a Python compat shim. kata
embeds [rhai](https://rhai.rs) for a Rust-native sandbox:

- `len(x)` works for strings and arrays via registered functions.
- `'a' in s` / `'a' not in s` are translated to `s.contains("a")` / `!s.contains("a")`.
- Single-quoted string literals are converted to double-quoted (rhai
  reserves single quotes for char literals).
- `.trim()` is overridden to return a new string rather than mutate
  in-place, so `s.trim().split(...)` chains work.
- For anything richer (Python-style boolean ops, regex captures), use an
  `llm` grader or split into multiple assertions.

## Quality gate

```sh
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## License

MIT.
