# Terraform smoke test

```sh
terraform init
terraform apply -auto-approve -var endpoint=http://localhost:4567  # fakecloud
terraform destroy -auto-approve -var endpoint=http://localhost:4567

terraform init -upgrade
terraform apply -auto-approve -var endpoint=http://localhost:4568  # rustack
```

## Observed results (2026-04-23)

- **fakecloud 0.10.1**: apply succeeds for all 4 resources. `aws_sqs_queue` destroy hangs for ~2 min 10 s because fakecloud emulates the AWS 60-second queue-deletion window.
- **rustack 0.7.0**: apply fails on `aws_s3_bucket.demo` because `GetObjectLockConfiguration` returns 404 for buckets that never had a lock configured (aws provider 5.x treats 404 as fatal). Also fails on `aws_dynamodb_table.users` because `DescribeContinuousBackups` is not implemented.
