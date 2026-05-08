# aws-sdk-rust smoke test

Minimal program that hits each emulator with the official `aws-sdk-s3` and
`aws-sdk-sqs` crates. Verifies the Rust SDK works with both servers (not just
the AWS CLI).

```sh
cargo run --release
```

The binary expects fakecloud on `http://localhost:4567` and rustack on
`http://localhost:4568`. Adjust in `src/main.rs` if ports differ.
