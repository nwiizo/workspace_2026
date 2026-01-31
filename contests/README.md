# Contests

Programming contests and security challenges.

## Contents

| Directory | Description | Status |
|-----------|-------------|--------|
| `juice_shop/` | OWASP Juice Shop CTF - Web security challenges | Active |
| `cp/` | Competitive Programming (AtCoder, Codeforces, etc.) | - |

---

## Competitive Programming

See [cp/README.md](cp/README.md) for details.

### Directory Structure

```
cp/
├── atcoder/
│   ├── abc300/       # AtCoder Beginner Contest 300
│   ├── arc180/       # AtCoder Regular Contest 180
│   └── typical90/    # 競プロ典型90問
├── codeforces/
│   └── round900/
├── leetcode/
│   ├── daily/
│   └── contests/
├── aoj/
└── yukicoder/
```

### Tools

```bash
# online-judge-tools (oj)
pip install online-judge-tools

# cargo-compete (Rust)
cargo install cargo-compete

# atcoder-cli (acc)
npm install -g atcoder-cli
```

### Workflow (Rust + cargo-compete)

```bash
# Setup
cargo compete init atcoder
cd atcoder

# Download contest
cargo compete new abc300

# Test solution
cargo compete test a

# Submit
cargo compete submit a
```

### Workflow (oj)

```bash
# Download test cases
oj download https://atcoder.jp/contests/abc300/tasks/abc300_a

# Test locally
oj test -c "cargo run --bin a"

# Submit
oj submit https://atcoder.jp/contests/abc300/tasks/abc300_a a.rs
```

### Templates

**Rust**
```rust
use proconio::input;

fn main() {
    input! { n: usize }
    println!("{}", solve(n));
}

fn solve(n: usize) -> usize {
    n
}
```

**Python**
```python
import sys
input = sys.stdin.readline

def main():
    n = int(input())
    print(n)

if __name__ == "__main__":
    main()
```

---

## OWASP Juice Shop

### Quick Start

```bash
# Start Docker runtime (macOS with Colima)
colima start

# Run Juice Shop
docker run -d -p 3000:3000 --name juice-shop bkimminich/juice-shop

# Open in browser
open http://localhost:3000
```

### Management

```bash
# Check status
docker ps --filter name=juice-shop

# Stop
docker stop juice-shop

# Start (after stop)
docker start juice-shop

# Remove and reset
docker rm -f juice-shop
docker run -d -p 3000:3000 --name juice-shop bkimminich/juice-shop
```

### Access Points

| URL | Description |
|-----|-------------|
| http://localhost:3000 | Main application |
| http://localhost:3000/#/score-board | Challenge scoreboard |
| http://localhost:3000/#/administration | Admin panel (requires login) |

## Adding New Contests

Create a subdirectory for each contest with:
- `README.md` - Contest overview, rules, and tools
- `CLAUDE.md` - Project-specific instructions (optional)
