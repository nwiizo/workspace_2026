# samples/

小規模なサンプルコード・学習用

## 用途

- 言語機能の学習・検証
- ライブラリの動作確認
- 小さなプロトタイプ
- 使い捨てのテストコード

## tools/ との違い

| samples/ | tools/ |
|----------|--------|
| 小規模・使い捨て | しっかりしたプロジェクト |
| 学習目的 | 公開・再利用目的 |
| 単一ファイルもOK | 構造化されたプロジェクト |

## プロジェクト例

```
samples/
├── rust_async_test/    # async/await検証
├── sqlx_example/       # sqlxの使い方確認
└── nvim_plugin_test/   # Neovimプラグイン検証
```

## 新規作成

```bash
cd samples/

# 単純なRustサンプル
cargo new sample-name

# スクリプト
touch test_script.sh
```
