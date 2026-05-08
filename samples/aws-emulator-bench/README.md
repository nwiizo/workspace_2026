# aws-emulator-bench

Verification code and raw results for the blog post comparing Rust-native AWS
emulators (fakecloud, rustack) with the Go original (kumo).

All measurements taken on Apple M3 / Darwin 25.4 / Docker 28.0.4 /
AWS CLI 2.34.35 / 2026-04-23.

## Layout

| Path | Purpose |
|------|---------|
| `terraform/` | `aws` provider 5.x config pointing at an endpoint variable — `apply` + `destroy` against each emulator |
| `aws-sdk-rust/` | Cargo crate using `aws-sdk-s3` + `aws-sdk-sqs` against both servers |
| `lambda/` | Python echo handler for `lambda create-function` + `invoke` tests |
| `sns-sqs-fanout/` | Shell script: SNS topic → SQS queue subscription with raw delivery |
| `results-summary.md` | Consolidated numbers (image size, cold start, memory) |
| `kumo-codex-verification.md` | OpenAI Codex CLI's independent kumo reference section |

## Running it

Start the emulators on distinct host ports so tests can run side-by-side.

```sh
docker run -d --name fc -p 4567:4566 ghcr.io/faiscadev/fakecloud:latest
docker run -d --name rs -p 4568:4566 \
  -e LAMBDA_DOCKER_ENABLED=true \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/tyrchen/rustack:latest
```

Then run each test:

```sh
./sns-sqs-fanout/run.sh http://localhost:4567
./sns-sqs-fanout/run.sh http://localhost:4568

( cd aws-sdk-rust && cargo run --release )

( cd terraform && terraform init && \
  terraform apply -auto-approve -var endpoint=http://localhost:4567 && \
  terraform destroy -auto-approve -var endpoint=http://localhost:4567 )
```

See each subdirectory's `README.md` for observed quirks.
