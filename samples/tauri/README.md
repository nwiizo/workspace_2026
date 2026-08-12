# 日々 (Hibi)

Rust と Tauri で作った、ローカル完結の Markdown 日記アプリです。1 日分を 1 つの `.md` ファイルとして保存し、日付一覧から読み返せます。

## 起動

Rust、Node.js、Tauri が要求する OS ごとの開発環境を用意して、次を実行します。

```sh
npm install
npm run tauri dev
```

品質チェックとリリースビルドは次のコマンドで実行できます。

```sh
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all --all-targets
npm run tauri build
```

## 保存形式

日記は Tauri のアプリデータディレクトリにある `entries/YYYY-MM-DD.md` へ UTF-8 で保存します。macOS では通常、次の場所です。

```text
~/Library/Application Support/com.nwiizo.hibi/entries/
```

日付はファイル名、タイトルは Markdown の最初の `# 見出し` から画面表示時に導出します。同じ日付を保存すると、そのファイルを安全に置き換えます。
