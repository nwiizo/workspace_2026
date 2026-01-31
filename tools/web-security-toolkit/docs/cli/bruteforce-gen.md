# bruteforce-gen

Brute force utilities for security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/bruteforce-gen
```

## Usage

### Common PIN Patterns

```bash
# Show summary
bruteforce-gen pins

# Output list for tools
bruteforce-gen pins --list > pins.txt
```

Includes:
- Sequential: 0000, 1234, etc.
- Years: 1950-2025
- Dates: MMDD format
- Common patterns: 6969, 0007, etc.

### Numeric Sequences

Generate zero-padded numeric sequences:

```bash
# 4-digit PINs 0000-9999
bruteforce-gen numeric 4 0 9999 --list > all_pins.txt

# 6-digit codes 000000-999999
bruteforce-gen numeric 6 0 999999 --list > codes.txt

# Show preview
bruteforce-gen numeric 4 0 100
```

### Rate Limit Bypass

```bash
bruteforce-gen rate-limit
```

Shows:
- X-Forwarded-For rotation
- X-Real-IP / X-Client-IP / True-Client-IP
- Case variation
- Parameter manipulation

### IP Rotation

Generate IPs for header rotation:

```bash
# Preview
bruteforce-gen ip-rotation 100

# Full list
bruteforce-gen ip-rotation 1000 --list > ips.txt
```

Usage:
```bash
curl -H "X-Forwarded-For: 0.0.0.1" https://target.com/login
```

### Security Question Wordlist

Generate answer candidates:

```bash
# Pet names (includes Juice Shop answers)
bruteforce-gen security-question pet

# Other types
bruteforce-gen security-question city
bruteforce-gen security-question mother
bruteforce-gen security-question school
bruteforce-gen security-question company
bruteforce-gen security-question sibling

# Output as list
bruteforce-gen security-question pet --list > pets.txt
```

### Username Enumeration

```bash
bruteforce-gen enumeration
```

Shows indicators to look for:
- Response time differences
- Error message differences
- Status code differences
- Response size differences

### Token Pattern Analysis

```bash
bruteforce-gen token-patterns
```

Shows predictable token patterns:
- Sequential tokens
- Timestamp-based
- Predictable UUIDs
- User ID based
- Email hash based

### Alphanumeric Combinations

Generate combinations (warning: can be huge):

```bash
# 3-character alphanumeric (limited to 1000)
bruteforce-gen alphanumeric 3

# Custom charset
bruteforce-gen alphanumeric 4 --charset "0123456789"

# Increase limit
bruteforce-gen alphanumeric 2 --charset "abc" --max 10000
```

## Brute Force Methodology

### 1. Information Gathering
```bash
# Check for enumeration
bruteforce-gen enumeration
```

### 2. Generate Wordlists
```bash
# PINs
bruteforce-gen pins --list > pins.txt

# Security questions
bruteforce-gen security-question pet --list > answers.txt
```

### 3. Bypass Rate Limits
```bash
# Get bypass headers
bruteforce-gen rate-limit

# Generate IPs
bruteforce-gen ip-rotation 1000 --list > ips.txt
```

### 4. Execute Attack
Use with tools like hydra, ffuf, or custom scripts.

## Use Cases

### CTF Challenges
- PIN brute force
- Security question answers
- Token prediction

### Security Testing
- Rate limit testing
- Enumeration detection
- Password policy testing

### Penetration Testing
- Credential stuffing preparation
- Account enumeration
- 2FA bypass attempts
