//! # procon-lib
//!
//! Comprehensive competitive programming library for Rust.
//!
//! ACL (AtCoder Library) compatible and beyond.
//!
//! ## Modules
//!
//! | Category | Module | Description |
//! |----------|--------|-------------|
//! | **Data Structures** | [`segtree`] | Segment Tree with Monoid trait |
//! | | [`lazy_segtree`] | Lazy Segment Tree with presets |
//! | | [`data_structures`] | BIT, Combination |
//! | **Graph** | [`graph`] | BFS, Dijkstra, SCC, 2-SAT, Union-Find |
//! | | [`tree`] | LCA, HLD, Rerooting DP |
//! | | [`mst`] | Kruskal, Prim |
//! | | [`flow`] | Max Flow, Min Cost Flow |
//! | **Math** | [`math`] | GCD, LCM, mod pow, matrix |
//! | | [`modint`] | Automatic modular arithmetic |
//! | | [`prime`] | Miller-Rabin, factorization, sieve |
//! | | [`number_theory`] | CRT, extended GCD, discrete log |
//! | **String** | [`string_algo`] | Z-Algorithm, Suffix Array, LCP, Rolling Hash |
//! | | [`trie`] | Trie, XOR Trie |
//! | | [`palindrome`] | Manacher |
//! | **DP** | [`dp`] | LIS, LCS, prefix sum |
//! | | [`dp_opt`] | CHT, Li Chao Tree, D&C Opt |
//! | **Geometry** | [`geometry`] | 2D geometry primitives |
//! | **Utility** | [`util`] | Coordinate compression, imos, grid |
//! | | [`search`] | Binary search |
//! | | [`convolution`] | NTT |
//!
//! ## Quick Start
//!
//! ```rust
//! use procon_lib::graph::UnionFind;
//! use procon_lib::modint::Mint998;
//! use procon_lib::segtree::{SegTree, Sum};
//!
//! // Union-Find
//! let mut uf = UnionFind::new(10);
//! uf.unite(0, 1);
//! assert!(uf.same(0, 1));
//!
//! // ModInt
//! let a = Mint998::new(2);
//! let b = a.pow(10);
//! assert_eq!(b.val(), 1024);
//!
//! // Segment Tree
//! let mut seg: SegTree<Sum> = SegTree::new(10);
//! seg.set(0, Sum(5));
//! seg.set(1, Sum(3));
//! assert_eq!(seg.query(0, 2).0, 8);
//! ```

// =============================================================================
// Macros
// =============================================================================

/// Update minimum value
///
/// Returns `true` if the value was updated.
///
/// # Example
/// ```
/// use procon_lib::chmin;
///
/// let mut a = 10;
/// assert!(chmin!(a, 5));
/// assert_eq!(a, 5);
/// assert!(!chmin!(a, 7));
/// assert_eq!(a, 5);
/// ```
#[macro_export]
macro_rules! chmin {
    ($a:expr, $b:expr) => {{
        let b = $b;
        if b < $a {
            $a = b;
            true
        } else {
            false
        }
    }};
}

/// Update maximum value
///
/// Returns `true` if the value was updated.
///
/// # Example
/// ```
/// use procon_lib::chmax;
///
/// let mut a = 5;
/// assert!(chmax!(a, 10));
/// assert_eq!(a, 10);
/// assert!(!chmax!(a, 7));
/// assert_eq!(a, 10);
/// ```
#[macro_export]
macro_rules! chmax {
    ($a:expr, $b:expr) => {{
        let b = $b;
        if b > $a {
            $a = b;
            true
        } else {
            false
        }
    }};
}

// =============================================================================
// Modules
// =============================================================================

// Data Structures
pub mod data_structures;
pub mod lazy_segtree;
pub mod segtree;

// Graph
pub mod flow;
pub mod graph;
pub mod mst;
pub mod tree;

// Math
pub mod math;
pub mod modint;
pub mod number_theory;
pub mod prime;

// String
pub mod palindrome;
pub mod string_algo;
pub mod trie;

// DP
pub mod dp;
pub mod dp_opt;

// Geometry
pub mod geometry;

// Utility
pub mod convolution;
pub mod search;
pub mod util;

// =============================================================================
// Re-exports for convenience
// =============================================================================

pub use modint::{Mint107, Mint998, ModInt};
pub use segtree::{Gcd, Max, Min, Monoid, SegTree, Sum};

// =============================================================================
// Constants
// =============================================================================

/// Common modulo for NTT-friendly operations
pub const MOD: i64 = 998_244_353;

/// Common modulo for general operations
pub const MOD2: i64 = 1_000_000_007;

/// Large value for infinity
pub const INF: i64 = 1_000_000_000_000_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chmin() {
        let mut a = 10;
        assert!(chmin!(a, 5));
        assert_eq!(a, 5);
        assert!(!chmin!(a, 7));
        assert_eq!(a, 5);
    }

    #[test]
    fn test_chmax() {
        let mut a = 5;
        assert!(chmax!(a, 10));
        assert_eq!(a, 10);
        assert!(!chmax!(a, 7));
        assert_eq!(a, 10);
    }
}
