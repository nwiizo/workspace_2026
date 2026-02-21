このリポジトリの包括的なアーキテクチャモダナイゼーション評価を実施してください。

modernization-orchestrator のフル分析モードに従い、以下の4フェーズで全10エージェントを実行してください。

## Phase 1: 戦略とコンテキスト

以下を **並行** で実行し、結果を `analysis/` に保存:
- **modernization-strategist** → `analysis/01-strategy.md`
- **wardley-mapping-analyst** → `analysis/02-wardley-map.md`
- **technical-debt-assessor** → `analysis/03-core-domain-chart.md`

## Phase 2: ドメイン発見

Phase 1 の結果を参照しながら以下を実行:
- **domain-discovery-facilitator** → `analysis/04-domain-discovery.md`
- **business-capability-mapper** → `analysis/05-capability-map.md`
- **legacy-code-analyzer** → `analysis/06-code-analysis.md`

## Phase 3: 設計

Phase 2 の結果を参照しながら以下を実行:
- **bounded-context-designer** → `analysis/07-bounded-contexts.md`
- **team-topologies-advisor** → `analysis/08-team-topologies.md`

## Phase 4: 実行計画

全結果を参照しながら以下を実行:
- **platform-engineering-consultant** → `analysis/09-platform-assessment.md`
- **strangler-fig-migration-planner** → `analysis/10-migration-plan.md`

## Phase 5: 統合

全結果を統合して以下を作成:
- `analysis/00-executive-summary.md` — 整合性検証結果を含む総合レポート

$ARGUMENTS がある場合はスコープとして使用してください。
