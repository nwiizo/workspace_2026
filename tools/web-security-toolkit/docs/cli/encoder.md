# encoder

Multi-format encoder/decoder CLI for web security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/encoder
```

## Usage

### Encode Data

```bash
# Z85 encoding
encoder encode z85 "text"

# Base64 encoding
encoder encode base64 "secret"

# Base64 URL-safe encoding
encoder encode base64url "data"

# Hex encoding
encoder encode hex "binary"
```

### Decode Data

```bash
# Z85 decoding
encoder decode z85 "encoded"

# Base64 decoding
encoder decode base64 "c2VjcmV0"

# Hex decoding
encoder decode hex "68656c6c6f"
```

### ROT13 Transformation

```bash
encoder rot13 "hello"
# Output: uryyb
```

### Juice Shop Coupon Generator

Generate Z85-encoded coupon codes for OWASP Juice Shop:

```bash
encoder juice-coupon JAN 26 90
# Output:
# Coupon: JAN26-90
# Z85:    n<Michz3{y
```

Arguments:
- `month` - Month code (JAN, FEB, etc.) [default: JAN]
- `year` - 2-digit year [default: 26]
- `discount` - Discount percentage [default: 90]

## Supported Formats

| Format | Encode | Decode | Description |
|--------|--------|--------|-------------|
| z85 | Yes | Yes | ZeroMQ Base-85 encoding |
| base64 | Yes | Yes | Standard Base64 |
| base64url | Yes | No | URL-safe Base64 |
| hex | Yes | Yes | Hexadecimal |
| rot13 | Yes | - | Caesar cipher (self-inverse) |

## Use Cases

### CTF Challenges
- Decode obfuscated data
- Generate coupon codes
- Transform encoded payloads

### Security Testing
- Prepare payloads in different encodings
- Decode responses for analysis
- Bypass input filters using encoding
