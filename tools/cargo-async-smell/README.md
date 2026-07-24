# cargo-async-smell

`cargo-async-smell` is a cargo subcommand for production-risk async Rust design smells. It is intentionally closer to an operations readiness diagnostic than to Clippy: findings are scored for incident impact, how easy the condition is to hit, and file volatility from git history.

## Quick Start

```bash
cargo run -- .
cargo run -- --summary
cargo run -- --json
cargo run -- --ai
cargo run -- --check --fail-on high
cargo run -- --baseline HEAD~1 --check
cargo run -- --blind-spots
cargo run -- --jp
```

When installed as a cargo subcommand:

```bash
cargo async-smell --check
```

## Detected Issue Types

- `guard-across-await`: a `MutexGuard` / `RwLockGuard`-like binding is live across `.await`.
- `blocking-in-async`: configured blocking functions or method names such as `std::fs`, `std::net`, `std::thread::sleep`, `reqwest::blocking`, `ureq`, or `rusqlite` appear in async code.
- `unbounded-spawn`: `tokio::spawn` or a simple import alias appears in a loop and the `JoinHandle` is discarded.
- `detached-task`: `tokio::spawn` or a simple import alias contains an infinite `loop` without a visible cancellation boundary.
- `missing-timeout`: configured external-looking methods such as `connect`, `send`, `recv`, or `request` are not under `timeout` or in a request chain with `.timeout(...)`.

## Score Model

Severity uses the shared `design-gate-core` `Severity` and `Grade` types.

```text
severity = incident impact + condition ease + git volatility
```

Impact is ordered as deadlock > starvation/leak > latency. Condition is derived per finding, for example guard distance to `.await`, branch/loop context, spawn loop shape, or blocking call class. Git volatility is counted from changed file names in recent git history; if history is unavailable, the blind spot manifest states that severity used only the impact and condition axes.

## Runtime Support

`--runtime tokio` is the default and the only runtime analyzed in Wave 1. `--runtime async-std` and `--runtime smol` are accepted for CLI compatibility, but runtime-specific analysis is not implemented and is reported as a blind spot.

## Baseline And CI Gate

`--baseline <GIT_REF>` uses `design-gate-core` to create a temporary detached worktree with a Drop guard. Issue keys are stable and repo-relative:

```text
(issue_type, "rel_path:Type::method", target)
```

Free functions use `rel_path:function`. Anonymous async blocks use the enclosing function plus a function-local async-block index. Duplicate identities inside the same type or function still receive `#N` suffixes. Line numbers are not part of the key. With `--check`, the output includes a `check: PASS` or `check: FAIL` line. JSON output includes a `gate` object.

## Suppressions

Use a line or item-level comment:

```rust
// async-smell-allow: missing-timeout
async fn call(client: Client) {
    client.send().await;
}
```

`all` is also accepted by the shared suppression resolver.

## Configuration

Create `async-smell.toml` near the analyzed path:

```toml
blocking_calls = [
  "std::fs",
  "std::net",
  "std::thread::sleep",
  "reqwest::blocking",
]

timeout_methods = ["connect", "send", "recv", "request"]

allow = [
  # "missing-timeout",
]
```

The default lists are extended by configured `blocking_calls` and `timeout_methods`. `allow` disables issue types globally.

## Blind Spots

The tool uses `ra_ap_syntax` CST walking and does not invoke rustc type resolution. It resolves simple `use` aliases and grouped imports for spawn, timeout, drop paths, JoinSet/select checks, and path-style blocking calls such as `use std::fs; fs::create_dir_all(...)`. Glob imports, re-exports, type aliases, dynamic dispatch, and exact external I/O receiver classification remain blind spots.

`try_lock` / `try_read` / `try_write` are detected as guard acquisition, but Tokio and std-like locks use the same synchronous-looking syntax, so those findings are conservative and scored lower. Request-level timeout chains such as `client.get(url).timeout(d).send().await` are recognized; client-level default timeouts configured with builders are not propagated across functions. Tokio channel `mpsc` / `oneshot` / `watch` / `broadcast` send and recv calls are filtered heuristically when the receiver chain or nearby binding initializer contains channel markers. Blocking behavior hidden behind local helper functions is not traced. Use `--blind-spots` for the full manifest.
