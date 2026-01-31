# http-client

Security-focused HTTP client for web testing.

## Installation

```bash
cargo build --release
# Binary: target/release/http-client
```

## Usage

### GET Request

```bash
http-client get https://example.com
```

Options:
- `-H, --header`: Add header (can be used multiple times)
- `--jwt`: Add JWT token to Authorization header
- `--cookie`: Add cookie (can be used multiple times)
- `-f, --follow`: Follow redirects
- `--show-headers`: Show response headers
- `--status-only`: Only show status code

Examples:
```bash
# With custom headers
http-client get https://api.example.com -H "X-API-Key: abc123"

# With JWT authentication
http-client get https://api.example.com --jwt "eyJhbGci..."

# With cookies
http-client get https://example.com --cookie "session=abc123"

# Follow redirects
http-client get https://example.com -f

# Show response headers
http-client get https://example.com --show-headers
```

### POST Request

```bash
# JSON body
http-client post https://api.example.com -d '{"username":"admin","password":"test"}'

# Form data
http-client post https://example.com -F "user=admin" -F "pass=test"
```

Options:
- `-d, --data`: JSON request body
- `-F, --form`: Form data (key=value)
- `-H, --header`: Add header
- `--jwt`: JWT token
- `--cookie`: Add cookie
- `--show-headers`: Show response headers

### Custom Method Request

```bash
http-client request PUT https://api.example.com/resource -d '{"key":"value"}'
http-client request DELETE https://api.example.com/resource/123
http-client request PATCH https://api.example.com/resource -d '{"field":"updated"}'
```

Options:
- `-d, --data`: Request body
- `-c, --content-type`: Content-Type header
- `-H, --header`: Add header
- `--jwt`: JWT token
- `--show-headers`: Show response headers

### Analyze Cookies

```bash
http-client cookies https://example.com
```

Shows cookie details:
- Name and value
- Path and domain
- Secure, HttpOnly, SameSite flags
- Security issues detected

### Extract JSON Value

```bash
http-client json-extract https://api.example.com "data.token"
```

Extracts nested JSON values using dot notation.

## Security Features

### Cookie Analysis
Automatically detects cookie security issues:
- Missing Secure flag
- Missing HttpOnly flag
- Missing SameSite attribute

### JWT Support
Easy JWT token injection:
```bash
http-client get https://api.example.com --jwt "eyJhbGci..."
# Adds: Authorization: Bearer eyJhbGci...
```

### Response Formatting
- Automatic JSON pretty-printing
- Header display option
- Status code extraction

## Use Cases

### API Testing
- Send authenticated requests
- Test different HTTP methods
- Extract values from responses

### Security Testing
- Cookie security analysis
- JWT token testing
- Header manipulation

### CTF Challenges
- Quick HTTP requests
- Token-based authentication
- Response analysis
