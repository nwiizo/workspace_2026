//! 競技プログラミング用ライブラリ
//!
//! typical90 から抽出した再利用可能なアルゴリズム集
//!
//! ## モジュール一覧
//!
//! | モジュール | 内容 |
//! |-----------|------|
//! | `convolution` | NTT、多項式乗算 |
//! | `data_structures` | BIT、セグ木（クロージャ版）、遅延セグ木、組み合わせ |
//! | `dp` | LIS、LCS、編集距離、部分列カウント、累積和 |
//! | `flow` | 最大フロー (Dinic)、最小費用流 |
//! | `graph` | BFS、ダイクストラ、ベルマンフォード、フロイドワーシャル、SCC、2-SAT、Union-Find、トポロジカルソート、0-1 BFS、クラスカル |
//! | `math` | GCD/LCM、mod累乗、逆元、行列累乗、座標圧縮、next_permutation、篩、素因数分解、約数列挙、拡張GCD |
//! | `modint` | ModInt (自動mod計算) |
//! | `search` | 二分探索、カッコ列検証 |
//! | `segtree` | Monoid trait ベースのセグ木 (Sum, Max, Min, Gcd) |
//! | `string_algo` | Z-Algorithm、Rolling Hash、Suffix Array、LCP Array |
//! | `testing` | ストレステスト、ランダム生成 |
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use typical90::graph::{dijkstra, UnionFind};
//! use typical90::math::{mod_pow, compress};
//! use typical90::dp::{lis, lcs};
//! use typical90::segtree::{SegTree, Sum};
//! ```

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
