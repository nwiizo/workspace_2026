このリポジトリのアーキテクチャモダナイゼーション評価を実施してください。

以下の3つのエージェントをこの順序で実行し、結果を `analysis/` ディレクトリに保存してください:

1. **legacy-code-analyzer** — コードベースの複雑性・ホットスポット・結合度を分析
   → 結果を `analysis/01-code-analysis.md` に保存

2. **technical-debt-assessor** — Core Domain Chart の8パターンで投資判断を策定
   → 結果を `analysis/02-core-domain-chart.md` に保存

3. **modernization-strategist** — Modernization Strategy Selector（MSS）で戦略を策定
   → 結果を `analysis/03-strategy.md` に保存

最後に、3つの結果を統合した **エグゼクティブサマリー** を `analysis/00-executive-summary.md` に作成してください。サマリーには以下を含めること:
- 主要発見事項 Top 5
- 推奨アクション Top 3
- サブドメイン別の MSS 戦略一覧
- Core Domain Chart のパターン分類
- 最優先リファクタリング対象

$ARGUMENTS がある場合はスコープとして使用してください（例: 特定のディレクトリパス）。
