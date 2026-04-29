# Cost Explorer Reading Learnings

公式ドキュメントに書かれていないが、実戦で必要な Cost Explorer の読み解き技術。

## USAGE_TYPE 別で見ないと分からないこと

- サービス合計だけでは削減候補が見えない。`--group-by Type=DIMENSION,Key=USAGE_TYPE` で内訳を取る
- 特に以下は内訳でしか見えない:
  - `EBS:SnapshotUsage` vs `EBS:VolumeUsage.gp3` vs `EBS:VolumeP-Throughput.gp3`（EC2-Other 配下）
  - `Multi-AZUsage:db.xxx` vs `InstanceUsage:db.xxx` vs `RDS:ChargedBackupUsage` vs `RDS:GP2-Storage` vs `RDS:Multi-AZ-GP2-Storage`（RDS配下）
  - `PublicIPv4:IdleAddress` vs `PublicIPv4:InUseAddress`（VPC配下）
  - 各リージョンの `XXX-Requests-*` / `XXX-DataTransfer-*`（SQS配下）
  - `CW:Requests` vs `VendedLog-Bytes-*` vs `DashboardsUsageHour` vs `MetricMonitorUsage`（CloudWatch 配下）
  - `Node:dc2.large` vs `Redshift:PaidSnapshots`（Redshift 配下）

## VPC コストの内訳に注意（NAT じゃない可能性）

VPC カテゴリの金額を見て「NAT Gateway だろう」と決めつけない。USAGE_TYPE で確認すると、NAT Gateway が 0 個でも以下が乗っていることがある:

- `APN1-PublicIPv4:InUseAddress` — EC2/ELB に割当済の Public IP（$0.005/h）
- `APN1-PublicIPv4:IdleAddress` — 未関連付けの EIP（$0.005/h）

[2024-02 から AWS は全公開 IPv4 アドレスに課金している](https://aws.amazon.com/jp/blogs/news/new-aws-public-ipv4-address-charge-public-ip-insights/)。EC2 直割当の Public IP（auto-assign）も対象。`describe-nat-gateways` で 0 個なら、VPC カテゴリの金額はほぼ全額 IPv4 課金と判断してよい。

## Unblended と Amortized は必ず両方確認する

- Cost Explorer のデフォルト metrics は `UnblendedCost`（請求発生ベース、前払いは契約月に全額乗る）
- 経済的負担を月平均で議論したいときは `--metrics AmortizedCost` を使う
- RI / Reserved Node / Savings Plans を購入した直後は、両者の差が大きくなる:
  - 購入月: Unblended 急増、Amortized は通常の月割値
  - 翌月以降: Unblended 急減、Amortized は前払い按分が乗って高めに見える
- レポートで「2月より4月の方が安い」と言うとき、Unblended ベースか Amortized ベースかで結論が変わることがある
- 比較を出すときは、月次表に Unblended と Amortized の両列を入れる

## 月額が異常値を示した場合の判断フロー

1. `HeavyUsage:xxx` の USAGE_TYPE が突然出ていないか → あれば RI / Reserved Node の **All Upfront 一括購入**。その月に前払い全額計上される
2. `NoUpfront` / `PartialUpfront` でも月額 RI 料金が出る
3. RI は購入月の翌月から使用料が激減するので、翌月の急減を見て確認できる
4. 税金（Tax）は本体費用に連動して増加。税込で「2倍になった」と見えることもある

## 月次データの `Estimated: true` フラグ

- 当月の進行中データには `Estimated: true` が付く。**AWS 側の月末予測値**か**日割り合算**かを区別する
- 月半ばの数値を「21日 ÷ 30日」で年換算するのと、AWS の月末予測値を読むのは意味が違う
- 外部共有時はどちらを使ったか明記する

## 数値の分母混在が最も事故る

- 月平均（過去 N ヶ月の平均）: 長期トレンド用
- 前月確定値との単月差分: 短期効果測定用
- N日実績 vs 30日確定値: **分母不一致**、比較してはいけない
- 「月末換算」（当月N日実績 × 30/N）: 推定値。完全な推測であることを明記
- レポートに「-$X/月削減」と書くときは、どの基準かを必ず併記する。1つの数字で語らない

## 前回調査との差分比較

- 月額で比較 → 季節変動（閏月、祝日、ワークロード変動）が混ざる
- 月平均で比較 → トレンド捕捉可だが、直前の改善効果がぼやける
- 両方併記し、どちらを重視するかを文脈で示す

## 権限なしでも推測できる情報

- CloudTrail: `FreeEventsRecorded` と `PaidEventsRecorded` が全リージョンで一致 → 同じイベントが複数 Trail で二重記録（重複 Trail の証拠）
- S3: 料金が高いのにバケット一覧権限がない → オブジェクト数・ストレージクラス内訳推定は困難、権限追加依頼を優先
- VPC: `PublicIPv4:IdleAddress` USAGE_TYPE の金額から未使用 EIP 数を逆算可能（単価 × 時間）
