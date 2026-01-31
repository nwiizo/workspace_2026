# totp-tool

TOTP/2FA utility for security testing.

## Installation

```bash
cargo build --release
# Binary: target/release/totp-tool
```

## Usage

### Generate TOTP Code

```bash
totp-tool generate JBSWY3DPEHPK3PXP
```

Output:
```
TOTP Code: 123456

Note: Code is valid for ~30 seconds
```

With time offset (for testing timing issues):
```bash
totp-tool generate JBSWY3DPEHPK3PXP --offset 30
# Generate code 30 seconds in the future
```

### Generate Time Window

Generate codes for multiple time intervals:

```bash
totp-tool window JBSWY3DPEHPK3PXP --size 2
```

Output:
```
=== TOTP Codes (window: +/-2 intervals) ===

  -2: 234567
  -1: 345678
   0: 456789 <-- current
  +1: 567890
  +2: 678901

Each interval is 30 seconds
```

### Analyze Secret

Analyze a TOTP secret format:

```bash
totp-tool analyze JBSWY3DPEHPK3PXP
```

Output:
```
=== Secret Analysis ===

Original:       JBSWY3DPEHPK3PXP
Normalized:     JBSWY3DPEHPK3PXP
Length:         16 characters
Valid Base32:   true
Decoded length: 10 bytes
Key strength:   80-bit (minimum, weak)

Current code:   456789
```

### 2FA Bypass Techniques

```bash
totp-tool bypasses
```

Shows common bypass methods:
- Response manipulation
- Token reuse
- Direct endpoint access
- Backup codes
- Race conditions

### Brute Force Codes

Generate common codes for testing:

```bash
# Show summary
totp-tool brute-force

# Output list for tools
totp-tool brute-force --list > codes.txt
```

### Juice Shop 2FA Challenge

```bash
totp-tool juice-shop
```

Shows:
- SQLi payload to extract TOTP secrets
- Step-by-step solution guide

## CTF Workflow

1. **Extract secret via SQLi**:
   ```bash
   totp-tool juice-shop
   # Get the SQLi payload
   ```

2. **Generate code**:
   ```bash
   totp-tool generate <extracted_secret>
   ```

3. **Use code to login**

## Use Cases

### Security Testing
- Test 2FA implementation
- Verify timing window handling
- Test bypass scenarios

### CTF Challenges
- TOTP code generation from leaked secrets
- 2FA bypass challenge solutions

### Pentesting
- Analyze TOTP implementations
- Test rate limiting on code entry
