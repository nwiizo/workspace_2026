# Async Rust を Rust 2024 edition で動かす

O'Reilly *Async Rust* (Maxwell Flitton & Caroline Morton, 2024) のサンプルコードを、
Rust 2024 edition (1.85, 2025-02 stable) で動かすための検証プロジェクトです。

書籍は 2021 edition / Tokio 1.26〜1.39 / nightly coroutines を前提に書かれています。
2024 edition で安定化された機能とベストプラクティスに沿って書き直すと、書籍コードの
9割は無修正で動く一方で、書き味と安全性が確実に上がる箇所があります。

## 何が変わって、何は変わらないか

| 章 | 主題 | 2024 edition での扱い |
|----|------|----------------------|
| 1 | Async 入門 | 依存バージョンを `tokio = "1.40"` / `reqwest = "0.12"` に上げるだけ。コードは同一。 |
| 2 | Future / Pin / Context | 手書き `Future` の `poll` 実装はそのまま。`std::pin::pin!` でヒープ無しの pin が可能。 |
| 3 | 自作 async キュー | `AsyncFnMut` を境界に置き換えると素直になる。 |
| 4 | hyper / mio 統合 | **依存側が破壊的変更**。書籍は hyper 0.x ベース。hyper 1.0 で API が大幅刷新。 |
| 5 | Coroutines | **要注意**。`std::ops::Coroutine` は依然 nightly。`gen` 予約語と変数名が衝突する。 |
| 6 | Reactive | edition による影響なし。 |
| 7 | Tokio 内部のカスタマイズ | tokio 1.40+ の API は概ね有効。 |
| 8 | Actor model | async fn in trait が安定化したのでボイラープレートを削れる。 |
| 9 | リトライ / サーキットブレーカー | async closures + `AsyncFnMut` で書籍より素直に書ける。 |
| 10 | std で async サーバー | ライフタイムキャプチャ変更で `+ '_` 註釈を外せる箇所が出る。 |
| 11 | テスト (deadlock / race) | **tail expression drop order の変更が直撃する章**。 |

## 適用した 2024 edition ベストプラクティス

調べて反映したものを列挙します。何を allow したかも明記します。

### Cargo.toml

- `edition = "2024"` (resolver = "3" を暗黙に有効化)
- `rust-version = "1.85"` 明示で MSRV-aware resolver の効果を得る
- `[lints.rust]` で `unsafe_op_in_unsafe_fn = deny` / `elided_lifetimes_in_paths = warn`
- `[lints.clippy]` で `pedantic` + `nursery` を warn 既定にしつつ、教科書的サンプルの
  ノイズを `missing_errors_doc / module_name_repetitions / must_use_candidate` allow
- async に効くものを deny / warn:
  - `unwrap_used = deny` (panic を増やさない)
  - `await_holding_lock = deny` / `await_holding_refcell_ref = deny`
  - `unused_async = warn` / `manual_async_fn = warn`
  - `mem_forget = warn` / `dbg_macro = warn` / `print_stderr = warn`

### コード側

- `unwrap()` を全廃。エラーは `thiserror` で型付けして `?` 伝搬
- `Box::pin` の代わりに `std::pin::pin!` マクロ
- `cx.waker().wake_by_ref()` を継続するため `poll` 内で `std::thread::sleep` を呼ばない
- async closure (`async || { ... }`) を `AsyncFnMut` で受ける
- async fn in trait を直接書く (静的ディスパッチ前提)
- `MutexGuard` を `.await` を跨いで保持しない (scope で閉じる)
- `manual_async_fn` などデモ意図で例外的に維持する箇所は `#[expect(..., reason = "...")]`
  で「外したら警告が出るべき」状態に固定する (`#[allow]` ではなく `#[expect]`)
- `let-else` で早期 return を素直に書く
- ライフタイムキャプチャは 2024 既定に任せ、明示的 opt-out は `+ use<>` で示す

## 動かし方

```sh
cargo build --examples
cargo clippy --examples --all-targets
cargo run --example 02_counter_future
cargo run --example 03_async_closure
cargo run --example 04_async_fn_in_trait
cargo run --example 05_lifetime_capture
cargo run --example 06_drop_order
cargo run --example 07_gen_keyword
# 01 はネットワーク必要
cargo run --example 01_hello_async
```

検証環境: rustc 1.95.0 (2026-04-14), edition 2024, tokio 1.52, reqwest 0.12.28。

## サンプル一覧

| ファイル | 対応する書籍の章 | 確認していること |
|----------|------------------|------------------|
| `01_hello_async.rs` | 1章 | tokio/reqwest を最新版に上げてもコードはそのまま動く |
| `02_counter_future.rs` | 2章 | 手書き `Future` + `pin!` マクロでヒープ不要 |
| `03_async_closure.rs` | 3章 / 9章 | `AsyncFnMut` で async クロージャを受ける |
| `04_async_fn_in_trait.rs` | 8章 | `trait { async fn ... }` が stable で書ける |
| `05_lifetime_capture.rs` | 4章 / 10章 | `+ '_` 不要、`+ use<>` で opt-out |
| `06_drop_order.rs` | 11章 | tail 式の一時値が局所変数より先に drop される / Mutex は scope で閉じる |
| `07_gen_keyword.rs` | 5章 | `gen` 予約語回避と `Iterator` での代替 |
| `08_task_queue.rs` | 3章 | `async-task` + `flume` + `LazyLock` で自作タスクキュー |
| `09_std_runtime.rs` | 10章 | `std` だけで `block_on` を実装 (`RawWaker` + `thread::park`) |
| `10_paused_time_test.rs` | 11章 | `tokio::test(start_paused)` + `advance` で仮想時間検証 |

## TODO

書籍の他章を引き続き 2024 edition に寄せていく作業。
優先度高めから順に。

- [ ] **ch4: hyper 1.0 への移行サンプル**。`Client::builder` の置き換え、
      `hyper-util` の `client::legacy::Client` 経由のパターンを最小ペアで示す
- [ ] **ch10: 自作ランタイムを拡張**。現在は `block_on` のみ。`Sender` / `Receiver`
      / `Sleep` を std だけで足してサーバー側まで届ける
- [ ] **ch11: deadlock テストの drop 順検証**。2021 と 2024 で結果が変わる
      ケースを並置する。`loom` 統合も検討
- [ ] **ch8: Actor の dyn 化サンプル**。`#[trait_variant::make]` 使用例と、
      手書き `Pin<Box<dyn Future>>` 版の比較
- [ ] **ch5: gen blocks の nightly 例**。stable 化が進んだら `gen { ... }`
      ブロックを `Iterator` 代替と並置 (現状 nightly のため別 toolchain)
- [ ] **ch3: タスクキューに `tokio::task::JoinSet` を採用した版**。書籍の
      手書きキューとの比較
- [ ] **ch6: イベントバスの `tokio::sync::broadcast` 版**
- [ ] **ベンチ**: `criterion` で `pin!` vs `Box::pin` の比較
- [ ] CI に `cargo deny` / `cargo audit` を追加 (依存ピン留めの確認)

## 参考

- Rust Edition Guide 2024: <https://doc.rust-lang.org/edition-guide/rust-2024/>
- Rust 1.85 release notes: <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/>
- RFC 3498 (lifetime capture rules): <https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html>
- Tokio shared state guide: <https://tokio.rs/tokio/tutorial/shared-state>
