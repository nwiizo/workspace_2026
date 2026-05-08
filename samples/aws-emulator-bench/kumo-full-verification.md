## 付録: 参照点としての kumo（Go 版）を実機で叩く

Apple M3 / Darwin 25.4.0 で実施しました。比較値は既存メモ準拠です。

### セットアップと数値

| 項目 | fakecloud 0.10.1 | rustack 0.7.0 | kumo latest |
| --- | ---: | ---: | ---: |
| Docker image | 206 MB | 39 MB | 32.9 MB |
| cold start | ~0.29 s | ~0.39 s | 0.603 s |
| idle memory | 2.67 MiB | 984 KiB | 2.504 MiB |
| 公称/実測サービス数 | 23 | 19 | README 76 / pkg.go.dev 73 / 起動ログ 77 |

`/health` は正常でした。起動ログの `service available` は 77 件で、README の 76 件より 1 件多く、`iam`、`cloudtrail`、`states`、`glue`、`rekognition`、`lambda`、`s3`、`sns`、`sqs`、`dynamodb` を含みました。

### 踏み込み 4 本

| 検証 | kumo 実測 |
| --- | --- |
| 1. SNS → SQS fanout | `sns create-topic` で `MissingServiceIdentifier: User-Agent header with api/ identifier is required for Query protocol routing`。fanout まで未到達でした。 |
| 2. `aws-sdk-rust` | S3 は `InvalidArgument: Invalid key`。ログでも `PUT /sdk-kumo/` が `/{bucket}/{key...}` に誤ルーティングされました。SQS は成功しました。 |
| 3. Lambda 実実行 | `iam create-role` は上記と同じ失敗でした。ダミー ARN でも `lambda create-function=InvalidRequest` で、`POST /2015-03-31/functions` が S3 に誤ルーティングされました。`invoke` 未到達で、実実行/stub は判定不能でした。 |
| 4. Terraform apply + destroy | `apply` は 170.895 秒で失敗し、SQS だけ 25 秒で作成完了、S3 は `policy (...ListBucketResult...) is invalid JSON`、DynamoDB は `couldn't find resource (21 retries)` でした。`destroy` も 5.022 秒で `ListTagsOfResource is not valid` まで含めて失敗しました。 |

既存メモでは fakecloud は Terraform apply 自体は通り、rustack は別原因で apply 失敗です。kumo は深掘り時の互換性で一段厳しい結果でした。

### kumo 独自サービスの実動作

| 試行 | 判定 | 観測 |
| --- | --- | --- |
| IAM `create-user` / `list-users` | not implemented | どちらも `MissingServiceIdentifier` で失敗しました。 |
| CloudTrail `describe-trails` | not implemented | `UnknownService com`。kumo ログも HTTP 400 でした。 |
| Step Functions `list-state-machines` | 空リスト | `{"stateMachines":[]}` が返りました。 |
| Glue `get-databases` | 成功 | exit 0、stdout 0 byte、ログは HTTP 200 でした。 |
| Rekognition `list-collections` | 成功 | exit 0、stdout 0 byte、ログは HTTP 200 でした。 |
| Comprehend `list-entity-recognizers` | not implemented | `UnknownOperationException` で失敗しました。 |

### 結論

kumo の強みは、32.9 MB という最小イメージと 77 件の表面積です。idle memory も fakecloud よりは小さめでした。ただし cold start は 0.603 秒で、fakecloud 0.29 秒、rustack 0.39 秒より遅く、数値面で全面優位ではありません。

互換性では制約がはっきり出ました。SQS は AWS SDK Rust で通る一方、SNS/IAM の Query protocol は AWS CLI で routing 失敗、Lambda `create-function` も S3 に誤ルーティング、Terraform も S3/DynamoDB の provider 期待値を満たしませんでした。軽くて広い参照点にはなりますが、無修正の AWS CLI / Terraform を受ける drop-in endpoint としては fakecloud / rustack より未整合が残ります。

### 再現

```text
aws-sdk-rust/src/main.rs
- endpoint を `http://localhost:4566` に変更
- S3/SQS を個別表示に変更

docker rm -f km 2>/dev/null || true
docker run -d --name km -p 4566:4566 ghcr.io/sivchari/kumo:latest
until curl -sf http://localhost:4566/health >/dev/null; do sleep 0.2; done
./sns-sqs-fanout/run.sh http://localhost:4566
(cd aws-sdk-rust && cargo run --release)
(cd terraform && terraform init && terraform apply -auto-approve -var endpoint=http://localhost:4566 && terraform destroy -auto-approve -var endpoint=http://localhost:4566)
```
