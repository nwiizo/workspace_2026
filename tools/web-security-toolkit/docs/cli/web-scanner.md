# web-scanner

Web vulnerability scanner for security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/web-scanner
```

## Usage

### Full Vulnerability Scan

```bash
web-scanner scan https://example.com
```

Options:
- `--headers-only`: Only check security headers
- `--skip-paths`: Skip common paths check
- `-o, --output FILE`: Save report to markdown file
- `--json`: Output in JSON format

Example:
```bash
# Full scan with report
web-scanner scan https://example.com -o report.md

# Headers only
web-scanner scan https://example.com --headers-only

# JSON output
web-scanner scan https://example.com --json
```

### Check Security Headers

```bash
web-scanner check-headers https://example.com
```

Options:
- `--recommendations`: Show recommendations for missing headers

Output:
```
=== Security Headers Analysis ===

✓ [INFO] Content-Type
       Value: text/html; charset=utf-8
✗ [HIGH] Strict-Transport-Security
       Missing HSTS header
✗ [MED ] Content-Security-Policy
       Missing CSP header
```

### Check Cookies

```bash
web-scanner check-cookies https://example.com
```

Analyzes:
- Secure flag
- HttpOnly flag
- SameSite attribute
- Path and Domain settings

### Test CORS Configuration

```bash
web-scanner test-cors https://api.example.com --origin https://evil.com
```

Detects:
- Wildcard origins (`*`)
- Origin reflection vulnerabilities
- Null origin acceptance
- Credentials with wildcard

### Show Recommended Headers

```bash
web-scanner recommended-headers
```

## Scan Findings

Findings are categorized by severity:
- **Critical**: Immediate action required
- **High**: Important security issues
- **Medium**: Should be addressed
- **Low**: Minor improvements
- **Info**: Informational findings

## Security Headers Checked

| Header | Description |
|--------|-------------|
| Strict-Transport-Security | HSTS - Force HTTPS |
| Content-Security-Policy | CSP - Control resource loading |
| X-Content-Type-Options | Prevent MIME sniffing |
| X-Frame-Options | Clickjacking protection |
| X-XSS-Protection | XSS filter (legacy) |
| Referrer-Policy | Control referrer information |
| Permissions-Policy | Feature permissions |

## Report Format

Markdown reports include:
- Target URL and scan timestamp
- Findings summary by severity
- Detailed findings with evidence
- Recommendations

## Use Cases

### Security Assessment
- Quick security header audit
- Cookie security review
- CORS misconfiguration detection

### CI/CD Integration
- Automated security checks
- JSON output for parsing
- Exit codes for pass/fail

### Compliance
- Generate audit reports
- Track security improvements
