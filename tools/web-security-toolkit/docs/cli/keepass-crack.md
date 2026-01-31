# keepass-crack

KeePass KDBX (3.x) password cracker for CTF challenges.

## Installation

```bash
cargo build --release
# Binary: target/release/keepass-crack
```

## Usage

### Show File Information

```bash
keepass-crack info database.kdbx
```

Output:
```
File: database.kdbx
KDBX Version: 3.1
Cipher: AES-256
Compression: GZip
Transform Rounds: 6000

Header details:
  Master seed: 32 bytes
  Transform seed: 32 bytes
  ...
```

### Crack Password

```bash
# Basic wordlist (common passwords)
keepass-crack crack database.kdbx

# Extended wordlist
keepass-crack crack database.kdbx --extended

# Custom wordlist
keepass-crack crack database.kdbx --wordlist rockyou.txt

# Single password attempt
keepass-crack crack database.kdbx --password "test123"

# With key file
keepass-crack crack database.kdbx --keyfile image.jpg

# Show progress every N attempts
keepass-crack crack database.kdbx --progress 50
```

### Decrypt and View Contents

```bash
# Display decrypted content
keepass-crack decrypt database.kdbx -p "password"

# Save to file
keepass-crack decrypt database.kdbx -p "password" -o decrypted.xml
```

### Extract Credentials

```bash
# Table format (default)
keepass-crack extract database.kdbx -p "password"

# JSON format
keepass-crack extract database.kdbx -p "password" --format json

# CSV format
keepass-crack extract database.kdbx -p "password" --format csv
```

Output (table):
```
Found 3 entries:

Title                Username                  Password                       URL
----------------------------------------------------------------------------------------------------
Admin Panel          admin                     SuperSecret123!                https://admin.example.com
Email                john@example.com          MyEmailPass456                 https://mail.example.com
```

### Generate Wordlist

```bash
# Basic wordlist
keepass-crack wordlist

# Extended wordlist
keepass-crack wordlist --extended

# Save to file
keepass-crack wordlist -o passwords.txt
```

## CTF Tips

### Low Transform Rounds
If `transform_rounds` is very low (< 100), the file is likely created for CTF:
```
⚠️  WARNING: Very low transform rounds (1)!
   This file should be easy to crack.
```

### Key File Attacks
CTF challenges often use images or other files as key files:
```bash
# Try common files in the challenge directory
keepass-crack crack database.kdbx --keyfile logo.png
keepass-crack crack database.kdbx --keyfile hint.jpg
```

### Common CTF Passwords
The built-in wordlist includes:
- Common passwords (password, admin, 123456)
- Keyboard patterns (qwerty, asdf)
- Variations (P@ssw0rd, p4ssw0rd)
- CTF-specific (flag, ctf, challenge)

## Supported Formats

- KDBX 3.x (KeePass 2.x)
- AES-256 encryption
- GZip compression
- Password and/or key file authentication

## Warning

This tool is for authorized security testing only:
- CTF challenges
- Penetration testing (with permission)
- Password recovery (own databases)

Unauthorized access to password databases is illegal.

## Performance Notes

- Transform rounds significantly affect cracking speed
- Use `--progress` to monitor progress on large wordlists
- Key file hashing adds overhead
- Consider using hashcat/john for large-scale attacks
