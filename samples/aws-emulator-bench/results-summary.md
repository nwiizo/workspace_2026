# Measurements summary (2026-04-23, Apple M3, Darwin 25.4.0, Docker 28.0.4)

All via `docker run -d ghcr.io/<repo>:latest`.

| | fakecloud 0.10.1 | rustack 0.7.0 | kumo (tbd) |
|---|---|---|---|
| Docker image | 206 MB | 39 MB | 32.9 MB |
| Cold start (run→ready) | ~0.29s | ~0.39s | ~0.90s |
| Idle memory | 2.67 MiB | 984 KiB (post-traffic 2.9 MiB) | 2.25 MiB |
| Services advertised | 23 (ses, cfn, orgs, kms, states, rds, sqs, cognito-idp, apigateway, sts, ssm, dynamodb, elasticache, iam, scheduler, bedrock, logs, s3, lambda, events, secretsmanager, kinesis, sns) | 19 (dynamodb, dynamodbstreams, sqs, ssm, iam, cloudwatch, ses, sts, sns, events, logs, kms, kinesis, secretsmanager, apigatewayv2, apigatewayv2-execution, lambda, cloudfront, s3) | 76 (advertised on GitHub) |
| Health endpoint | /_fakecloud/health | /_localstack/health, /health | /health |
| Default account id | 123456789012 | 000000000000 | — |
| License | AGPL-3.0 | MIT | MIT |
| Stars (2026-04-23) | 232 | 21 | (upstream) |
| Age | 2026-04-04 (~3 weeks) | 2026-02-27 (~2 months) | — |

## Happy path (both PASS)
- S3 mb/cp/ls
- SQS create-queue / send-message / receive-message
- DynamoDB create-table / put-item / get-item
- Secrets Manager create/get
- SQS FIFO create-queue
- SNS create-topic
- KMS create-key / generate-data-key
- EventBridge put-events
- DynamoDB conditional put → ConditionalCheckFailedException (correct)
- S3 presign URL (both)

## Quirks observed
- fakecloud: `aws s3 cp` reports "Expected checksum did not match" on GET because `ChecksumCRC64NVME` header is returned empty; the payload itself is correct.
- rustack: returns ChecksumCRC64NVME correctly (ChecksumType=FULL_OBJECT).
- fakecloud has Bedrock with preloaded foundation-model list — unique.
- rustack advertises cloudfront which fakecloud lacks.

