このリポジトリの移行計画を策定してください。

以下の2つのエージェントを実行し、結果を `analysis/` ディレクトリに保存してください:

1. **legacy-code-analyzer** — コードベースのホットスポット × 複雑性マトリックスを作成し、サービス分離候補を特定
   → 結果を `analysis/code-analysis.md` に保存

2. **strangler-fig-migration-planner** — legacy-modernization.io の25パターンカタログから最適な移行パターンを選定し、Feature Parity Trap チェックを実施
   → 結果を `analysis/migration-plan.md` に保存

最後に、以下を含む **移行計画サマリー** を作成してください:
- 移行対象コンポーネントの優先順位（ビジネス価値 × 独立性 × リスク）
- 各コンポーネントの推奨移行パターン
- データ同期戦略
- Feature Parity Trap で除外すべき未使用機能
- 段階的移行スケジュールとロールバック計画

$ARGUMENTS がある場合はスコープとして使用してください（例: 特定のモジュールパス）。
