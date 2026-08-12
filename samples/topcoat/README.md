# Tatami Log

稽古後に1分で残し、次の稽古前に3分で思い出す、BJJ向けの日本語トレーニングノートです。
[Topcoatの発表記事](https://tokio.rs/blog/2026-07-22-announcing-topcoat)を起点に、[Topcoat 0.5.0](https://github.com/tokio-rs/topcoat) のフルスタック機能を検証するサンプルとして実装しています。

## 判定

Topcoatは、RustだけでSSRと細かなインタラクションを持つMVPを作る用途では実用可能でした。一方、バージョン間のAPI変化、schema migration、認証運用まで含めた本番採用には追加検証が必要です。

| 領域 | 結果 |
| --- | --- |
| routing / SSR / layout | 日本語画面と認証ガードを実装できた |
| signal / procedure / shard | 答えの開閉、復習保存、技検索を実ブラウザーで確認できた |
| local auth / session | Argon2id、Cookieセッション、ログアウト後の失効、再ログインを確認できた |
| Toasty / SQLite | 5モデル、unique/index、transaction、ユーザー別データ分離を確認できた |
| testability | assetなしのRouter受け入れテストと、CLI起動後のブラウザーE2Eを併用できた |

## 検証している機能

- server-side renderingによる日本語UI
- `signal` による答えの表示・非表示
- `#[shard]` によるユーザー別の技検索と部分再描画
- `#[procedure]` による復習結果の保存
- Topcoat sessionと、Secure / HttpOnly / SameSite=Lax Cookieによるローカル認証
- Toasty 0.7 + SQLiteによるユーザー・セッション・稽古・技カード・復習履歴の永続化
- デフォルトOrigin Policyによるcross-siteの状態変更拒否

## 起動

Rust 1.95以上とTopcoat CLI 0.5.0が必要です。

```sh
cargo install topcoat-cli --version 0.5.0
cargo topcoat dev
```

ブラウザーで <http://127.0.0.1:3000> を開きます。DBはデフォルトで `tatami-log.db` に保存されます。
別のSQLiteファイルを使う場合は次のように指定します。

```sh
DATABASE_URL=sqlite:verification.db cargo topcoat dev
```

## 検証

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

HTTP受け入れテストは、登録から稽古記録、別ユーザーからの不可視性、ログアウト後の旧セッション無効化までをインプロセスのTopcoat Routerで確認します。

## サンプルとしての境界

- ローカル認証にはArgon2idを使いますが、メール確認、パスワード再設定、ログイン試行のrate limitは未実装です。
- 復習日は日本向けMVPとしてJST固定です。ユーザーごとのタイムゾーン設定はありません。
- `push_schema` で空DBへスキーマを反映します。本番運用では明示的なmigrationへ置き換える必要があります。
- ToastyはTopcoat公式サンプルと揃えた0.7系です。最新Toastyへの移行はこの検証の対象外です。
- 技検索は個人利用規模のMVPとして、ユーザーのカードを読み込んで最大8件へ絞ります。大量データ向けの全文検索や入力debounceは対象外です。
- Topcoat自体がearly-stageのため、version更新時はmacro、runtime asset、Cookie/session APIの再検証が必要です。
