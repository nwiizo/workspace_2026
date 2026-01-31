# procon-lib

Comprehensive competitive programming library for Rust.

## Features

- **ACL Compatible**: Covers all AtCoder Library functionality
- **Zero Dependencies**: Uses only Rust standard library
- **Type Safe**: Leverages Rust's type system for safety
- **Well Documented**: Every function has examples and complexity analysis
- **Tested**: Comprehensive test coverage

## Quick Start

```rust
use procon_lib::graph::UnionFind;
use procon_lib::modint::Mint998;

fn main() {
    // Union-Find
    let mut uf = UnionFind::new(10);
    uf.unite(0, 1);
    assert!(uf.same(0, 1));

    // ModInt
    let a = Mint998::new(2);
    let b = a.pow(1000000);
    println!("{}", b);
}
```

## Modules

### Data Structures
- `segtree` - Segment Tree with Monoid trait
- `lazy_segtree` - Lazy Segment Tree with presets
- `data_structures` - BIT, Combination

### Graph
- `graph` - BFS, Dijkstra, SCC, 2-SAT, Union-Find
- `tree` - LCA, HLD, Rerooting DP
- `mst` - Kruskal, Prim
- `flow` - Max Flow, Min Cost Flow

### Math
- `math` - GCD, LCM, mod pow, matrix
- `modint` - Automatic modular arithmetic
- `prime` - Miller-Rabin, factorization
- `number_theory` - CRT, extended GCD

### String
- `string_algo` - Z-Algorithm, Suffix Array, LCP, Rolling Hash
- `trie` - Trie, XOR Trie
- `palindrome` - Manacher

### DP
- `dp` - LIS, LCS, prefix sum
- `dp_opt` - CHT, Li Chao Tree

### Geometry
- `geometry` - 2D geometry primitives

### Utility
- `util` - Coordinate compression, imos method
- `search` - Binary search
- `convolution` - NTT

## License

MIT OR Apache-2.0
