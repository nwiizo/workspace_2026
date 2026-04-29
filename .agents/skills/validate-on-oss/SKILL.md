---
name: validate-on-oss
description: ツールを実際のOSSプロジェクト(bat, fd, ripgrep, eza, tokei)で検証する。ツールの品質・パフォーマンス検証時に使用。
disable-model-invocation: true
---

# Validate on OSS

ツールを実際の OSS プロジェクトで検証する。

## Test Projects (Rust)

| Project | Size | Clone |
|---------|------|-------|
| bat | M | `gh repo clone sharkdp/bat /tmp/bat` |
| fd | S | `gh repo clone sharkdp/fd /tmp/fd` |
| ripgrep | L | `gh repo clone BurntSushi/ripgrep /tmp/rg` |
| eza | M | `gh repo clone eza-community/eza /tmp/eza` |
| tokei | S | `gh repo clone XAMPPRocky/tokei /tmp/tokei` |

## Workflow
```bash
for proj in /tmp/bat /tmp/fd /tmp/eza; do
  echo "=== $proj ==="
  your-tool $proj/src
done
```

## Expected Outcomes
bat: A-B / fd: A / ripgrep: B-C / eza: B

## Red Flags
- 全プロジェクト同一グレード（ツールが鈍感）
- 品質コードが F（厳しすぎ）
- 実コードでクラッシュ
- 中規模で >30s
