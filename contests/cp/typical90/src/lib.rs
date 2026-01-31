//! 競技プログラミング用ライブラリ
//!
//! typical90 から抽出した再利用可能なアルゴリズム集
//!
//! ## モジュール一覧
//!
//! | モジュール | 内容 |
//! |-----------|------|
//! | `convolution` | NTT、多項式乗算 |
//! | `data_structures` | BIT、セグ木、組み合わせ |
//! | `dp` | LIS、LCS、部分列カウント、累積和 |
//! | `flow` | 最大フロー、最小費用流 |
//! | `graph` | BFS、ダイクストラ、SCC、2-SAT、Union-Find |
//! | `math` | GCD/LCM、mod累乗、逆元、行列累乗 |
//! | `modint` | ModInt (自動mod計算) |
//! | `search` | 二分探索、カッコ列検証 |
//! | `segtree` | Monoid trait ベースのセグ木 |
//! | `string_algo` | Z-Algorithm、Rolling Hash、Suffix Array、LCP |

pub mod convolution;
pub mod data_structures;
pub mod dp;
pub mod flow;
pub mod graph;
pub mod math;
pub mod modint;
pub mod search;
pub mod segtree;
pub mod string_algo;
pub mod testing;
