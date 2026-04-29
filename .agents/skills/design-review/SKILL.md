---
name: design-review
description: 設計書を5.0点満点で評価。完全性・一貫性・明確性・実現可能性・リスク管理の5軸で矛盾検出・改善提案。設計レビュー時に使用。
disable-model-invocation: true
---

# Design Document Review

設計書を 5.0 点満点で評価し、矛盾検出・改善を行う。

## Evaluation (各1.0点)

| 軸 | 1.0 | 0.5 | 0.0 |
|---|---|---|---|
| **完全性** | 全セクション網羅 | 一部欠落 | 主要欠落 |
| **一貫性** | 矛盾なし | 軽微な不整合 | 重大な矛盾 |
| **明確性** | 曖昧さなし | 一部曖昧 | 意図不明多数 |
| **実現可能性** | 具体的で実装可能 | 一部検証不足 | 非現実的 |
| **リスク管理** | 網羅的 | 主要のみ | 記述なし |

スコア: 4.5+=本番OK / 3.5+=良好 / 2.5+=要改善 / <2.5=大幅見直し

## Required Sections

**必須**: 概要 / 背景・動機 / ゴール・非ゴール / 設計詳細 / 代替案
**推奨**: API定義 / データモデル / エラー処理 / テスト戦略 / マイグレーション
**任意**: パフォーマンス / セキュリティ / 可観測性 / ロールバック / タイムライン / 依存関係

## Review Process

### Phase 1: 構造チェック
必須セクション存在、用語統一性、図表参照整合性

### Phase 2: 内容チェック
記述の深さ、具体例の有無、判断根拠、前提条件の明示

### Phase 3: 矛盾検出

| パターン | 例 |
|---|---|
| ゴール vs 設計 | 「高可用性」なのに単一障害点 |
| 非ゴール vs 実装 | 非ゴールの内容が設計に混入 |
| 要件 vs 却下理由 | 代替案却下理由が要件と矛盾 |
| 性能 vs 設計 | 低レイテンシ要件で同期処理多用 |
| 用語不統一 | 「ユーザー」「利用者」「アカウント」混在 |

### Phase 4: ベストプラクティス照合
- アーキテクチャ: 単一責任、疎結合、Blast Radius局所化
- 運用: デプロイ戦略、SLO/SLI、ランブック
- データ: 冪等性、TTL、バックアップ、GDPR
- セキュリティ: 認証認可、入力バリデーション、TLS、OWASP Top 10
- テスト: Unit/Integration/E2E/負荷/カオスエンジニアリング

## Output Format

```markdown
# Design Review Report

## Score: X.X / 5.0
| 軸 | スコア | 判定 |
|---|---|---|
| 完全性 | X.X | ✅/⚠️/❌ |
...

## Critical Issues
- [ ] {issue} — {section} — {fix}

## Warnings
- [ ] {issue} — {section} — {improvement}

## Contradictions
- {sectionA} vs {sectionB} — {detail}

## Actions (priority order)
1. **[P0]** {action} — {reason}
```

## Auto-Improvement
スコア 4.5 未満 → Critical Issues の修正案提示 → ユーザー確認 → 適用 → 再評価（最大3回）

注意: 設計意図は変更しない。追記は `[ADDED BY REVIEW]` タグ付き。

## Reference Formats
- **Google Design Doc**: Context, Goals/Non-goals, Design, Alternatives, Cross-cutting
- **ADR**: Status, Context, Decision, Consequences
- **RFC**: Problem, Solution, Design, Drawbacks, Alternatives, Unresolved

## Anti-Patterns

| Pattern | Detection |
|---|---|
| 手段の目的化 | 「なぜ」なく「何を」だけ |
| 楽観バイアス | リスクが空 or 「特になし」 |
| スコープクリープ | 非ゴール内容が設計に混入 |
| 暗黙の前提 | 「〜とする」「〜と仮定」の欠如 |
| 過剰設計 | 現時点で不要な抽象化レイヤー |
