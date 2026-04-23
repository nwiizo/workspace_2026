# OWASP ZAP remediation notes

This lab keeps both versions of the Rust/Axum library service in the repository.

| Version | Path | Purpose |
| --- | --- | --- |
| Before | `vulnerable-app/src/main.rs` | Intentionally vulnerable target for ZAP findings |
| After | `fixed-app/src/main.rs` | Remediated target for before/after comparison |

## Main fixes

- Reflected XSS: escape text nodes and quoted attributes with `html-escape`.
- SQL injection: replace SQL string concatenation with `rusqlite` placeholders.
- Path traversal: replace user-controlled path joins with an allow list.
- Error disclosure: stop rendering SQL and database errors into HTML.
- Cookie flags: add `HttpOnly` and `SameSite=Lax`.
- Security headers: add CSP, frame, content-type, permissions, and cross-origin policy headers through a shared response middleware.
- CSRF forms: add and validate a demo CSRF token. A production version should use a server-side session token store.

## Verification commands

```sh
cd tools/owasp-zap/fixed-app
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all

cd ..
make playwright-screenshots
make playwright-screenshots-fixed
ZAP_MAX_SCAN_MINUTES=3 ZAP_MAX_RULE_MINUTES=1 make zap-full
make zap-summary
ZAP_MAX_SCAN_MINUTES=3 ZAP_MAX_RULE_MINUTES=1 make zap-full-fixed
make zap-summary-fixed
```

## Latest comparison

The vulnerable app still produces the expected High findings:

- Cross Site Scripting (Reflected)
- SQL Injection
- SQL Injection - SQLite
- Path Traversal

The fixed app was rescanned with the same ZAP full scan settings. The High findings above pass after remediation. The remaining CSRF active-check warning is expected for this local sample because it does not implement a production-grade server-side session token store.
