# Hidden Cost Detection Patterns

「課金が続いているのに気づかれにくい」リソースを機械的に検出するための判定基準。

## 停止リソースの隠れ課金

- 停止中 EC2: インスタンス料金は $0 だが **EBS ボリューム / EIP / AWS Backup の日次スナップショット** は課金継続
- 停止中 RDS: インスタンス料金 $0、ただし**ストレージ費は継続**、7日で自動再起動される
- 停止中 Redshift: Paused 中はコンピュート $0、ただし**スナップショットストレージは継続**
- 判定: インスタンスの `State=stopped` と、`BlockDeviceMappings` / `Addresses` / `Snapshots` の紐付きを交差で確認

## 孤児リソースの検出

### 孤児スナップショット（親 AMI 削除済み）

- Snapshot の `Description` に `ami-xxxxxxxx` の参照があり、その AMI が `describe-images --owners self` に存在しない
- AMI 削除時に関連スナップショットが残ったまま放置されるパターン
- 論理的に不要なのでほぼ全件削除可能、ただし削除前に再確認

### 未アタッチ EBS

- `State=available` かつ `Attachments=[]`
- インスタンス削除時にボリューム保持設定が残った残骸
- Compute Optimizer は 32 日以上未アタッチなら snapshot+delete を推奨

### 未紐付け EIP

- `InstanceId=null` かつ `NetworkInterfaceId=null` かつ `AssociationId=null` → 完全に遊んでいる
- `InstanceId=null` だが `NetworkInterfaceId` 有り → NLB/NAT/ENI 経由で使用中の可能性、解放前に確認必要

### Classic / 未稼働 ELB

- ALB/NLB: すべてのターゲットグループの `TargetHealthDescriptions` が空または全 unhealthy
- CLB: `Instances=[]`
- 基本料金は使用量にかかわらず月額課金される
- **CloudWatch メトリクスからの判定がより確実**:
  - ALB: `AWS/ApplicationELB` の `RequestCount` が直近 1 ヶ月で 0
  - NLB: `AWS/NetworkELB` の `ActiveFlowCount` Maximum / `ProcessedBytes` Sum が 0
  - CLB: `AWS/ELB` の `RequestCount` が 0、`HealthyHostCount` Maximum も 0
- LCU 課金（`LCUUsage`）が極めて低い場合も idle LB が多い裏付けになる

## 名前・説明欄からの検出

- `remove-before-YYYY-MM-DD` / `finalsnapshot` / `old-` / `test-` / `deprecated-` 命名 → 長期放置の候補
- Description が空の手動スナップショット → 用途追跡不可
- AWS Backup 由来（`AwsBackup_` prefix）は自動生成、保持ポリシーで対応

## 重複インフラの検出

- CloudTrail: Cost Explorer の `FreeEventsRecorded` と `PaidEventsRecorded` が一致 → マルチリージョン Trail が複数存在、2つ目以降は有料
- VPC Endpoint: 同リージョンで同サービス向けのエンドポイントが複数
- NAT Gateway: AZ ごとではなく冗長で複数ある場合は用途確認

## リージョン見落とし

- SQS / SNS / S3 の一部は**利用頻度の高くないリージョン**にリソースが残っていることがある
- `ap-northeast-1` だけ見ると見落とす。Cost Explorer の USAGE_TYPE で `USW2-*` 等の他リージョン prefix があれば要調査
- `aws <service> list-... --region us-west-2` 等を全主要リージョンで確認する

## 判定の注意事項

- 「論理的に不要」と見えるリソースでも、過去の DR 計画や監査要件で保持されている可能性あり
- 削除提案前に「いつから放置されているか」「担当者の認識があるか」を必ずセットで確認
- production タグ付きリソースは停止中であっても即時解放を提案しない

## paused リソースの隠れ課金

- **Redshift dc2.large は paused でもノード時間料金が止まらない**（ストレージとコンピュートが分離されていないため）
- ra3 ノードタイプは paused 中のコンピュート課金が 0 になる
- Redshift Serverless は使用時のみ課金、未使用時 $0
- 「paused だから安心」ではなく、`describe-clusters --query 'Clusters[].{id:ClusterIdentifier,status:ClusterStatus,type:NodeType}'` で全 paused かつ dc2 系のクラスタが残っていないか確認する
- 検出: Cost Explorer で `Node:dc2.*` の USAGE_TYPE が出ているのに、コンソールで全クラスタ paused

## SQS の空転（コンシューマ消失）

- 送信側のアプリは残っているが受信側（コンシューマ）が落ちて／削除されているパターン
- CloudWatch メトリクスで判定: `NumberOfMessagesSent > 0` かつ `NumberOfMessagesReceived ≈ 0`
- dev / 検証環境のキューで起こりやすい。本番停止後に dev だけ送信し続ける状況
- `ApproximateNumberOfMessagesVisible` が累積していたら、メッセージが滞留している（14 日で expire）
- 検出: 全リージョンで `aws sqs list-queues` → 各キューに `get-metric-statistics` で sent / received 比較
- リクエスト料金 $0.40/100万 で計算するとそれなりの金額になる（数千万送信 → 月 $10 単位）

## CloudWatch CW:Requests の発信源

- `APN1-CW:Requests` が月数百ドル規模なら、外部監視 SaaS（Datadog / Mackerel / NewRelic / Splunk Observability）が CloudWatch API を叩いている可能性
- AWS 純正の CloudWatch Agent や Container Insights を使っているかは `aws cloudwatch list-metrics --namespace CWAgent` / `--namespace ECS/ContainerInsights` で確認
- これらが 0 件なのに `CW:Requests` が出ているなら、外部 SaaS 由来とほぼ確定
- 削減手段: [CloudWatch Metric Streams](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-Metric-Streams.html) で Kinesis Firehose 経由で外部に流す方が cheaper（メトリクスあたり $0.003）

## 公開 IPv4 アドレスの整理（2024-02 開始の有料化）

- 完全に未紐付けの EIP（`InstanceId=null` かつ `AssociationId=null`）は最優先で解放
- ENI には紐付いているが Instance がない EIP（`InstanceId=null` だが `AssociationId` あり）は要確認
  - NAT Gateway / VPC Endpoint / Lambda VPC / NLB の使用ぶん（解放してはいけない）
  - 削除済 EC2 の ENI 残骸（解放可）
- EC2 直割当の Public IP（auto-assign）も $0.005/h 課金。常時稼働の EC2 が多い場合、EIP に集約 + ENI 単位での管理に切り替えると見通しが良い
- VPC USAGE_TYPE で `IdleAddress` の金額から idle ぶんの個数を逆算可能（$0.005/h × 730h = $3.65/月/個）
