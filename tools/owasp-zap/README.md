# OWASP ZAP local lab

This directory contains a local-only DAST lab for writing and verifying an
OWASP ZAP article.

The lab starts an intentionally vulnerable Rust/Axum app and scans it from the
official ZAP Docker image. The default target is the internal Compose URL
`http://vulnerable-app:5000`.

Do not point the active scan at systems you do not own or have explicit
permission to test.

## What is included

- A vulnerable Rust/Axum library lending app in `vulnerable-app/`
- A remediated before/after comparison app in `fixed-app/`
- Docker Compose wiring for the app and ZAP
- `make` targets for baseline and full scans
- Report output under `reports/`
- A reusable `zap-triage` agent skill example in `skills/zap-triage/`

The library app includes intentionally unsafe examples:

- Reflected XSS in `/search?q=...`
- SQL injection in `/book?id=...`
- SQL injection in `/login`
- Path traversal in `/download?file=...`
- Missing security headers
- Insecure session cookie attributes
- State-changing form without a CSRF token

## Commands

Start the vulnerable app:

```sh
make up
```

Open the app from the host:

```text
http://127.0.0.1:18080
```

Run a passive baseline scan:

```sh
make zap-baseline
```

Run a full active scan:

```sh
make zap-full
```

Run the remediated app and scan it:

```sh
make up-fixed
make zap-full-fixed
make zap-summary-fixed
```

Summarize the ZAP JSON for agent review:

```sh
make zap-summary
```

The full scan uses `zap/full-scan.conf` to skip the browser-based DOM XSS rule
in this local lab. The regular reflected XSS active rule still runs. The
wrapper also caps active scan duration with:

- `ZAP_MAX_RULE_MINUTES`, default `1`
- `ZAP_MAX_SCAN_MINUTES`, default `5`

Capture UI screenshots with Playwright CLI:

```sh
make playwright-screenshots
```

Stop the lab:

```sh
make down
```

Remove generated reports:

```sh
make clean
```

## Reports

Each scan writes HTML, Markdown, and JSON reports to `reports/`.

Expected files:

- `reports/zap-baseline.html`
- `reports/zap-baseline.md`
- `reports/zap-baseline.json`
- `reports/zap-full.html`
- `reports/zap-full.md`
- `reports/zap-full.json`
- `reports/zap-full-fixed.html`
- `reports/zap-full-fixed.md`
- `reports/zap-full-fixed.json`
- `reports/zap-findings-summary.md`
- `reports/zap-findings-summary-fixed.md`
- `reports/playwright-home.png`
- `reports/playwright-home-mobile.png`
- `reports/playwright-sqli.png`
- `reports/playwright-reflected-html.png`

ZAP can exit with code `2` when warnings are found. The wrapper treats exit
codes `0`, `1`, and `2` as completed scans because this lab is expected to
produce findings. Exit code `3` is treated as a scan/runtime failure.

## Scanning another authorized service

The scripts default to the local lab target. To scan another service reachable
from the ZAP container, set `ZAP_TARGET` and explicitly acknowledge the scope:

```sh
ALLOW_NON_LOCAL_TARGET=1 ZAP_TARGET=http://my-service:8080 make zap-baseline
ALLOW_NON_LOCAL_TARGET=1 ZAP_TARGET=http://my-service:8080 make zap-full
```

Use this only for services you own or have written authorization to test.

## Agent skill integration

The safest integration point is not "let an agent run ZAP against anything".
Use ZAP to create local evidence, then give the agent a small triage artifact.

The intended flow is:

1. Run `make zap-full` against the local lab or another explicitly authorized
   target.
2. Run `make zap-summary` to turn `reports/zap-full.json` into
   `reports/zap-findings-summary.md`.
3. Ask an agent to use the `skills/zap-triage/SKILL.md` playbook and map
   findings back to source code.
4. Decide which findings should be fixed, then run the relevant tests and ZAP
   scan again.

For Codex, point `skills.config[].path` at the `skills/zap-triage` directory.
For Claude Code, copy or symlink the same directory to
`.claude/skills/zap-triage` or `~/.claude/skills/zap-triage`.

Keep the allow-list in `scripts/run-zap.sh`; the skill should not be the
control that decides whether an external target is safe to scan.

## Blog reproduction flow

1. Explain the scope: local vulnerable app, Docker network, no third-party
   target.
2. Start the app with `make up`.
3. Show manual symptoms, for example:
   - `http://127.0.0.1:18080/search?q=<script>alert(1)</script>`
   - `http://127.0.0.1:18080/book?id=1%20OR%201=1`
   - `http://127.0.0.1:18080/download?file=../secret-config.txt`
4. Run `make zap-baseline` and describe passive findings.
5. Run `make zap-full` and describe active findings.
6. Run `make zap-summary` and show how an agent should consume the compact
   findings summary.
7. Compare the HTML/Markdown reports and note which findings require manual
   confirmation.
