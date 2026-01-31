# Repository Guidelines

## Project Structure & Module Organization
workspace_2026 groups experiments and hardened tools. `samples/` is the lab; `samples/ory-hydra-rust` (DONADONA) mixes a Rust Axum backend, Next.js frontend (`frontend/`), and Hydra stack driven by `docker-compose.yml`, while `ory-keto-` and `ory-kratos-` directories host focused reproductions. Promote code that graduates from samples into `tools/`, such as `tools/rust-sql-anti-pattern`, where reusable crates live. `infrastructure/` captures security reproductions like `api-security-demo`, and `blogs/` stores narrative write-ups that cite the code. Each major directory ships a `CLAUDE.md` brief—review it before editing so your changes align with the intent of that area.

## Build, Test, and Development Commands
Run Rust hygiene from the crate root you are touching: `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings` followed by `cargo test --all`. Use `cargo run --release --bin <demo>` inside `infrastructure/api-security-demo` to exercise security samples (`bola-demo`, `mass-assignment-demo`, etc.). In `samples/ory-hydra-rust`, `docker compose up --build` starts Hydra/Kratos/Keto services; the backend launches with `cargo run` and the Next.js UI with `npm run dev` inside `frontend/`. Add new Docker services next to existing ones and document env vars in the local README.

## Coding Style & Naming Conventions
Rust files follow `rustfmt` defaults (4 spaces, snake_case modules, PascalCase types). Keep feature flags explicit in Cargo manifests. TypeScript/Next.js code uses the repository ESLint config via `npm run lint`; favor PascalCase components, camelCase hooks, and hyphenated file names for non-components. Bash scripts such as `samples/ory-hydra-rust/scripts/create-client.sh` should stay POSIX-compliant and annotated with usage comments.

## Testing Guidelines
Gate every change with `cargo test --all --all-targets`; integration tests live under each crate’s `tests/` directory. For Hydra flows, run the Playwright MCP routines described in `samples/ory-hydra-rust/CLAUDE.md` or reproduce the OAuth dance via the provided shell scripts so we know ID-token claims still match expectations. Frontend changes need at least `npm run lint` and a manual login round-trip through the locally running stack. Capture logs for auth failures and attach them to your PR.

## Commit & Pull Request Guidelines
Follow the existing Conventional Commit pattern (`feat(ory-hydra-rust): ...`, `fix(ory-hydra-rust): ...`). Squash noisy WIP fixes before opening the PR. Your PR description should state the scenario, commands used, and the sample/tool touched; link to blog posts or docs that must be updated. UI or API tweaks require screenshots or curl examples plus any new env variables. Reference the “Idea → Sample → Tools” lifecycle when proposing migrations so reviewers know whether a change is exploratory or production-ready.
