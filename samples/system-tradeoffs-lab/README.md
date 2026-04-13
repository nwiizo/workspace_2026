# system-tradeoffs-lab

Small Rust lab for a practical talk on:

- non-functional requirements
- distributed systems
- implementation tradeoffs

This sample focuses on two concrete themes:

1. `timeout / retry / idempotency`
2. `consistency / latency`

## Repository

- GitHub: https://github.com/nwiizo/workspace_2026
- Sample: https://github.com/nwiizo/workspace_2026/tree/main/samples/system-tradeoffs-lab

## What This Sample Verifies

This is not a full end-to-end system with HTTP, databases, or external services.
It is a local scenario lab that reproduces the behavior discussed in the slides.

Confirmed locally on `2026-04-13`:

- `cargo test`
  - `retry_without_idempotency_can_duplicate_the_charge`
  - `idempotency_stops_the_second_charge_after_timeout`
  - `replica_can_be_fast_but_stale_until_replication_happens`
- `cargo run`
  - scenario 1 shows duplicate charge without idempotency
  - scenario 2 shows safe retry with idempotency
  - scenario 3 shows fast but stale replica reads before replication
- `./smoke.sh`
  - checks that the CLI output contains the expected lines for all 3 scenarios

## Run

```sh
cargo run
```

## Test

```sh
cargo test
```

## Smoke Check

```sh
./smoke.sh
```
