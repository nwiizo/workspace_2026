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

## 本番投入の判定基準

現状は検証用MVPであり、そのままインターネットへ公開できる状態ではありません。P0をすべて満たし、証跡を残すことを公開条件とします。`一部`は安全な要素を実装済みでも、本番運用に必要な経路が不足している状態です。

### P0: 公開前の必須条件

| 領域 | 現状 | 公開の受け入れ条件 |
| --- | --- | --- |
| HTTPSとHTTP防御 | 一部 | TLS終端、HTTPからHTTPSへのredirect、HSTS、CSP、`frame-ancestors`、`nosniff`、Referrer-Policyを本番responseで確認する。proxy経由でもschemeとclient IPを誤認しない |
| 認証ライフサイクル | 一部 | メール確認、単回使用・短期限のpassword reset、credential変更通知を実装する。登録・login・resetはアカウントの存在をresponse本文や顕著な時間差で漏らさない |
| login防御 | 未実装 | アカウント単位と送信元単位のrate limit、段階的backoff、監視可能なlockout方針を設ける。DoSに使える永久lockは避ける |
| session管理 | 一部 | idle timeoutとabsolute timeoutを明示し、login・password変更・権限変更時にtokenをrotateする。全deviceからのlogoutと期限切れrecordの定期削除を確認する |
| 認可とデータ分離 | 一部 | すべてのread/write/procedure/shardでサーバー側owner確認を行い、別ユーザーID、欠損ID、改ざんIDのdeny testを置く。管理操作は通常ユーザー経路と分離する |
| CSRF・入力・abuse対策 | 一部 | TopcoatのOrigin検証を無効化せず、全状態変更を対象にtestする。request body上限、文字数・数値範囲、timeout、危険なURLやHTMLを扱う場合のallowlistを定義する |
| DB schemaと整合性 | 一部 | `push_schema`を起動時migrationとして使わない。version付きmigration、foreign key、unique制約、transaction、前方・後方互換のdeploy手順を用意し、空DBと既存DBの両方で検証する |
| backupと復旧 | 未実装 | 暗号化backup、保持期間、別障害領域への保管、RPO/RTOを決める。backup取得ではなく、隔離環境へのrestoreと件数・参照整合性の確認を定期実行する |
| observabilityと障害対応 | 未実装 | request ID付きstructured log、latency/error/rate limit/DB指標、health/readiness endpoint、alert、runbookを用意する。password、session token、reset token、本文中の個人情報はlogへ出さない |
| privacyとアカウント終了 | 未実装 | privacy policy、利用目的、保存期間、問い合わせ窓口を表示する。本人によるexport・退会・削除と、backupから期限後に消える手順を実装する。法令判断は専門家の確認を受ける |
| releaseとsupply chain | 一部 | CIでformat、Clippy、全test、browser E2Eを実行する。binary applicationの`Cargo.lock`を固定し、依存脆弱性・license・secretをscanする。rollback手順とmigration失敗時の停止条件を確認する |
| 脅威分析 | 未実装 | asset、Cookie、procedure、shard、DB、mailを含むtrust boundaryを図示し、credential stuffing、session theft、CSRF、IDOR、XSS、data lossをrelease前にreviewする |

### P1: 初期運用で追加する条件

| 領域 | 条件 |
| --- | --- |
| 強い認証 | 管理者・support操作にはMFAまたはpasskeyを必須にする。一般ユーザーへの導入はriskとrecovery運用を含めて決める |
| session可視化 | 利用中device、最終利用時刻、個別失効を本人が確認できるようにする |
| 時刻 | JST固定を廃止し、ユーザーのtimezoneと日付境界を保存・testする |
| SQLite運用 | 単一writerと永続volumeを前提に、WAL、busy timeout、disk容量、connection数を負荷testする。複数region・多数同時writeが必要になる前にDB移行基準を決める |
| 検索・pagination | ユーザーあたりの件数上限と目標latencyを決め、全件loadをやめる閾値、index、pagination、入力debounceを実測から決める |
| accessibility | keyboard操作、focus、label、contrast、screen reader、200% zoomを主要browserで確認する |
| 運用者機能 | 本人確認を伴うsupport手順、audit log、誤操作を戻せる管理操作、脆弱性報告窓口を用意する |

判断基準には、[OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/)、[Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)、[Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)、[Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)、[Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)を使用します。日本で個人情報を扱う際は、個人情報保護委員会の[法令・ガイドライン](https://www.ppc.go.jp/personalinfo/legal/)も確認します。

## サンプルとしての境界

- ローカル認証にはArgon2idを使いますが、メール確認、パスワード再設定、ログイン試行のrate limitは未実装です。
- 復習日は日本向けMVPとしてJST固定です。ユーザーごとのタイムゾーン設定はありません。
- 新規または空のSQLiteファイルだけに`push_schema`でスキーマを反映します。ファイルの有無を見る処理はmigrationではないため、本番運用では明示的なversion付きmigrationへ置き換える必要があります。
- ToastyはTopcoat公式サンプルと揃えた0.7系です。最新Toastyへの移行はこの検証の対象外です。
- 技検索は個人利用規模のMVPとして、ユーザーのカードを読み込んで最大8件へ絞ります。大量データ向けの全文検索や入力debounceは対象外です。
- Topcoat自体がearly-stageのため、version更新時はmacro、runtime asset、Cookie/session APIの再検証が必要です。
