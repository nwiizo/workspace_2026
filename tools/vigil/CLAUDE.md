# vigil — Claude Code Security Audit Toolkit

Opus 4.6 のセマンティック推論で SAST が見逃す脆弱性を発見するツールキット。

## ディレクトリマップ

| パス | 内容 |
|------|------|
| `.claude/commands/security-audit.md` | 包括的セキュリティ監査オーケストレーション |
| `.claude/commands/threat-model.md` | STRIDE 脅威モデリング |
| `.claude/commands/attack-surface.md` | 攻撃面の列挙と分類 |
| `.claude/commands/webshell-hunt.md` | Web シェル・バックドア探索 |
| `.claude/commands/dangerous-functions.md` | 危険関数の全数検査 + Web 到達性判定 |
| `agents/vulnerability-assessor.md` | 脆弱性評価エージェント |
| `agents/compromise-investigator.md` | 侵害調査エージェント |
| `agents/remediation-planner.md` | 修正計画エージェント |
| `agents/threat-modeler.md` | 脅威モデリングエージェント |
| `skills/owasp-assessment.md` | OWASP Top 10 マッピングスキル |
| `docs/methodology.md` | 監査方法論リファレンス |

## 使い方

コマンドを対象プロジェクトの `.claude/commands/` にコピーして `/security-audit` 等で起動。
エージェントは Task ツールの prompt に貼り付けて使用。
