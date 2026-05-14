# Rust ホットリロード検証サンプル

2026年現在で利用可能な3つのホットリロードアプローチを実際に検証するサンプルコード。

## 構成

| ディレクトリ | アプローチ | クレートバージョン |
|---|---|---|
| `part1-hot-lib-reloader/` | 動的ライブラリ方式（汎用） | hot-lib-reloader 0.8.2 |
| `part2-dioxus/` | RSX パッチ + Subsecond | dioxus 0.7.4 |
| `part3-leptos/` | view パッチ + Subsecond | leptos 0.8.17 |

## 前提条件

```sh
# Rust stable
rustup update stable

# WASM ターゲット（Part 2, 3 で必要）
rustup target add wasm32-unknown-unknown

# cargo-watch（Part 1 の lib 自動リビルドに使用）
cargo install cargo-watch

# dx CLI（Part 2）
cargo install dioxus-cli

# trunk（Part 3 CSR モード）
cargo install trunk
```

> **注意**: 環境に `RUSTFLAGS="-C target-cpu=native"` が設定されている場合、wasm32 ビルドが失敗します。
> Part 2 は `RUSTFLAGS="" dx serve`、Part 3 は `RUSTFLAGS="-C target-cpu=generic" trunk serve` で実行してください。

> **注意**: `dx` コマンドが Deno の alias になっている場合があります。`which dx` で `~/.cargo/bin/dx` を指していることを確認してください。

## Part 1: hot-lib-reloader

ロジックを dylib に分離し、`libloading` 経由で実行中にリロードする汎用的なアプローチ。

### 実行方法

```sh
# ワークスペースルート (hotreload-rs/) で実行

# ターミナル1: lib を監視して自動リビルド
cargo watch -w part1-hot-lib-reloader/lib -x 'build -p hot-lib'

# ターミナル2: アプリを実行（ホットリロードモード）
cargo run -p hot-app

# 静的リンクモード（比較用）
cargo run -p hot-app --no-default-features
```

### 検証シナリオ

1. **関数のロジック変更**: `lib/src/lib.rs` の `step()` で `counter += 1` → `counter += 10` に変更して保存 → 次のリロードで反映されるか
2. **表示変更**: `render()` のフォーマットを変更 → 反映確認
3. **型変更（危険）**: `State` に `pub extra: f64` フィールドを追加 → segfault するか確認
4. **シリアライズ回避**: `step_serialized()` を使った JSON 経由のパターンで型変更に耐えるか確認
5. **debounce**: 保存から反映までの時間を体感（`file_watch_debounce: 300` ms 設定）

### 構造

```
part1-hot-lib-reloader/
├── app/          # bin クレート（hot-lib-reloader でライブラリをロード）
│   └── src/main.rs
└── lib/          # dylib クレート（ホットリロード対象）
    └── src/lib.rs
```

### feature flag による切り替え

- `cargo run -p hot-app` → `reload` feature 有効、dylib を動的ロード
- `cargo run -p hot-app --no-default-features` → 通常の静的リンク

## Part 2: Dioxus 0.7

RSX ホットリロード（テンプレートパッチ）と Subsecond（Rust ホットパッチ）の2層構成。

### 実行方法

```sh
cd part2-dioxus

# RSX ホットリロード（デフォルト）
dx serve

# RSX + Subsecond ホットパッチ
dx serve --hotpatch
```

### 検証シナリオ

#### RSX ホットリロード（`dx serve`）
- 要素の追加・削除・属性変更
- フォーマット文字列内の変数移動（`{count}` を別の場所へ）
- for ループ / if 条件ブロック内の要素変更
- コンポーネント props のリテラル値変更

#### Subsecond（`dx serve --hotpatch`）
- `compute()` 関数のロジック変更（`n * n` → `n * n * n`）
- hooks 内の計算変更（`doubled` の計算式）
- 新しい変数・式の追加 → フルリビルドが走るか

#### CSS ホットリロード
- `assets/main.css` の色やレイアウト変更 → リロードなしで反映

## Part 3: Leptos 0.8

view テンプレートパッチと CSS ホットリロード。Subsecond 統合は実験的。

### 実行方法

```sh
cd part3-leptos

# CSR モード（trunk）— このサンプルのデフォルト
trunk serve --open
```

> **SSR モードについて**: `cargo leptos watch --hot-reload` による view ホットリロードは SSR 構成が必要です（`[package.metadata.leptos]` セクション、`ssr` feature、サーバーバイナリの定義等）。このサンプルは CSR 構成のため trunk を使用します。

### 検証シナリオ

#### view パッチ
- HTML 構造変更（要素追加・削除、クラス変更）
- テキストリテラルの変更
- `Show` / `For` コンポーネント内の変更
- 壊れるパターンの記録（DOM の不整合など）

#### CSS ホットリロード
- `style/main.css` の変更 → ページリロードなしで反映

#### Subsecond（実験的）
- `Cargo.toml` で `leptos = { features = ["csr", "subsecond"] }` を追加
- `compute()` のロジック変更が反映されるか

## 横断比較

| 観点 | hot-lib-reloader | Dioxus 0.7 | Leptos 0.8 |
|---|---|---|---|
| 対象領域 | 汎用（ゲーム、CLI 等） | UI フレームワーク | Web フレームワーク |
| リロード粒度 | 関数単位 | RSX / Rust / アセット | view / CSS / Rust |
| 状態保持 | 手動管理 | 自動（RSX）/ 手動（Subsecond） | 自動（view）/ 未検証（Subsecond） |
| stable Rust | ✅ | RSX: ✅ / Subsecond: 要確認 | ✅ |
| 型変更への耐性 | ❌（segfault） | RSX: N/A / Subsecond: 制限あり | 未検証 |
| セットアップの手間 | 中（workspace 分離） | 低（dx CLI） | 中（cargo-leptos） |
| 成熟度 | 安定（v0.8.2） | RSX 安定 / Subsecond 実験的 | view 安定 / Subsecond 初期 |

## 計測方法

変更保存 → 画面反映までの秒数を手動計測:

```sh
# 変更前にタイムスタンプ記録
date +%s%3N
# ここでファイルを保存 → 反映を目視確認して再度:
date +%s%3N
```
