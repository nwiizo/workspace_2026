# vigil

Claude Code security audit toolkit — Opus 4.6 のセマンティック推論で SAST が見逃す脆弱性を発見する。

## Why vigil?

従来の SAST ツール（Semgrep, Bandit 等）はパターンマッチングに依存するため、以下を検出できない:

- **セカンドオーダー SQLi**: DB保存→取得→引用符なし使用の多段攻撃
- **ロジック脆弱性**: 認証バイパス、権限昇格、TOCTOU
- **攻撃チェーン**: 個別には低リスクだが連鎖すると Critical になる脆弱性群
- **難読化コード**: Base64/gzinflate/str_rot13 の多重エンコード
- **コンテキスト依存の欠陥**: エスケープはしているが引用符外で使用

vigil は Opus 4.6 の深い推論力でデータフロー・信頼境界・攻撃シナリオを分析し、SAST の盲点を補完する。

## インストール

```bash
# 対象プロジェクトにコマンドをコピー
cp -r vigil/.claude/commands/* YOUR_PROJECT/.claude/commands/

# エージェント・スキルは参照用（Task ツールの prompt に組み込んで使用）
```

## Slash Commands

| コマンド | 概要 |
|---------|------|
| `/security-audit` | 包括的セキュリティ監査。4エージェントを順に起動し統合レポートを生成 |
| `/threat-model` | STRIDE 脅威モデリング。エントリポイント→信頼境界→脅威→対策 |
| `/attack-surface` | 攻撃面の列挙。認証なしエンドポイント、外部入力、ファイルアップロード |
| `/webshell-hunt` | Web シェル・バックドア探索。難読化検出、不正配置、言語非依存 |
| `/dangerous-functions` | 危険関数の全数検査。言語別リスト→grep→Web 到達性判定 |

## Subagents

| エージェント | Model | Tools | 概要 |
|-------------|-------|-------|------|
| `vulnerability-assessor` | Opus | Read, Grep, Glob | セマンティック脆弱性分析。OWASP ID 付き |
| `compromise-investigator` | Opus | Read, Grep, Glob, Bash | 侵害調査。Web シェル検出、危険関数マトリクス |
| `remediation-planner` | Opus | Read, Grep, Glob | 修正計画。Before/After コード、Phase 分類 |
| `threat-modeler` | Opus | Read, Grep, Glob | 脅威モデリング。STRIDE、攻撃ツリー |

## Skills

| スキル | 概要 |
|--------|------|
| `owasp-assessment` | OWASP Top 10:2021 + API Security Top 10:2023 の全20カテゴリに対する検査・マッピング |

## OWASP 準拠

2つの OWASP 標準に完全対応:

### OWASP Top 10:2021（Web アプリケーション）
A01 Broken Access Control / A02 Cryptographic Failures / A03 Injection / A04 Insecure Design / A05 Security Misconfiguration / A06 Vulnerable Components / A07 Auth Failures / A08 Software & Data Integrity / A09 Logging & Monitoring / A10 SSRF

### OWASP API Security Top 10:2023
API1 BOLA / API2 Broken Authentication / API3 Broken Object Property Level Authorization / API4 Unrestricted Resource Consumption / API5 BFLA / API6 Unrestricted Access to Sensitive Business Flows / API7 SSRF / API8 Security Misconfiguration / API9 Improper Inventory Management / API10 Unsafe Consumption of APIs

各カテゴリの検査項目、CWE マッピング、grep パターン、Opus 4.6 深掘りポイントは [skills/owasp-assessment/SKILL.md](skills/owasp-assessment/SKILL.md) に定義。

## SAST との使い分け

| 観点 | SAST ツール | vigil (Opus 4.6) |
|------|-----------|-------------------|
| パターンマッチング | 得意（SAST に委譲） | 不要 |
| セカンドオーダー SQLi | 検出困難 | データフロー推論で検出 |
| ロジック脆弱性 | 検出不可 | コンテキスト理解で検出 |
| 認証フローの欠陥 | 限定的 | 全体フローを追跡 |
| 攻撃チェーン推論 | 不可 | 複数脆弱性の連鎖を推論 |
| 修正案の生成 | テンプレート | 既存コードに適合する修正 |
| 難読化コード解析 | ルール依存 | セマンティック理解で解読 |

**推奨**: Semgrep/Snyk で既知パターンを掃討 → vigil でロジック・チェーン・難読化を深掘り。

## 参考

- [docs/methodology.md](docs/methodology.md) — 監査方法論リファレンス
- [Anthropic: Automate security reviews with Claude Code](https://www.anthropic.com/news/automate-security-reviews-with-claude-code)

## License

MIT
