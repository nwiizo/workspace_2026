# hashids-tool

Hashids encoder/decoder for CTF and security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/hashids-tool
```

## Usage

### Encode Numbers

```bash
hashids-tool encode 1,2,3 --salt "my secret"
```

Options:
- `-s, --salt`: Salt for encoding [default: ""]
- `-m, --min-length`: Minimum length of output [default: 0]

### Decode Hashid

```bash
hashids-tool decode "abc123" --salt "my secret"
```

### Discover Salt

Try common salts to decode a hashid:

```bash
hashids-tool discover "someHashid"

# With expected values for validation
hashids-tool discover "someHashid" --expected 1,2,3
```

### List Known Salts

```bash
# Juice Shop salts only
hashids-tool salts

# All known salts (including common ones)
hashids-tool salts --all
```

## Juice Shop Mode

Special commands for OWASP Juice Shop challenges:

### Generate Imaginary Challenge Codes

```bash
hashids-tool juice-shop --imaginary
```

Output:
```
Generating imaginary challenge codes...

Salt: "this is my salt" | Default salt
Code: abcdef123...

Salt: "hashids" | Common salt
Code: xyz789...

Try submitting these codes at: /#/score-board
```

### Decode Continue Code

```bash
hashids-tool juice-shop --decode "yourContinueCode"
```

### Discover Salt for Continue Code

```bash
hashids-tool juice-shop --discover "yourContinueCode"
```

### Generate Continue Code

```bash
hashids-tool juice-shop --encode "1,2,3,4,5" --salt "this is my salt"
```

## How Hashids Work

Hashids encodes integers into short, unique strings:

```
[1, 2, 3] + salt "my secret" -> "abc123"
```

Features:
- Non-sequential (1 != a, 2 != b)
- URL-safe characters
- Deterministic (same input + salt = same output)
- Salt-dependent

## Common Salts

Juice Shop specific:
- `this is my salt`
- `hashids`
- ``  (empty)

General:
- `secret`
- `salt`
- `password`
- Application-specific values

## Use Cases

### CTF Challenges

**Imaginary Challenge** (Juice Shop):
1. Generate codes with different salts
2. Try submitting at score-board
3. Find the correct salt

**Continue Code Analysis**:
1. Get a continue code from the application
2. Try to decode with known salts
3. Understand the structure of challenge IDs

### Security Testing

- Identify hashids in URLs/parameters
- Discover salts through analysis
- Predict valid IDs

## Warning

Hashids is NOT encryption:
- It's obfuscation, not security
- Never use for sensitive data
- IDs can be decoded if salt is discovered
