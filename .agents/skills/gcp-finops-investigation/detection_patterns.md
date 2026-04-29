# Hidden Cost Detection Patterns (GCP)

Recommender 任せでは見つからない、あるいは機械判定の盲点になるパターン。

## Recommender の判定基準と盲点

### Idle VM Recommender

- 判定: CPU 使用率 < 0.03 を 14 日間の 97% 以上、ネットワーク受信 < 2600 bytes/sec の 95% 以上、送信 < 1000 bytes/sec の 95% 以上
- **バッチ系ワークロードで誤検知**: 日次で数時間だけ動く VM は、残り時間の idle 判定で推奨対象化される
- 判定前に「スケジュール起動か、常時稼働か」を必ずラベルで区別する
- VM が停止済み（TERMINATED）でも Persistent Disk は課金継続 → Idle VM 推奨と同時に Idle Disk 推奨が出ているか確認

### Idle Persistent Disk

- 未アタッチディスクを Recommender が検出
- 削除前に**スナップショットを先に取る**推奨。Compute Optimizer 相当の「snapshot-and-delete」は GCP では手動
- PD スナップショットは差分課金なので、定期的に取られているなら最新のみ残せば容量削減可

### Idle Address

- 外部 IP アドレスで VM に紐付いていないもの → 1個あたり数ドル/月
- リージョナル/グローバル（Cloud Load Balancing 用）で価格が違う
- DNS レコードに残っている可能性があるので解放前に確認

### Idle Reservation（2026 新機能）

- オンデマンドリザベーションで 7 日以上未使用 → Recommender が modify/delete 推奨
- 購入直後は使用率が低いのが普通なので、本番反映前は誤検知あり

## Recommender では見つからないもの

### 古いスナップショット・マシンイメージ

- Recommender は「idle」判定が対象、単に古いだけのリソースは対象外
- スケジュールポリシーで日次自動生成されるスナップショットは世代数設定がなければ無限増殖
- カスタムマシンイメージはスナップショットより管理が漏れやすい
- Cloud Storage に `.img` が保存されていることもある

### オーバープロビジョニング

- VM のリソースが大きすぎる（CPU/メモリ過剰）の判定は Rightsizing Recommender で別途
- 標準ロール `roles/recommender.computeViewer` とは別に Rightsizing 用のロールが必要なことも
- SQL / BigQuery は Rightsizing 推奨が別 API

### ストレージクラス最適化

- Cloud Storage のオブジェクトが Standard のまま放置されている
- Autoclass を有効にすれば自動的にクラス移行されるが、既存バケットには後付け可
- ライフサイクルルール未設定のバケットは要確認

### ネットワーク関連

- **リージョン間のデータ転送費**: Multi-region / Dual-region の GCS、VPC Peering 経由のクロスリージョン通信
- **Cloud NAT**: NAT Gateway と同様、長時間の未使用や過剰プロビジョニング
- **Cloud Interconnect / VPN**: 使われていない VLAN アタッチメントが残存

### BigQuery 特有

- オンデマンド料金モード vs Capacity (Reservation) モード: スキャン量が一定以上なら Capacity が安い、逆なら オンデマンド
- クエリのスキャン量が過剰（`SELECT *`、パーティションフィルタなし）は「コスト」として見えづらい
- マテリアライズドビュー / Capacity Commitments の選択は使用パターン次第
- テーブルパーティション未設定 / クラスタリング未設定の大規模テーブル

### Cloud SQL

- HA 構成（Regional）を Zonal にすれば半額、ただし HA 要件確認必須
- バックアップ保持期間のデフォルトが長いことがある
- 古い世代（db-n1-*）から新世代へ更新で性能向上 + コスト減の可能性

## ラベルに基づく判定の限界

- production / staging / dev の区別がラベルになければ機械判定できない
- プロジェクト分離で運用されている組織では、プロジェクト名から推測するしかない
- CUD カバレッジの按分も同様で、明確なラベル戦略がないと正確なチャージバックは不可能

## リージョン見落とし

- `asia-northeast1` だけ見ると別リージョンの残骸を見落とす
- Cloud Run / Cloud Functions はリージョンごとのサービスなので、使っていないリージョンに過去のリビジョンが残っていることも
- BigQuery のマルチリージョンは EU / US / asia 単位、ロケーション違いでクロスリージョン転送発生

## 判定時の注意事項

- Recommender の「idle」と「unused」は同義ではない。idle は使用ログベース、unused は紐付けベース
- production ラベルのリソースは停止状態でも担当者確認必須（外部疎通リスク）
- 削除提案前に「いつから idle か」を必ずセット（Recommender の `firstObservationTime` 参照）

## 停止 / paused リソースの隠れ課金

「停止 = 課金 0」と思い込むと取りこぼす。以下は停止・休止していてもコストが続く:

- **Compute Engine の TERMINATED VM**: VM 本体の課金は止まるが、Persistent Disk と Reserved External IP は課金継続
- **Cloud SQL の停止**: インスタンス料金は停止できるが、ストレージとバックアップ保持は継続。HA 構成のセカンダリストレージも残る
- **Bigtable**: 最小ノード数の設定により、未使用でもノード料金が発生
- **BigQuery Reservation**: スロットコミットメント期間中は使用量に関係なく料金発生
- **Cloud Run の minimum instances**: min instances > 0 だとアイドル時もコンテナ料金発生

検出: Billing Export の SKU と、各サービスのリソース状態（status / state）を突合する。

## Public IPv4 の整理（GCP も時間課金）

- Reserved 状態で VM に紐付いていない外部 IP（Static IP, unattached）は時間課金（リージョン約 $0.010/h、グローバル約 $0.020/h）
- VM に紐付いている External IP も、エフェメラルでも 2024 以降は課金（Premium / Standard tier で異なる）
- 検出: `gcloud compute addresses list --filter='status=RESERVED AND -users:*'` で未使用の予約 IP を抽出
- DNS や firewall で参照されている可能性があるので解放前に確認

## Cloud Monitoring API の課金源

- Cloud Monitoring のメトリクス取得 API（`monitoring.googleapis.com/api/request_count`）が大量に発生していると、API 呼び出し課金が膨らむ
- 外部監視 SaaS（Datadog / New Relic / Splunk Observability）が GCP メトリクスを引っ張ると顕著
- Monitoring の Self-monitoring メトリクス（`monitoring.googleapis.com/billing/api_request_count`）で送信元（principal）を特定可能
- 削減手段: Cloud Monitoring の Metric Export を Pub/Sub 経由でまとめて流す方が cheaper

## アイドル Load Balancer

- Forwarding rule（HTTP(S) LB / TCP LB / UDP LB）は使用量に関係なく時間課金
- リクエスト数 0 のロードバランサーが残っているケース
- 検出: `gcloud compute forwarding-rules list` ＋ Monitoring の `loadbalancing.googleapis.com/https/request_count` 等で活動量を確認
- AWS の ELB と同様、削除候補として常時チェックすべき

## Pub/Sub の空転

- Topic に publish は続いているが Subscription が削除済み、または ack 0% のケース
- AWS SQS の dev キュー空転と同パターン
- 検出: `pubsub.googleapis.com/topic/send_message_operation_count` と `subscription/ack_message_count` の比較
- Subscription なし の Topic への publish は通常無害だが、retain メッセージ設定があるとストレージ料金が増える
