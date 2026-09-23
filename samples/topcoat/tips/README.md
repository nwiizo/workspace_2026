# Topcoat Tips

Topcoat 0.5.0の小さな検証用binaryをまとめた独立packageです。親directoryのアプリケーションへ依存せず、各sampleを個別に起動、testできます。

## Samples

- `multipage`: 3ページと共通layout、portを開かないRouter test
- `typed_input`: 型付きpath/query、400 response、HTML escape
- `json_io`: JSON request/responseと不正bodyの400 response
- `form_redirect`: form入力、validation、303 See Other
- `session_security`: session layerによるcross-site POST拒否
- `sqlite_restart`: 新規SQLiteだけのschema初期化と再起動test
- `reactive_boundaries`: signal、procedure、shardの使い分け
- `opentelemetry`: 最小fieldのHTTP request spanとin-memory exporter test

## Verification

```sh
cargo fmt --manifest-path samples/topcoat/tips/Cargo.toml --all -- --check
cargo clippy --manifest-path samples/topcoat/tips/Cargo.toml \
  --all-targets --all-features -- -D warnings
cargo test --manifest-path samples/topcoat/tips/Cargo.toml --all-targets
```

個別に起動する時はbinary名を指定します。

```sh
cargo run --manifest-path samples/topcoat/tips/Cargo.toml --bin multipage
```
