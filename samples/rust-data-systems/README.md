# Rust Data Systems -- Verification Code

データシステムの基礎概念をRustで実装し、理論値と実測値を比較検証するためのコード集。

ブログ記事シリーズに対応する10個のcrateを含む。各crateは `cargo test` で計測まで実行できる。

## Crates

| Crate | テーマ | 主な実装 |
|-------|--------|----------|
| `bloom-filter` | 確率的データ構造 | `BloomFilter`, `CountingBloomFilter`, `BlockedBloomFilter` |
| `consistent-hashing` | キー配置 | `HashRing` (vnode), `RendezvousHashing`, `jump_consistent_hash`, `MementoHash` |
| `crdt` | 結果整合性 | `GCounter`, `PNCounter`, `ORSet`, `LWWRegister`, `DeltaGCounter` |
| `transaction-isolation` | MVCCトランザクション | `MvccStore` (RC/SI/SSI/Cahill SSI) |
| `lsm-tree` | ストレージエンジン | `Memtable`, `SSTable`, `LsmTree` (`compact_tiered`) |
| `fencing-token` | 分散リース安全性 | `FencingToken`, `FencedStorage`, `CasStorage` (S3 If-Match風) |
| `logical-clocks` | 論理時計 | `LamportClock`, `VectorClock`, `HLC` (`receive_bounded`) |
| `tail-latency` | 尾部遅延の数理 | `theoretical_tail_probability`, `tied_request_latency`, `adaptive_hedge_one_request` |
| `watermark` | ストリーム時間管理 | `EventTimeWindowing`, `WatermarkTracker` (`aligned_pause_set`) |
| `idempotency-key` | 二重処理防止 | `IdempotentPaymentService`, `AtomicIdempotentService`, `OutboxService` |

## ビルド・テスト

```sh
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## 設計方針

- Rust 2024 edition
- `thiserror` で型付きエラー
- `Result<T, E>` で失敗を表現、production code に `.unwrap()` を残さない
- proptest で半束則・冪等性などの不変条件を検証
- 教育目的の単純化を優先し、本番品質の最適化（バイナリフォーマット、ロックフリー化等）は意図的に省略

## 出典

シリーズの各記事は別途公開されている。記事と実装はセットで読むことを想定している。
