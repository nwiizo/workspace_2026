# Blog notes

## Thesis

OWASP ZAP is easiest to explain when the target is scoped tightly. This lab
keeps both the vulnerable Rust library service and the scanner inside Docker
Compose, then publishes reports as local artifacts.

## Baseline versus full scan

- Baseline scan: spider plus passive scanning. Good for CI smoke checks and
  production-safe checks when the scope is approved.
- Full scan: spider plus active scanning. It sends attack payloads and should
  be limited to local labs, staging, or explicitly authorized targets.

## Suggested article structure

1. Scope and ethics
2. Lab architecture
3. Vulnerable app walkthrough
4. Passive baseline scan
5. Active full scan
6. Reading reports without over-trusting automation
7. Agent skill triage with Codex or Claude Code
8. CI/CD guardrails

## Manual probes

Use these only against the local lab:

```text
http://127.0.0.1:18080/search?q=<script>alert(1)</script>
http://127.0.0.1:18080/book?id=1%20OR%201=1
http://127.0.0.1:18080/download?file=../secret-config.txt
```

For the login form, try username `librarian' --` with any password.

## Expected finding themes

ZAP versions and add-ons change, so exact names may differ. Expect findings
around these themes:

- Missing or weak HTTP security headers
- Cookie flags
- Reflected XSS
- SQL injection
- Path traversal
- Anti-CSRF token absence
- Application error disclosure

## Agent skill integration

Do not make the skill the thing that decides scan scope. Keep scope checks in
`scripts/run-zap.sh`, then hand the agent a compact summary:

```sh
make zap-summary
```

The triage skill should:

- Read `reports/zap-findings-summary.md` first.
- Load raw JSON only for exact evidence.
- Map High and Medium findings to Rust routes and line numbers.
- Separate confirmed findings from false positives and scan setup issues.
- Avoid applying fixes unless asked.

Suggested skill entrypoint:

```text
skills/zap-triage/SKILL.md
```
