# rust-types-as-walls

関数型まつり2026 公募セッション「型は壁、Rustでもバグを直すな、表現できなくせよ」のスライドに含まれるサンプルコードを、実コンパイル可能な形で検証するためのプロジェクトです。

## ブログ記事シリーズ

このコードを下敷きにした記事シリーズが `blogs/contents/rust-types-as-walls/` にあります。各記事が下敷きにする主なサンプルは次の通りです。

| 記事 | テーマ | 主なサンプル |
|------|--------|-------------|
| 001 | 不正な状態は、なぜ生まれるのか | 01, 02, 03 |
| 002 | 状態ごとに型を分ける | 04, 07, 08 |
| 003 | ワークフローを型で貫く | 05, 09, 11 |
| 004 | 境界でだけ parse する | 14, 15, 16 |
| 005 | 公開 API を sealed で閉じる | 17, 18, 22, `sealed_payment`, `api_evolution` |
| 006 | 型の壁を摩擦にしない | 19, 20, 21, 23, `customer_id`, `password` |
| 007 | 摩擦と関数型の道具 | 12, 13, 24, 25, 26, 27, 33 |
| 008 | 統合: ミニ注文サービス | `order_service`, `customer_id`, `idiomatic_email` |

## 実行方法

```sh
# 全サンプルをビルド
cargo build --examples

# 個別のサンプルを実行
cargo run --example 01_illegal_states_problem
cargo run --example 05_workflow
cargo run --example 14_boundary_parse
cargo run --example 16_idiomatic_smart_constructor
cargo run --example 18_non_exhaustive
cargo run --example 24_persistent_vector
cargo run --example 27_tap_pipeline
cargo run --example 33_rayon_map_reduce

# 品質ゲート
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## サンプル一覧

| # | ファイル | 対応スライド |
|---|---|---|
| 01 | `01_illegal_states_problem.rs` | 不正な状態は、なぜ生まれるのか / 何が「不正」なのか / なぜ型はこれを止められなかったか |
| 02 | `02_ownership_wall.rs` | 所有権という壁 |
| 03 | `03_immutability_wall.rs` | イミュータビリティという壁 |
| 04 | `04_state_types.rs` | パターン1: 状態ごとに型を分ける |
| 05 | `05_workflow.rs` | ワークフロー全体を型で貫く |
| 06 | `06_newtype.rs` | パターン2: newtype |
| 07 | `07_smart_constructor.rs` | パターン3: Smart Constructor + 複数の制約 + Parse, don't validate |
| 08 | `08_miu.rs` | パターン4: Make Illegal States Unrepresentable + コンパイル時ユニットテスト |
| 09 | `09_type_state.rs` | Type State + PhantomData |
| 10 | `10_nonzero_nonempty.rs` | 既製の制約型 (2024 edition `NonZero<T>`) |
| 11 | `11_evolving_design.rs` | 設計を進化させるときも、型を作る |
| 12 | `12_friction_ownership.rs` | 摩擦1: 所有権と状態遷移 |
| 13 | `13_friction_variants.rs` | 摩擦2: enumバリアント間のフィールド重複 |
| 14 | `14_boundary_parse.rs` | 境界: HTTPリクエストの parse |
| 15 | `15_db_roundtrip.rs` | 境界: DBスキーマとADTの往復 |
| 16 | `16_idiomatic_smart_constructor.rs` | より現実的な Smart Constructor (`TryFrom<&str>` / `FromStr`) |
| 17 | `17_sealed_trait.rs` | sealed trait / closed enum による公開APIの壁 |
| 18 | `18_non_exhaustive.rs` | `#[non_exhaustive]` による将来互換のある enum / struct 設計 |
| 19 | `19_ergonomic_newtype.rs` | `From` / `TryFrom` + `serde(transparent)` で newtype の ergonomics を上げる |
| 20 | `20_const_generic_password.rs` | const generic + Smart Constructor |
| 21 | `21_builder_vs_smart_constructor.rs` | Builder と Smart Constructor の役割分担 |
| 22 | `22_sealed_state_machine.rs` | sealed trait + PhantomData による closed state machine |
| 23 | `23_error_conversion.rs` | `thiserror` による階層化エラー変換 |
| 24 | `24_persistent_vector.rs` | `Vec` と `im-rs` 系 `Vector` の対比。構造共有つき永続ベクタを検証 |
| 25 | `25_persistent_hash_map.rs` | `im-rs` 系 `HashMap` による純粋 update。`12_friction_ownership.rs` の対比例 |
| 26 | `26_itertools_vs_std.rs` | `chunk_by` / `tuple_windows` / `cartesian_product` / `intersperse` の対比 |
| 27 | `27_tap_pipeline.rs` | `tap::Pipe` / `tap::Tap` による左から右の Result パイプライン |
| 29 | `29_derive_more_newtypes.rs` | `derive_more` で `CustomerId` / `OrderId` の boilerplate を削減 |
| 33 | `33_rayon_map_reduce.rs` | `rayon` による FP スタイル並列 map-reduce |

## FP ツール統合後の構成

- 採用した道具は `derive_more`, `im-rc`, `itertools`, `rayon`, `tap`。`05_workflow.rs` は `tap::Pipe` で左から右に読める形へ寄せ、`src/customer_id.rs` と `src/order_service.rs` の `CustomerId` / `OrderId` は `NonZeroU64` を内側に持たせつつ `Display` / `AsRef` / `From<NonZeroU64>` を derive しました。
- 改善が分かりやすかったのは 3 つです。`im-rs` 系コレクションは「元の値を残したまま更新したい」話に素直に乗り、`itertools` は iterator の途中で考えを切らずに済み、`rayon` は純粋関数の map-reduce をそのまま並列化できました。`tap` は Rust 標準のメソッドチェーンに近い手触りのまま、関数パイプを suffix position に持ち込めるのが見どころです。
- 合わなかった、または今回は見送った道具もあります。`cargo fetch` はこの環境では `https://index.crates.io/config.json` の名前解決に失敗したため、ローカル cache に無い crate は採用できませんでした。そのため `nutype`, `frunk`, `do-notation` は今回未導入です。また `im` 本体も online fetch ができず、手元に cache があった同系列の `im-rc` で 24/25 を代替しました。

Rust は関数型言語ではないので、「全部を式で繋いで終わり」にはなりません。エラー型の持ち上げ、所有権の移動、`collect` の挿入、境界での `String` 化のように、どこかで具象化や調停が必要になります。その代わり、newtype、state type、sealed trait、Smart Constructor のような「型で不変条件を閉じる」道具は非常に強く、ここはむしろ ML 系や Haskell の設計感覚と相性が良いです。

今回追加したライブラリ群で見えたのは、Rust の FP らしさは「純粋性の徹底」よりも「副作用の境界を狭め、変換を合成し、違法状態を型へ押し戻す」ところにある、ということです。永続データ構造、iterator combinator、pipe、map-reduce 並列はその補助線としてかなり有効でしたが、複数エラー蓄積の applicative 合成のような領域は依然として専用 crate への依存度が高く、そこは今後の課題として残りました。

## テスト

- `tests/idiomatic_email.rs`: `TryFrom<&str>` / `FromStr` の Smart Constructor を通常テストと生成ケースで検証
- `tests/sealed_payment.rs`: sealed trait による state 遷移 API の動作確認
- `tests/api_evolution.rs`: `#[non_exhaustive]` を前提にした downstream 側コードの通常系テスト
- `tests/customer_id.rs`: newtype の `From` / `TryFrom` / `serde` 連携を検証
- `tests/ui.rs`: fixture crate に対する `cargo check --offline` で compile-fail を回し、sealed trait と `#[non_exhaustive]` が実際に「壁」として効くことを確認

## 前提

- Rust 1.85+ / edition = "2024"
- 依存: `thiserror`, `chrono`, `serde`, `serde_json`, `axum`, `tokio`, `sqlx`, `tower`
- FP ツール統合で追加: `derive_more`, `im-rc`, `itertools`, `rayon`, `tap`
