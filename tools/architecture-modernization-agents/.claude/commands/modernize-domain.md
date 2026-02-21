このリポジトリのドメイン境界分析を実施してください。

以下の3つのエージェントをこの順序で実行し、結果を `analysis/` ディレクトリに保存してください:

1. **domain-discovery-facilitator** — コードからドメインイベントを抽出し、6つのサブドメイン境界ヒューリスティックとピボタルイベント分析でサブドメイン境界を提案
   → 結果を `analysis/domain-discovery.md` に保存

2. **business-capability-mapper** — ISH（Independent Service Heuristics）の10問で各サブドメイン候補の独立サービス適性を評価
   → 結果を `analysis/capability-map.md` に保存

3. **bounded-context-designer** — Vlad Khononov の結合モデル（4種類 + Pain 公式）で結合分析を行い、Bounded Context Canvas（11セクション）を作成
   → 結果を `analysis/bounded-contexts.md` に保存

最後に、3つの結果を統合した **ドメイン境界サマリー** を作成してください:
- サブドメイン一覧と Core/Supporting/Generic 分類
- ISH スコア上位の独立サービス候補
- Context 間の結合マップと Pain 評価
- ユビキタス言語の不一致箇所

$ARGUMENTS がある場合はスコープとして使用してください（例: 特定のディレクトリパス）。
