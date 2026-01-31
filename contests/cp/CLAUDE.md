# CLAUDE.md - Competitive Programming

## Overview

競技プログラミングのコンテスト解答を管理するディレクトリ。

## Structure

```
cp/
├── abc300/         # AtCoder Beginner Contest 300
│   ├── a.rs
│   ├── b.rs
│   └── ...
├── typical90/      # 競プロ典型90問
├── edpc/           # Educational DP Contest
└── round900/       # Codeforces Round 900
```

## Workflow

```bash
# cargo-compete (推奨)
cargo compete new abc300    # コンテスト取得
cargo compete test a        # テスト
cargo compete submit a      # 提出

# online-judge-tools
oj download URL
oj test -c "cargo run --bin a"
oj submit URL a.rs
```

---

## 計算量の目安

| N | 許容計算量 | 典型アルゴリズム |
|---|-----------|-----------------|
| 10^8 | O(N) | 線形探索、累積和 |
| 10^6 | O(N), O(N log N) | ソート、二分探索 |
| 10^5 | O(N log N), O(N√N) | セグ木、平方分割 |
| 10^4 | O(N²) | 2重ループ、DP |
| 10^3 | O(N² log N), O(N³) | Floyd-Warshall |
| 20-25 | O(2^N) | bit全探索、半分全列挙 |
| 10-12 | O(N!) | 順列全探索 |

---

## データ構造

### 基本

| 構造 | 計算量 | 用途 |
|------|--------|------|
| HashMap/HashSet | O(1) | 存在判定、カウント |
| BinaryHeap | O(log N) | ダイクストラ、貪欲 |
| VecDeque | O(1) 両端 | BFS |

### 高度

| 構造 | 計算量 | 用途 |
|------|--------|------|
| Union-Find | O(α(N)) | 連結判定、グループ化 |
| Segment Tree | O(log N) | 区間クエリ |
| BIT | O(log N) | 転倒数、累積和更新 |

---

## グラフ

### 最短経路の選択

| 条件 | アルゴリズム | 計算量 |
|------|-------------|--------|
| 重みなし | BFS | O(V + E) |
| 重み0/1 | 0-1 BFS | O(V + E) |
| 非負重み | ダイクストラ | O(E log V) |
| 負辺あり | ベルマンフォード | O(VE) |
| 全点対 | Floyd-Warshall | O(V³) |

### 典型問題

| 問題 | 手法 |
|------|------|
| 連結成分数 | Union-Find / DFS |
| 二部グラフ判定 | 2色塗り (BFS/DFS) |
| トポロジカルソート | Kahn / DFS |
| 最小全域木 | Kruskal / Prim |
| 木の直径 | 2回BFS |
| LCA | ダブリング |

---

## 動的計画法 (DP)

### パターン

| パターン | 状態設計 |
|---------|----------|
| ナップサック | dp[i][w] = i番目まで見て重さwの最大価値 |
| 区間DP | dp[l][r] = 区間[l,r]の最適値 |
| bit DP | dp[S][v] = 集合Sを訪問し現在vの最小コスト |
| 桁DP | dp[i][tight][state] |
| 木DP | dp[v][0/1] = vを選ぶ/選ばない時の部分木の値 |

### よく使うDP

- **LIS** (最長増加部分列): 二分探索で O(N log N)
- **LCS** (最長共通部分列): O(NM)
- **編集距離**: O(NM)
- **部分和**: O(N × sum)

---

## 探索テクニック

### 二分探索

```rust
// 条件を満たす最小のx
fn binary_search<F: Fn(i64) -> bool>(lo: i64, hi: i64, f: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if f(mid) { hi = mid; } else { lo = mid + 1; }
    }
    lo
}
```

### bit全探索

```rust
for mask in 0..(1 << n) {
    for i in 0..n {
        if mask >> i & 1 == 1 {
            // i番目を選択
        }
    }
}
```

### 半分全列挙

N=40 を O(2^20) に分割して二分探索で合流。

---

## 数学

### 頻出

```rust
// GCD
fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a } else { gcd(b, a % b) } }

// 繰り返し二乗法
fn mod_pow(mut base: i64, mut exp: i64, m: i64) -> i64 {
    let mut result = 1;
    base %= m;
    while exp > 0 {
        if exp & 1 == 1 { result = result * base % m; }
        exp >>= 1;
        base = base * base % m;
    }
    result
}

// 逆元 (mが素数)
fn mod_inv(a: i64, m: i64) -> i64 { mod_pow(a, m - 2, m) }
```

### 定数

```rust
const MOD: i64 = 998244353;     // NTT friendly
const MOD2: i64 = 1_000_000_007; // よく使う素数
const INF: i64 = 1_000_000_000_000_000_000; // 10^18
```

---

## Rust テンプレート

```rust
use proconio::input;
use proconio::marker::{Chars, Usize1};

macro_rules! chmin { ($a:expr, $b:expr) => { if $b < $a { $a = $b; true } else { false } } }
macro_rules! chmax { ($a:expr, $b:expr) => { if $b > $a { $a = $b; true } else { false } } }

fn main() {
    input! {
        n: usize,
        a: [i64; n],
    }
    println!("{}", solve(n, &a));
}

fn solve(n: usize, a: &[i64]) -> i64 {
    // 実装
    0
}
```

---

## Python テンプレート

```python
import sys
input = sys.stdin.readline
sys.setrecursionlimit(10**6)

from collections import defaultdict, deque, Counter
from heapq import heappush, heappop
from bisect import bisect_left, bisect_right
from itertools import permutations, combinations, accumulate
from functools import lru_cache

INF = float('inf')
MOD = 998244353

def main():
    n = int(input())
    a = list(map(int, input().split()))
    print(solve(n, a))

def solve(n, a):
    return 0

if __name__ == "__main__":
    main()
```

---

## 実装スニペット

### Union-Find

```rust
struct UnionFind { parent: Vec<usize>, rank: Vec<usize> }
impl UnionFind {
    fn new(n: usize) -> Self { Self { parent: (0..n).collect(), rank: vec![0; n] } }
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x { self.parent[x] = self.find(self.parent[x]); }
        self.parent[x]
    }
    fn unite(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry { return false; }
        if self.rank[rx] < self.rank[ry] { self.parent[rx] = ry; }
        else { self.parent[ry] = rx; if self.rank[rx] == self.rank[ry] { self.rank[rx] += 1; } }
        true
    }
}
```

### ダイクストラ

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

fn dijkstra(graph: &[Vec<(usize, i64)>], start: usize) -> Vec<i64> {
    let n = graph.len();
    let mut dist = vec![i64::MAX; n];
    let mut heap = BinaryHeap::new();
    dist[start] = 0;
    heap.push(Reverse((0i64, start)));
    while let Some(Reverse((d, v))) = heap.pop() {
        if d > dist[v] { continue; }
        for &(next, cost) in &graph[v] {
            if d + cost < dist[next] {
                dist[next] = d + cost;
                heap.push(Reverse((dist[next], next)));
            }
        }
    }
    dist
}
```

### BFS

```rust
use std::collections::VecDeque;

fn bfs(graph: &[Vec<usize>], start: usize) -> Vec<i32> {
    let n = graph.len();
    let mut dist = vec![-1; n];
    let mut queue = VecDeque::new();
    dist[start] = 0;
    queue.push_back(start);
    while let Some(v) = queue.pop_front() {
        for &next in &graph[v] {
            if dist[next] == -1 {
                dist[next] = dist[v] + 1;
                queue.push_back(next);
            }
        }
    }
    dist
}
```

### 累積和

```rust
// 1次元
let prefix: Vec<i64> = std::iter::once(0).chain(a.iter().scan(0, |acc, &x| { *acc += x; Some(*acc) })).collect();
// 区間[l, r)の和 = prefix[r] - prefix[l]

// 2次元
let mut sum = vec![vec![0i64; m + 1]; n + 1];
for i in 0..n {
    for j in 0..m {
        sum[i+1][j+1] = sum[i+1][j] + sum[i][j+1] - sum[i][j] + a[i][j];
    }
}
```

---

## 典型90問から学んだパターン

### 001 - Yokan Party (★4) - 答えで二分探索

```rust
// 「最小値を最大化」「最大値を最小化」は二分探索
// check(x) = 「条件xを満たせるか」を判定関数として二分探索
fn binary_search_on_answer<F: Fn(i64) -> bool>(lo: i64, hi: i64, check: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if check(mid) { lo = mid; } else { hi = mid; }
    }
    lo
}
```

### 002 - Encyclopedia of Parentheses (★3) - bit全探索 + 検証

- N≤20ならbit全探索 (2^20≈10^6)
- カッコ列の妥当性: どの位置でも `'(' の数 >= ')' の数`

### 003 - Longest Circular Road (★4) - 木の直径

```rust
// 木の直径 = 任意の点から最遠点を求め、そこから再度最遠点を求める
let (u, _) = bfs(0);    // 任意の点から最遠点
let (_, d) = bfs(u);    // 直径
```

### 004 - Cross Sum (★2) - 前計算で高速化

```rust
// 行和・列和を前計算 → O(1)で十字の和を計算
// answer[i][j] = row_sum[i] + col_sum[j] - a[i][j]
```

### 005 - Restricted Digits (★7) - 行列累乗

- DP遷移が線形 → 行列で表現可能 → N乗を O(B³ log N) で計算
- 大きなNでも log N に落とせる

### 006 - Smallest Subsequence (★5) - 貪欲 + 前計算

```rust
// next[i][c] = 位置i以降で文字cが最初に現れる位置
// 貪欲に辞書順最小の文字を選んでいく
```

### 007 - CP Classes (★3) - 二分探索（partition_point）

```rust
// Rustのpartition_point = C++のlower_bound相当
let pos = sorted.partition_point(|&x| x < target);
// pos: target以上の最小のインデックス
// pos-1: target未満の最大のインデックス
```

### 008 - AtCounter (★4) - 部分列カウントDP

```rust
// 特定文字列の部分列を数える
// dp[i] = target の最初のi文字を作る方法の数
// 後ろから更新して重複を防ぐ
for &c in s {
    for i in (0..target.len()).rev() {
        if c == target[i] { dp[i+1] += dp[i]; }
    }
}
```

### 010 - Score Sum Queries (★2) - 条件別累積和

```rust
// 条件ごとに累積和を別々に持つ
// → 区間クエリを O(1) で処理
```

---

## 実装から得た重要な学び

### 1. Edition 2024 の変更点

```rust
// Edition 2024 ではパターンマッチの挙動が変わる
// ❌ 古い書き方
.max_by_key(|(_, &d)| d)

// ✅ 新しい書き方
.max_by_key(|&(_, d)| d)
```

### 2. 問題の見極め方

| キーワード | 典型アルゴリズム |
|-----------|-----------------|
| 「最小値を最大化」「最大値を最小化」 | 答えで二分探索 |
| 「部分列」「何通り」 | DP（後ろから更新） |
| 「木」「最長経路」 | 2回BFS/DFS（木の直径） |
| 「区間の和」「クエリ」 | 累積和またはセグ木 |
| N ≤ 20 | bit全探索 |
| N ≤ 10^18、遷移が線形 | 行列累乗 |

### 3. Rust競プロ Tips

```rust
// partition_point（二分探索）
let pos = sorted.partition_point(|&x| x < target);
// pos: target以上の最小index
// pos-1: target未満の最大index（pos > 0 のとき）

// 固定長配列の初期化（ヒープ確保不要）
let mut dp = [0i64; 8];  // vec! より速い

// clippy対策: 競プロでは範囲ループが必要なことが多い
#[allow(clippy::needless_range_loop)]
fn solve() { ... }
```

### 4. デバッグのコツ

```rust
// テストを必ず書く
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // 問題の入力例をそのままテストに
        assert_eq!(solve(input), expected);
    }
}
```

---

## typical90 ライブラリ

`typical90/src/` に再利用可能なアルゴリズムを整備：

| モジュール | 内容 |
|-----------|------|
| `search` | 二分探索、カッコ列検証 |
| `graph` | BFS、木の直径、Union-Find |
| `math` | GCD/LCM、mod累乗、行列累乗 |
| `dp` | 部分列カウント、1D/2D累積和 |

### 使用例

```rust
use typical90::graph::UnionFind;
use typical90::math::{mod_pow, MOD2};
use typical90::dp::{prefix_sum, range_sum};

// Union-Find
let mut uf = UnionFind::new(n);
uf.unite(0, 1);
if uf.same(0, 2) { ... }

// 累積和
let prefix = prefix_sum(&a);
let sum = range_sum(&prefix, l, r);  // a[l..r] の和
```

---

## 学習リソース

- [競プロ典型90問](https://atcoder.jp/contests/typical90)
- [Educational DP Contest](https://atcoder.jp/contests/dp)
- [CSES Problem Set](https://cses.fi/problemset/)
- [CP-Algorithms](https://cp-algorithms.com/)
- [AtCoder Library (ACL)](https://github.com/atcoder/ac-library)
