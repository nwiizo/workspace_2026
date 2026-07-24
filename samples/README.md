# samples/

The laboratory. Expect explosions.

## Projects

| Directory | Description | Tech |
|-----------|-------------|------|
| [ory-hydra-rust](./ory-hydra-rust/) | DONADONA - Gamified engineer assignment platform with OAuth2/OIDC | Rust, Axum, Next.js, Ory Hydra |
| [ory-hydra-verification](./ory-hydra-verification/) | Minimal OAuth2/OIDC flow verification (Login/Consent Provider) | Rust, Axum, Ory Hydra |
| [ory-kratos-verification](./ory-kratos-verification/) | Identity management verification with Ory Kratos | Docker, Ory Kratos, Hydra |
| [ory-keto-verification](./ory-keto-verification/) | Authorization (Zanzibar) verification with Ory Keto | Docker, Ory Keto |
| [cargo-mutants-sample](./cargo-mutants-sample/) | Mutation testing experiments | Rust, cargo-mutants |
| [rust-formal-verification](./rust-formal-verification/) | Rust formal verification tool evaluation with executable examples | Rust, Kani, Creusot, Prusti, Verus, Flux |
| [thirty-seconds-silence](./thirty-seconds-silence/) | Blog sample code: hypothesis-driven problem solving | Python |

## Highlights

### ory-hydra-rust (DONADONA)

Full-stack SaaS platform demonstrating:
- Multi-tenant architecture with per-tenant database schemas
- OAuth2/OIDC authentication via Ory Hydra
- Game mechanics: levels, XP, achievements, leaderboards
- Playwright MCP setup for E2E testing

### ory-hydra-verification

Minimal implementation of Hydra's Login/Consent Provider pattern:
- Context pattern for passing user data between providers
- Skip handling for returning sessions
- ID token claim injection

## What's Here

- Code snippets that answer "what if..."
- Library experiments before committing to them
- Minimal reproductions of bugs (for science)
- Things I'll probably forget I wrote

## Rules

1. No rules. This is the sandbox.
2. Okay, one rule: if it works, consider moving it to `tools/`
3. If it doesn't work, that's also fine. Learning happened.

## Lifecycle

```
Idea → Sample → Works? → tools/
                  ↓
               Doesn't work? → Delete or keep for laughs
```

## Note

Code quality here ranges from "surprisingly clean" to "what was I thinking". Both are acceptable.
