# CLAUDE.md - procon-lib

## Overview

ACL (AtCoder Library) を完全にカバーし、さらに拡張した Rust 競技プログラミングライブラリ。

## Build & Test

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Module Structure

| カテゴリ | モジュール | 内容 |
|---------|-----------|------|
| **データ構造** | `segtree` | Monoid SegTree |
| | `lazy_segtree` | 遅延評価 + プリセット |
| | `data_structures` | BIT, Combination |
| **グラフ** | `graph` | BFS, Dijkstra, SCC, 2-SAT, Union-Find |
| | `tree` | LCA, HLD, Rerooting |
| | `mst` | Kruskal, Prim |
| | `flow` | MaxFlow, MinCostFlow |
| **数学** | `math` | GCD, LCM, mod演算, 行列 |
| | `modint` | ModInt (自動mod計算) |
| | `prime` | Miller-Rabin, Pollard, 篩 |
| | `number_theory` | CRT, extgcd, 離散対数 |
| **文字列** | `string_algo` | Z, SA, LCP, Rolling Hash |
| | `trie` | Trie, XOR Trie |
| | `palindrome` | Manacher |
| **DP** | `dp` | LIS, LCS, 累積和 |
| | `dp_opt` | CHT, Li Chao, D&C Opt |
| **幾何** | `geometry` | 2D幾何全般 |
| **その他** | `util` | 座標圧縮, いもす, グリッド |
| | `search` | 二分探索 |
| | `convolution` | NTT |

## 型エイリアス

```rust
pub type Mint998 = ModInt<998_244_353>;
pub type Mint107 = ModInt<1_000_000_007>;
```

## マクロ

```rust
chmin!(a, b);  // a = min(a, b); returns true if updated
chmax!(a, b);  // a = max(a, b); returns true if updated
```

## 使用例

```rust
use procon_lib::graph::{UnionFind, dijkstra};
use procon_lib::modint::Mint998;
use procon_lib::segtree::{SegTree, Sum};

fn main() {
    // Union-Find
    let mut uf = UnionFind::new(10);
    uf.unite(0, 1);

    // ModInt
    let a = Mint998::new(123456789);
    let b = a.pow(1000);

    // Segment Tree
    let mut seg: SegTree<Sum> = SegTree::new(10);
    seg.set(5, Sum(100));
}
```

## ACL との対応

| ACL | procon-lib |
|-----|------------|
| `atcoder::dsu` | `graph::UnionFind` |
| `atcoder::segtree` | `segtree::SegTree` |
| `atcoder::lazysegtree` | `lazy_segtree::LazySegTree` |
| `atcoder::modint` | `modint::ModInt` |
| `atcoder::maxflow` | `flow::MaxFlow` |
| `atcoder::mincostflow` | `flow::MinCostFlow` |
| `atcoder::scc` | `graph::scc` |
| `atcoder::twosat` | `graph::TwoSat` |
| `atcoder::convolution` | `convolution::convolution` |
| `atcoder::string` | `string_algo::suffix_array`, `string_algo::lcp_array`, `string_algo::z_algorithm` |
| `atcoder::math` | `math::*`, `number_theory::*` |

## ACL を超える機能

- DP最適化: CHT, Li Chao Tree, D&C Opt
- 幾何: 2D幾何全般
- 素数: Miller-Rabin, Pollard's rho
- 木: LCA (ダブリング), HLD
- 文字列: Trie, Manacher
- ユーティリティ: 座標圧縮, いもす法
