---
name: zap-triage
description: Turn OWASP ZAP JSON reports into code-level remediation work for any authorized web application without launching unscoped scans.
argument-hint: [reports/zap-full.json]
disable-model-invocation: true
---

# ZAP Triage

Use this skill after an authorized OWASP ZAP scan has produced JSON output. It is intentionally generic: it can be used with Rust, Rails, Django, Node, Go, Java, or any other web stack as long as the report and source code are available.

## Safety Boundary

- Treat ZAP as evidence collection, not proof by itself.
- Do not run active scans unless the user gives an explicit target and authorization scope.
- Do not expand the scan scope from the report on your own.
- Do not apply fixes unless the user asks for implementation after triage and the affected files are clear.
- Prefer reading generated summaries before loading large raw HTML reports.

## Workflow

1. Confirm the ZAP report path, application root, target URL, and scan authorization scope.
2. Prefer a compact summary. If only raw JSON exists, summarize alert, risk, route, parameter, evidence, and instance count.
3. Group findings by route, risk, parameter, and likely code owner.
4. Map each High and Medium finding to source files, handler/controller/action names, and line numbers when possible.
5. For each finding, identify the vulnerable implementation pattern:
   - reflected output without context-aware escaping
   - SQL or query string concatenation
   - user-controlled filesystem path
   - missing shared response headers
   - missing or weak CSRF/session handling
   - authentication or authorization logic issue
   - scan configuration or authentication coverage gap
6. Propose the smallest safe remediation:
   - escape at the output boundary
   - use prepared statements or query builder parameters
   - use allow lists or canonical path containment
   - move headers into middleware or shared response code
   - add server-side CSRF/session token validation
   - add targeted authorization checks
7. Classify each item as:
   - confirmed
   - likely true positive, needs manual reproduction
   - likely false positive
   - scan configuration issue
8. Recommend the smallest verification command for each confirmed item.
9. When implementation is requested, keep patches finding-scoped and preserve unrelated behavior.

## Output

Return a compact triage report:

```markdown
# ZAP triage

## Confirmed

| Risk | Alert | Route | Source | Recommended fix |
| --- | --- | --- | --- | --- |

## Fix Pattern

| Pattern | Vulnerable code shape | Safer code shape |
| --- | --- | --- |

## Needs Manual Check

| Risk | Alert | Why | Next command |
| --- | --- | --- | --- |

## Scan Notes

- Scope:
- Report:
- Exclusions or skipped rules:
- Residual risk:
- Follow-up scan:
```

## Reusable Prompt

Use this prompt shape when invoking the skill manually:

```text
Use zap-triage.
Report: <path-to-zap-json-or-summary>
Application root: <path>
Target URL: <authorized-url>
Task: map High and Medium findings to source code and propose minimal remediations.
Do not run active scans unless I explicitly ask.
```
