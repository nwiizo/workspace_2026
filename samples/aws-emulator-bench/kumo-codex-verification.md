## kumo (Go 製オリジナル) を検証する

`ghcr.io/sivchari/kumo:latest` は、2026年4月23日時点の [GitHub main README](https://github.com/sivchari/kumo) では「76 services」を掲げています。一方で、同日時点の [pkg.go.dev の v0.8.0 ドキュメント](https://pkg.go.dev/github.com/sivchari/kumo) は「73 services」としており、公開されている説明だけでも数字が揺れています。さらに [server 実装](https://raw.githubusercontent.com/sivchari/kumo/main/internal/server/router.go) を読むと `/health` は `{"status":"healthy"}` を返し、[起動処理](https://raw.githubusercontent.com/sivchari/kumo/main/internal/server/server.go) では `service available` ログで登録サービス名を列挙する作りですが、サービス一覧を返す専用エンドポイントは見当たりませんでした。

| 項目 | fakecloud | rustack | kumo |
| --- | ---: | ---: | ---: |
| image size | 206 MB | 39 MB | 32.9 MB |
| cold start | ~0.29 s | ~0.39 s | 未計測 |
| idle memory | 2.67 MiB | ~1 MiB | 未計測 |

本稿の検証環境では Docker デーモンの UNIX socket と `127.0.0.1` への接続がサンドボックスで拒否され、`docker run`、`docker stats`、AWS CLI の `--endpoint-url http://localhost:4566` を使う疎通確認を完遂できませんでした。したがって、S3/SQS/DynamoDB/Secrets Manager/SNS/KMS/EventBridge/presign の happy path と、IAM/CloudTrail/Rekognition/Glue/Step Functions/Comprehend の差別化サービスは、この記事用の実測としては成功・失敗を確定していません。

実測で言える範囲に限ると、kumo の強みは 32.9 MB というイメージサイズの小ささです。逆に弱みは、公開ドキュメント上で対応サービス数が 76 と 73 で一致せず、広い対応範囲を主張している一方で、その runtime を今回の条件では裏取りできなかった点です。少なくとも [`internal/service`](https://github.com/sivchari/kumo/tree/main/internal/service) には IAM、CloudTrail、Glue、Rekognition、Step Functions、Comprehend を含む多数のサービス実装が並んでおり、差別化の方向性自体は確認できますが、動作保証まではこの原稿では書けません。
