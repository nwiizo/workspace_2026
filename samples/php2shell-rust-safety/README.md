# Rustのメモリ安全性と、保証されないセキュリティ

PHP2Shellを入口に、メモリの寿命、初期化途中の中断、認可の違いを確かめる小さなRustサンプルです。PHP2ShellのRCEを再現する攻撃コードではありません。ネットワーク通信や外部サービスを使わず、固定データと少量のメモリで動きます。外部クレートへの依存はありません。

## 確認すること

| 対象 | コード | 期待する結果 |
| --- | --- | --- |
| 解放後の参照と未初期化の値 | `src/lib.rs`のdoctest | 2例はコンパイルに失敗し、所有権を移す例は成功する |
| safeなAPIの初期化と中断 | `tests/lifecycle.rs` | panic後のVecには初期化済みの要素だけが残る。処理全体の取り消しは保証されない |
| 範囲外アクセス | `tests/lifecycle.rs` | インデックスアクセスはpanicし、`get()`は`None`を返す |
| 認可漏れ | `tests/authorization.rs`、`examples/authorization.rs` | IDだけの検索は他テナントの請求書を返す。認証済みテナントも条件に加えた検索は拒否する |
| 誤ったunsafe実装 | `examples/unsafe_initialization.rs` | Miriが未初期化メモリの読み取りを検出し、異常終了する |

テナントとは、この例では同じサービスを利用する別々の顧客組織です。認証の処理自体は省略し、認証済みの利用者から得たテナントIDを固定値で渡します。リクエストの自己申告をそのままテナントIDとして信頼すれば、修正後の検索でも認可にはなりません。

## 通常の実行

Rust 1.90以降を使います。以下はこのディレクトリで実行します。`rtk`を使わない環境では、`rtk proxy`の部分を外せます。

```sh
rtk proxy cargo test --all
rtk proxy cargo run --example authorization
```

テストは実行時の6件とdoctestの3件、計9件が成功します。doctestのうち2件は「コンパイルできないこと」を成功とします。認可デモの出力は次のとおりです。

```text
authenticated tenant: 1
ID-only lookup: Some(Invoice { id: 202, tenant_id: 2, description: "Tenant 2 private invoice" })
tenant-scoped lookup: None
```

IDだけの検索は、Rustのメモリ安全性を満たしたまま他テナントのデータを返します。テストはこの不具合も意図的に確認しています。「テストが成功した」という事実と、何を期待値にしたかは分けて読んでください。

## Miriでメモリの使い方を検査する

MiriはRustのコードを実行して、未初期化メモリの使用などの未定義動作を検出するツールです。nightlyのMiriとrust-srcが必要です。導入する環境でのみ、次を実行します。

```sh
rtk proxy rustup component add miri rust-src --toolchain nightly
rtk proxy cargo +nightly miri test --test authorization --test lifecycle
```

この6件はすべて成功します。認可漏れのある検索にも、Miriが検出するメモリの不正操作はありません。Miriは「誰に請求書を返してよいか」を判断するツールではありません。

次は、**検出による失敗を期待する別の実験**です。

```sh
rtk proxy cargo +nightly miri run --example unsafe_initialization
```

この例は`Vec`に2要素分の領域を確保した後、初期化前に`unsafe { values.set_len(2) }`を呼びます。1要素だけを書き、途中でpanicさせます。panicを捕まえても、2要素目の初期化は完了しません。その領域を読むと、Miriは次の種類のエラーを出して終了します。

```text
error: Undefined Behavior: reading memory ... but memory is uninitialized
```

最初に表示されるpanicは実験で意図的に発生させたものです。その後の`Undefined Behavior`が確認対象です。実測では終了コード1でした。Miriの診断文や内部のメモリアドレスはバージョンによって変わります。

`set_len()`の安全性の条件を破っているのは、初期化より先に長さを増やした時点です。Miriが読み取り箇所で報告することは、その前の操作が正しかったことを意味しません。通常の`Vec`のAPIを使うコードには、この誤った長さを公開する操作はありません。

ネイティブ実行では未定義動作の例を動かさず、Miri用コマンドを表示して終了コード2で終了します。

```sh
rtk proxy cargo run --example unsafe_initialization
```

この例は中断と初期化の関係を切り出したモデルです。PHPのメモリ不足や`longjmp()`と、Rustのpanicは同じ動作ではありません。このRustの例では実際のOOMを発生させていません。また、`catch_unwind()`はabortを捕まえず、DB更新や送信済みの通知を取り消しません。

## 検証記録

2026年10月22日、macOS / aarch64で確認しました。

- Rust 1.99.1で`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all`、`cargo test --all --all-targets`が成功しました。
- Rust 1.90でも`cargo test --all`の9件が成功しました。
- Miri `0.1.0 (1a2c4797ef 2026-10-21)`で通常の6件が成功し、unsafeの例では未初期化メモリの読み取りを検出しました。
- 通常実行のunsafe例が終了コード2で止まり、認可デモが上記の結果を表示することを確認しました。

`#![forbid(unsafe_code)]`はライブラリと認可デモに指定しています。検証用のunsafe例は別のコンパイル対象です。依存先や標準ライブラリの内部まで、この属性でunsafeの使用を禁止できるわけではありません。Miriの成功も、実行していない入力や処理経路の安全性を証明しません。

## PHPのメモリ不足と終了処理を観察する

`php/oom.php`は、PHPの`memory_limit`を超えたときにどこまで処理が進むかを観察する小さな例です。`ok`は1 MiBの文字列を2個、`oom`は最大32個まで配列に追加します。PHPの上限を16 MiBに設定し、正常終了、`finally`、その後の処理、終了処理のコールバックに出力を置いています。

**この実験はPHP2Shellのメモリ破壊やRCEを再現するものではありません。** CLIで単純なメモリ不足を発生させるだけです。Webリクエストの解析、ApacheモジュールやPHP-FPMのワーカーの再利用、解放済みの領域の読み書き、攻撃者による実行先の制御は検証していません。

2026年10月22日、macOS上の既存のDocker context `colima-arrove-fax-ui`を使い、Linux / arm64の公式PHPイメージで実行しました。デフォルトのDocker contextは変更していません。確認したイメージのdigestは次のとおりです。タグの内容が変わる可能性があるため、同じ内容を使うときはdigestを指定します。

| イメージ | RepoDigest |
| --- | --- |
| `php:8.4.26-cli-alpine` | `php@sha256:37c8c64177612b72e7f9d2c00481739c166a3708bea27c1cfd27fea1291d82d1` |
| `php:8.4.27-cli-alpine` | `php@sha256:8d29b132476e5f54633d30bfbfa0b0b7c95a9cb0c308b3175d61351aea73b643` |

各実行は使い捨てのコンテナです。ネットワークを無効にし、ポートを公開せず、非rootユーザーで実行しました。マウントしたのはこのサンプルの`php/`だけで、コンテナのファイルシステムとともに読み取り専用です。コンテナ全体のメモリを128 MiB、CPUを1、プロセス数を32に制限しています。PHPの16 MiB上限に先に達してエラーになる条件であり、ホストのメモリを使い切る実験ではありません。

実行したコマンドの共通部分を含めた、8.4.27のOOM実験は次のとおりです。別の環境ではcontext名とマウント元の絶対パスを手元の配置に合わせます。

```sh
rtk proxy docker --context colima-arrove-fax-ui run \
  --rm --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --user 65534:65534 \
  --memory 128m --cpus 1 --pids-limit 32 \
  --mount type=bind,src=/Users/nwiizo/ghq/github.com/nwiizo/workspace_2026/samples/php2shell-rust-safety/php,dst=/lab,readonly \
  php:8.4.27-cli-alpine \
  php -n -d memory_limit=16M -d display_errors=stderr -d log_errors=0 \
  /lab/oom.php oom
```

構文検査と各モードを含む実行記録です。コンテナ引数と`php -n -d memory_limit=16M -d display_errors=stderr -d log_errors=0`はすべて上と同じで、イメージとその後の引数だけを表のとおり変えました。`-n`によりイメージ内の`php.ini`を読み込まず、PHPのメモリ上限を明示しています。

| イメージ | PHPオプションに続く引数 | 終了コード | 結果 |
| --- | --- | --- | --- |
| `php:8.4.26-cli-alpine` | `-l /lab/oom.php` | 0 | `No syntax errors detected in /lab/oom.php` |
| `php:8.4.26-cli-alpine` | `/lab/oom.php ok` | 0 | 正常終了。以下の正常時の出力 |
| `php:8.4.26-cli-alpine` | `/lab/oom.php oom` | 255 | メモリ不足。以下のOOM時の出力 |
| `php:8.4.27-cli-alpine` | `-l /lab/oom.php` | 0 | `No syntax errors detected in /lab/oom.php` |
| `php:8.4.27-cli-alpine` | `/lab/oom.php ok` | 0 | 正常終了。以下の正常時の出力 |
| `php:8.4.27-cli-alpine` | `/lab/oom.php oom` | 255 | メモリ不足。以下のOOM時の出力 |
| `php:8.4.27-cli-alpine` | `/lab/oom.php invalid` | 2 | `usage: php oom.php [ok\|oom]` |

正常時の出力は両バージョンで同じ順番でした。先頭行の`PHP_VERSION`だけが異なります。

```text
PHP 8.4.26, mode=ok, memory_limit=16M
before allocation
allocation completed
finally block
after allocation
shutdown callback
```

8.4.27でも先頭行が`PHP 8.4.27, mode=ok, memory_limit=16M`になり、その後は上と同じでした。

OOM時の出力は次のとおりです。

```text
PHP 8.4.26, mode=oom, memory_limit=16M
before allocation
Fatal error: Allowed memory size of 16777216 bytes exhausted (tried to allocate 1052672 bytes) in /lab/oom.php on line 23
shutdown callback
```

```text
PHP 8.4.27, mode=oom, memory_limit=16M
before allocation
Fatal error: Allowed memory size of 16777216 bytes exhausted (tried to allocate 2097152 bytes) in /lab/oom.php on line 23
shutdown callback
```

両バージョンとも、OOM時には`allocation completed`、`finally block`、`after allocation`が出ず、`shutdown callback`が出ました。修正後の8.4.27でも、このPHPコードの`finally`は実行されていません。したがって、`finally`の不実行、OOMエラー、終了コード255を見ただけでは、PHP2Shellの脆弱性やRCEの成立を判定できません。メモリ不足で処理が中断することと、PHP内部のCの実装で不整合な状態を扱いメモリを破壊することは、区別して確認する必要があります。

この小さな例で終了処理のコールバックが実行されたことも、あらゆるメモリ不足で終了処理が完了する保証にはなりません。出力中の割り当て要求サイズの違いについては、原因や悪用可能性をこの実験から推測していません。

## 参照

- [PHP公式：OOMによるメモリ破壊](https://github.com/php/php-src/security/advisories/GHSA-w8hw-w9fx-j79w)、[古いグローバル状態の使用](https://github.com/php/php-src/security/advisories/GHSA-6wmf-4rv5-p3rp)
- [Rustonomicon：Exception Safety](https://doc.rust-lang.org/nomicon/exception-safety.html)、[Vec::set_lenの要件](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len)
- [The Rust Programming Language：References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [Miri](https://github.com/rust-lang/miri)、[OWASP：Broken Object Level Authorization](https://owasp.org/API-Security/editions/2023/en/0xa1-broken-object-level-authorization/)
- [記事の草稿](../../blogs/contents/php2shell-rust-safety.md)
