---
name: aws-finops-investigation
description: AWS アカウントの FinOps 調査・コスト削減分析。ReadOnly + MFA 環境での非対話認証突破、Cost Explorer の読み解き罠、RI/SP の誤認パターン、レポート数値の分母混在を避けるための学び集。
---

# AWS FinOps Investigation Skill

FinOps 調査で実践で躓いた学びのみを記載。一般的な AWS / FinOps 知識は含まない。

## Trigger

- AWS コスト削減・棚卸し調査
- 前回調査との差分確認、定期レビュー
- RI / Savings Plans 購入判断、未使用リソース特定

## 参照するサブファイル

- [best_practices_2026.md](best_practices_2026.md) — FinOps Foundation 2026 Framework、業界ベンチマーク、クラウド非依存の原則
- [auth.md](auth.md) — MFA + AssumeRole の非対話環境での突破手順と罠
- [cost_analysis.md](cost_analysis.md) — Cost Explorer の読み方、異常値の解釈、数値の分母混在回避
- [detection_patterns.md](detection_patterns.md) — 孤児リソース・隠れ課金の検出パターン
- [ri_sp_gotchas.md](ri_sp_gotchas.md) — RI/SP の誤認パターンと判断基準
- [reporting.md](reporting.md) — レポート出力規律、history ファイル運用、優先度付け

## Core Principles

- **ReadOnly 厳守**: 提案のみ、変更系操作は自動実行しない
- **数値の分母統一**: 月平均 / 月末換算 / N日実績を混ぜない。外部共有は月平均ベースを基準にする
- **production タグのリソースは慎重扱い**: 停止中でも DNS/SG/外部許可リストで参照されている前提で担当者確認を必須にする
- **権限は時間経過で変化する**: 前回使えた API が今回 NG のことがある。調査冒頭で必ず再検証する
- **独立レビューを最低1回**: codex などの別エージェントで重複計上・警告漏れ・計算矛盾をチェックする
- **学びは即座に CLAUDE.md / memory に反映**: 同じ罠を次回踏まないため

## Phases（実施順）

1. 認証確立（MFA + AssumeRole、対話シェルで JSON 生成→Claude が読む）
2. 権限検証（使える/使えない API を冒頭で明示）
3. Cost Explorer で大枠把握（月次推移 → USAGE_TYPE 内訳 → 異常値解釈）
4. リソース棚卸し（並列で describe 系 API を叩いて JSON 保存）
5. 削減候補抽出（検出パターンを機械的に適用）
6. レポート作成（テンプレート、3段階優先度、重複除外）
7. 独立レビュー（codex agent で最低1回、修正必須の指摘に全対応）
8. 学びの memory / CLAUDE.md 反映

## Anti-Patterns（やらかしがち）

- いきなり describe 系 API を全件取得 → 先に Cost Explorer で金額の大枠を押さえる
- 単一の削減額を1数字で提示 → 分母ごとに3通り併記する
- RI 購入直後の月を「異常」と警告 → `HeavyUsage:*` USAGE_TYPE を確認
- Savings Plans が RDS に効くと誤案内 → 専用 RI が必要
- 停止中 EC2 を「課金ゼロ」と報告 → EBS / EIP / AWS Backup は課金継続
- 孤児スナップショット削除で親 AMI を見ずに deregister → 関連エラーになる
- production タグを見ずに「解放可能」と提案 → 外部疎通断の事故
- 削減額を施策間で重複計上（Snapshot 系が特に重なる）
- レポートを前回の上書きで消す → 履歴ファイルを別途運用して差分追跡可能にする

## Output Discipline

- レポート: `docs/cost_analysis_YYYYMMDD.md`（日付付き、上書き禁止）
- 生データ: `output/<resource>_YYYYMMDD.json`（再分析時のベースライン）
- 履歴: `docs/finops_history.md` 追記式、完了チェックボックス付き
- 優先度: 🔴即時 / 🟡担当者確認 / 🟢中期 の3段階で統一
- 追加権限依頼: IAM ポリシー JSON と Slack 依頼文のセットで出力
