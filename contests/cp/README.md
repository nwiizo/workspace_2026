# Competitive Programming

競技プログラミングのコンテスト・問題集

## Directory Structure

大会名・問題集名をそのままディレクトリ名にする。

```
cp/
├── abc300/          # AtCoder Beginner Contest 300
├── abc301/          # AtCoder Beginner Contest 301
├── arc180/          # AtCoder Regular Contest 180
├── agc060/          # AtCoder Grand Contest 060
├── typical90/       # 競プロ典型90問
├── edpc/            # Educational DP Contest
├── tessoku/         # 競技プログラミングの鉄則
├── round900/        # Codeforces Round 900
├── div2_950/        # Codeforces Div.2 Round 950
├── weekly400/       # LeetCode Weekly Contest 400
└── cses/            # CSES Problem Set
```

## Naming Convention

| Platform | Format | Example |
|----------|--------|---------|
| AtCoder ABC | `abc{number}` | `abc300`, `abc350` |
| AtCoder ARC | `arc{number}` | `arc180` |
| AtCoder AGC | `agc{number}` | `agc060` |
| AtCoder 典型90 | `typical90` | `typical90` |
| AtCoder DP | `edpc` | `edpc` |
| AtCoder 鉄則 | `tessoku` | `tessoku` |
| Codeforces | `round{number}` / `div2_{number}` | `round900`, `div2_950` |
| LeetCode | `weekly{number}` / `biweekly{number}` | `weekly400` |
| CSES | `cses` | `cses` |

## Tools

```bash
# cargo-compete (Rust)
cargo compete new abc300
cargo compete test a
cargo compete submit a

# online-judge-tools (oj)
oj download URL
oj test -c "python main.py"
oj submit URL main.py
```
