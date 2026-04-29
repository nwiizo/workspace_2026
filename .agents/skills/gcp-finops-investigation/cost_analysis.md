# GCP Cost Analysis Learnings

Billing Export (BigQuery)、FinOps Hub、Cost Table / Reports の読み解き罠。

## Billing Export の2種類

- **Standard export**: サービス・SKU・ラベル単位の集計。リソース単位の紐付けは粗い
- **Detailed export**: リソース単位の行もあり、どの VM / どの Disk がコストを出しているか追える
- 分析のためには Detailed 必須。Standard だけだと「どの VM が高い」と特定できない
- 設定後のデータ反映は最大1日ラグあり、過去データは含まれない

## BigQuery クエリの基礎構造

- 料金テーブル: `project.billing_export_dataset.gcp_billing_export_resource_v1_<BILLING_ACCOUNT_ID>`（Detailed の場合）
- 期間指定は `usage_start_time` か `export_time` を明示。`_PARTITIONTIME` フィルタも重要（スキャン量削減）
- コスト算出は `SUM(cost) + SUM((SELECT SUM(amount) FROM UNNEST(credits)))` で実質コスト
- credits は SUD / CUD / Promotional / Commitment Use などを含み、**マイナス値**として計上される
- クレジット除外を忘れると見かけ上のコストが実質より2〜3割高く出る

## 2026 年の仕様変更の影響

- Spend-Based CUD の計上方式が 2026 年から変わった: SKU 価格に**直接適用**、credit offset ではなくなった
- 過去データと新データで計算式が変わるので、長期トレンドで**歴史的データは credit、最新は SKU 価格**が混在
- レポートで「CUD 効果」を語るなら期間で区切って説明する
- 対象サービス拡大: Cloud Run / H3 / M シリーズ / メモリ最適化 VM が Spend-Based CUD 対象化

## Reservation ラベル（2025-09-17 以降）

- Billing Export の行に reservation name がラベルとして付与される
- **使用されているリザベーション** と **空きリザベーション（idle）** を区別できる
- `system_labels` 配下に `compute.googleapis.com/reservation_name` などのキーが入る
- 購入済みだが使われていないリザベーションの検出はこれで可能

## FinOps Hub UI

- Console UI で BigQuery クエリを書かなくても主要な分析が可能
- Recommender 推奨事項、CUD カバレッジ、異常検知も一覧化
- ただし細かいフィルタやグループ化は BigQuery の方が柔軟
- 新機能（Idle Reservation など）は FinOps Hub 経由で先行提供される傾向

## ラベル/タグの取り扱い

- リソースに付与されたラベル（小文字、`:` 区切り）は Billing Export で `labels` 配列として出る
- 継承ルール: プロジェクトラベル → リソースラベル（継承されないことに注意、VM 作成時に明示指定必要）
- ラベル未設定リソースは `labels IS NULL` でしか絞れない。ラベル戦略の導入前データは完全には按分不可
- 組織ポリシーで必須ラベルを強制できる（Organization Policy Service）が、既存リソースには遡及しない

## 数値の分母混在を避ける

- 月平均（過去N ヶ月）: 長期トレンド、外部共有の基準
- 月末確定値: 担当者報告の基準
- 進行中月の推定（N日分 × 30/N）: 推定値であることを明記
- Billing Console の「Cost Breakdown」はデフォルトで進行中月を表示するが、月末予測値か実績かを読み取る

## cost と effective_cost の使い分け

Detailed Billing Export には複数のコスト列がある:

- `cost` — リスト価格ベースの請求発生額（割引前）
- `credits` — SUD / CUD / プロモーションなど、マイナス値で計上
- 実コスト = `cost + SUM(credits.amount)`
- 2026 年からの Spend-Based CUD は credit ではなく SKU 価格に直接適用されるため、`cost` だけでも実コストが出るケースが増えている（過去データと混在に注意）

CUD を「Premium 一括前払い」で買った場合、その前払い額は購入月に一時的に乗る。AWS の Unblended と同じ罠で、購入翌月以降に「コストが下がって見える」。月次推移で議論する際は:

- 短期キャッシュ視点 → `cost + credits` をそのまま見る
- 経済的負担を月平均で見る → 前払いを契約期間で按分して加算した値を併記
- 長期トレンドの比較 → 購入月と非購入月で見え方が違う点を必ず注記

## VM コストが「Compute Engine」で潰れる罠

サービス名 `Compute Engine` の合計だけでは、削減候補の解像度が出ない。Detailed Billing Export の `sku.description` で内訳を見る。代表的な分割軸:

- VM の Hourly（`<machine type> Instance Core/RAM running in <region>`）
- Persistent Disk（`Storage PD Capacity` / `Storage PD SSD Capacity`）
- Snapshot（`Storage PD Snapshot in <multi-region>`）
- License（`Licensing Fee for ...`）
- Public IPv4（`External IP Charge on a <state> VM`）

AWS の `EC2 - Other` と同様、Disk / Snapshot / Public IP が VM 本体と混ざるので分解必須。

## 権限不足時の迂回

- 自プロジェクトだけ Billing Export を確認できて、組織全体が見えない → 組織レベル Billing Account Viewer を依頼
- BigQuery にデータセットはあるが `SELECT` できない → `roles/bigquery.dataViewer` + `roles/bigquery.jobUser` の2つが必要
- Recommender API が叩けない → `roles/recommender.viewer` が別途必要（`roles/viewer` には含まれない）
