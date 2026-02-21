# Chirp — Twitter-like SNS

## Stack
- Leptos 0.8 (SSR + Hydration) + Axum 0.8
- SQLx 0.8 + PostgreSQL 17
- tower-sessions 0.14 + argon2 0.5
- Tailwind CSS v4

## Build
```sh
cargo leptos build
cargo leptos serve  # dev server on :3000
```

## Quality
```sh
cargo fmt
cargo clippy --features ssr -- -D warnings
cargo clippy --features hydrate -- -D warnings
```

## DB
```sh
docker compose up -d   # PostgreSQL on :5432
sqlx migrate run       # run migrations
cargo sqlx prepare --workspace  # offline mode cache
```

## Conventions
- No `.unwrap()` — use `?` + `AppError`
- UUID v7 for all IDs
- Cursor-based pagination
- Server functions in `src/server/`
- UI components in `src/components/`
