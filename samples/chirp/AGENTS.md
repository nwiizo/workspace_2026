# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the Leptos SSR front end alongside Axum server functions, with components living under `src/components/` and backend helpers under `src/server/`. Database migrations sit in `migrations/`, and Tailwind styles plus static assets live in `style/` and `public/`. Reusable demos, blog notes, and test assets remain under `e2e/`, `test-results/`, and `blog_*.md`. Keep experimental crates in `samples/` until they graduate to `tools/`, following the Idea → Sample → Tools lifecycle outlined in the workspace README.

## Build, Test, and Development Commands
Use `cargo leptos build` for a production bundle and `cargo leptos serve` for the combined SSR + hydration dev server on port 3000. Run `docker compose up -d` to provision PostgreSQL locally, followed by `sqlx migrate run` and `cargo sqlx prepare --workspace` any time schemas change. Standard hygiene is `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings`, then `cargo test --all --all-targets`. Security reproductions elsewhere in the repo follow the same Rust workflow plus scenario-specific commands documented in their local CLAUDE.md files.

## Coding Style & Naming Conventions
Rust adheres to `rustfmt` defaults (4-space indentation, snake_case modules, PascalCase types). Avoid `.unwrap()`; propagate failures with `?` and project-specific `AppError`. IDs must use UUID v7, and pagination stays cursor-based. Tailwind v4 powers styling—prefer utility classes close to their components and keep shared tokens in `style/`. Frontend TypeScript or Next.js samples rely on the workspace ESLint config via `npm run lint`.

## Testing Guidelines
Rust crates require `cargo test --all --all-targets` before every push, plus targeted integration suites under each crate’s `tests/`. Hydration-specific regressions need both `cargo clippy --features ssr` and `cargo clippy --features hydrate`. Database-affecting changes should rerun `sqlx migrate run` and snapshot the resulting schema. For Hydra-focused samples, replay the Playwright MCP flows in `samples/ory-hydra-rust/CLAUDE.md` to confirm OAuth claims; for chirp, smoke-test login and timeline rendering through the Leptos UI.

## Commit & Pull Request Guidelines
Follow Conventional Commits, e.g., `feat(chirp): add cursor pagination helper` or `fix(api-security-demo): guard mass assignment`. Each PR must state the scenario, commands executed (build, lint, tests, migrations), and any linked issues or blog posts that changed. Include screenshots or curl transcripts for UI/API tweaks and note new environment variables in the local README. Squash incidental WIP commits so reviewers see a single polished change set, and describe how the update advances an experiment toward tool hardening.

## Security & Configuration Tips
Store secrets in your shell environment rather than committing `.env` files. When running Docker services, ensure ports do not collide with other samples. Regenerate session keys and Argon2 salts for every new deployment, and audit dependency bumps for crates touching auth or crypto. Document any new configuration knobs in the relevant CLAUDE.md so future agents can reproduce your setup end to end.
