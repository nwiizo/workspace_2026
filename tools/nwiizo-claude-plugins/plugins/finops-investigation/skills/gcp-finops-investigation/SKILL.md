---
name: gcp-finops-investigation
description: GCP プロジェクトの FinOps 調査・コスト削減分析。Billing Export (BigQuery)、Active Assist Recommender、CUD/SUD の誤認パターン、プロジェクト/フォルダ/組織階層での集計、レポート数値の分母混在を避けるための学び集。
---

# GCP FinOps Investigation Skill

GCP 特有のコスト可視化・削減の学びのみを記載。一般的な GCP / FinOps 知識は含まない。

## Trigger

- GCP コスト削減・棚卸し調査
- 前回調査との差分確認、定期レビュー
- CUD / Spot VM / リザーブド購入判断、未使用リソース特定

## 参照するサブファイル

- [best_practices_2026.md](best_practices_2026.md) — FinOps Foundation 2026 Framework、業界ベンチマーク、クラウド非依存の原則
- [auth.md](auth.md) — 認証（gcloud ADC、サービスアカウント impersonation、OIDC）の罠
- [cost_analysis.md](cost_analysis.md) — Billing Export (BigQuery) の読み方、FinOps Hub、数値の分母混在回避
- [detection_patterns.md](detection_patterns.md) — Active Assist / Recommender の判定基準と見落とされがちなパターン
- [cud_sud_gotchas.md](cud_sud_gotchas.md) — CUD/SUD/Spot の誤認パターン、2026 年の仕様変更
- [reporting.md](reporting.md) — レポート出力規律、優先度付け、履歴運用

## Core Principles

- **ReadOnly 厳守**: 提案のみ、変更系操作は自動実行しない
- **数値の分母統一**: 月平均 / 月末換算 / N日実績を混ぜない。外部共有は月平均ベース基準
- **production ラベル/タグのリソースは慎重扱い**: 停止状態でも外部参照の可能性で担当者確認必須
- **権限は時間経過で変化する**: 前回使えた Recommender 等が今回 NG のことがある。調査冒頭で必ず再検証
- **独立レビューを最低1回**: 別エージェントで重複計上・警告漏れ・計算矛盾をチェック
- **学びは即座に CLAUDE.md / memory に反映**

## Phases

1. 認証確立（gcloud ADC または SA 鍵、非対話環境で impersonation する場合はトークン生成経路を明示）
2. 権限検証（Viewer/Billing Account Viewer/Recommender Viewer を冒頭で確認）
3. Billing Export の BigQuery で大枠把握（Detailed export がある前提で SKU 内訳、なければ Console Reports）
4. Recommender API で機械判定された削減候補取得（Idle VM、Unattached Disk、Idle Address、Idle Reservation）
5. リソース棚卸し（Compute / Cloud SQL / GCS / BigQuery の並列 describe）
6. 削減候補抽出（検出パターン + Recommender + CUD カバレッジ）
7. レポート作成（3段階優先度、重複除外、前回差分）
8. 独立レビュー（修正必須指摘に全対応）
9. 学びを memory / CLAUDE.md に反映

## Anti-Patterns

- Console Reports だけで分析 → SKU 単位の内訳が取れないので Billing Export の BigQuery を使う
- 「SUD が効いている」と言って CUD 購入を検討しない → SUD は自動だが CUD ほど割引率は高くない
- Spend-Based CUD と Resource-Based CUD を混同 → 割引率も柔軟性も全く違う
- Preemptible VM と Spot VM を区別せず提案 → Preemptible は新規利用不可、Spot のみ
- プロジェクトラベルだけで按分 → リソースラベルとラベル継承ルールの両方が必要
- BigQuery オンデマンド → スロットベース切替を「安い」の一言で提案 → 使用量パターンで逆転することがある
- Regional PD を Zonal PD にダウングレード提案 → HA 要件を確認せずは事故
- 古いスナップショットを一括削除提案 → スケジュールポリシーで再生成される可能性あり
- Anthos / GKE Autopilot のコストを VM 換算で見積もる → 別の課金モデル
- Idle VM Recommender を鵜呑み → 負荷パターンがバッチ起動のワークロードは誤検知

## Output Discipline

- レポート: `docs/cost_analysis_YYYYMMDD.md`（日付付き、上書き禁止）
- 生データ: `output/<resource>_YYYYMMDD.json`
- 履歴: `docs/finops_history.md` 追記式、完了チェックボックス付き
- 優先度: 🔴即時 / 🟡担当者確認 / 🟢中期 の3段階
- 権限依頼は IAM ロール + 必要な追加 Permission を具体的に列挙（既定ロールは粒度が粗いので Custom Role 推奨時も明記）
