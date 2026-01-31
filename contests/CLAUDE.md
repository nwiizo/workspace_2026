# CLAUDE.md

## Overview

Programming contests and security challenges (CTF, competitive programming).

## Structure

```
contests/
├── juice_shop/     # CTF (Web security)
├── atcoder/        # AtCoder contests
├── codeforces/     # Codeforces rounds
└── leetcode/       # LeetCode problems
```

## Guidelines

- Document solutions with thought process, not just answers
- Include setup instructions for reproducibility
- Keep sensitive data (flags, credentials) local only

---

## 競技プログラミング

### ディレクトリ構成

```
atcoder/abc300/
├── a.rs            # 解答コード
├── b.rs
└── README.md       # コンテストメモ（オプション）
```

### コマンド

```bash
# Rust (cargo-compete)
cargo compete new abc300    # コンテスト取得
cargo compete test a        # テスト
cargo compete submit a      # 提出

# Python (oj)
oj download URL             # テストケース取得
oj test -c "python main.py" # テスト
oj submit URL main.py       # 提出
```

### 計算量の目安

| N | 許容計算量 | 典型アルゴリズム |
|---|-----------|-----------------|
| 10^8 | O(N) | 線形探索、累積和 |
| 10^6 | O(N), O(N log N) | ソート、二分探索 |
| 10^5 | O(N log N), O(N√N) | セグ木、平方分割 |
| 10^4 | O(N²) | 2重ループ、DP |
| 10^3 | O(N² log N), O(N³) | 3重ループ、Floyd-Warshall |
| 10^2 | O(N³), O(N⁴) | 行列累乗 |
| 20-25 | O(2^N) | bit全探索、半分全列挙 |
| 10-12 | O(N!) | 順列全探索 |

---

### データ構造

#### 基本データ構造

| 構造 | 用途 | 計算量 | 使用場面 |
|------|------|--------|----------|
| **配列** | 順序付きデータ | O(1) アクセス | 基本 |
| **HashMap** | キー検索 | O(1) 平均 | 存在判定、カウント |
| **HashSet** | 重複排除 | O(1) 平均 | ユニーク判定 |
| **deque** | 両端操作 | O(1) 両端 | BFS、スライディングウィンドウ |
| **priority_queue** | 最大/最小取得 | O(log N) | ダイクストラ、貪欲 |

#### 高度なデータ構造

| 構造 | 用途 | 計算量 | 使用場面 |
|------|------|--------|----------|
| **Union-Find** | 集合管理 | O(α(N)) ≈ O(1) | 連結判定、グループ化 |
| **Segment Tree** | 区間クエリ | O(log N) | 区間和、区間最小/最大 |
| **BIT (Fenwick Tree)** | 区間和 | O(log N) | 転倒数、累積和更新 |
| **遅延セグ木** | 区間更新+クエリ | O(log N) | 区間加算+区間和 |
| **Trie** | 文字列検索 | O(|S|) | 接頭辞検索、辞書 |
| **SparseTable** | 静的RMQ | O(1) クエリ | 更新なし区間最小 |

#### Union-Find 実装

```rust
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // 経路圧縮
        }
        self.parent[x]
    }
    fn unite(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry { return false; }
        // Union by rank
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else {
            self.parent[ry] = rx;
            if self.rank[rx] == self.rank[ry] { self.rank[rx] += 1; }
        }
        true
    }
}
```

#### Segment Tree 実装

```rust
struct SegTree<T, F> {
    n: usize,
    data: Vec<T>,
    e: T,      // 単位元
    op: F,     // 結合関数
}

impl<T: Clone, F: Fn(&T, &T) -> T> SegTree<T, F> {
    fn new(n: usize, e: T, op: F) -> Self {
        Self { n, data: vec![e.clone(); 2 * n], e, op }
    }
    fn set(&mut self, mut i: usize, x: T) {
        i += self.n;
        self.data[i] = x;
        while i > 1 { i /= 2; self.data[i] = (self.op)(&self.data[2*i], &self.data[2*i+1]); }
    }
    fn query(&self, mut l: usize, mut r: usize) -> T {
        l += self.n; r += self.n;
        let (mut vl, mut vr) = (self.e.clone(), self.e.clone());
        while l < r {
            if l & 1 == 1 { vl = (self.op)(&vl, &self.data[l]); l += 1; }
            if r & 1 == 1 { r -= 1; vr = (self.op)(&self.data[r], &vr); }
            l /= 2; r /= 2;
        }
        (self.op)(&vl, &vr)
    }
}
```

---

### グラフアルゴリズム

#### 最短経路アルゴリズムの使い分け

| アルゴリズム | 条件 | 計算量 | 用途 |
|-------------|------|--------|------|
| **BFS** | 重みなし/重み1 | O(V + E) | 迷路、グリッド |
| **0-1 BFS** | 重み0か1 | O(V + E) | 特殊グラフ |
| **ダイクストラ** | 非負重み | O(E log V) | 一般的な最短路 |
| **ベルマンフォード** | 負辺あり | O(VE) | 負閉路検出 |
| **Floyd-Warshall** | 全点対 | O(V³) | 小規模全点対 |
| **SPFA** | 負辺あり | O(VE) 最悪 | 実用的に高速 |

#### ダイクストラ法

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

fn dijkstra(graph: &Vec<Vec<(usize, i64)>>, start: usize) -> Vec<i64> {
    let n = graph.len();
    let mut dist = vec![i64::MAX; n];
    let mut heap = BinaryHeap::new();

    dist[start] = 0;
    heap.push(Reverse((0i64, start)));

    while let Some(Reverse((d, v))) = heap.pop() {
        if d > dist[v] { continue; } // 既により短い経路で訪問済み
        for &(next, cost) in &graph[v] {
            let nd = d + cost;
            if nd < dist[next] {
                dist[next] = nd;
                heap.push(Reverse((nd, next)));
            }
        }
    }
    dist
}
```

#### BFS (幅優先探索)

```rust
use std::collections::VecDeque;

fn bfs(graph: &Vec<Vec<usize>>, start: usize) -> Vec<i32> {
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

#### DFS (深さ優先探索)

```rust
fn dfs(graph: &Vec<Vec<usize>>, v: usize, visited: &mut Vec<bool>) {
    visited[v] = true;
    for &next in &graph[v] {
        if !visited[next] {
            dfs(graph, next, visited);
        }
    }
}
```

#### グラフの典型問題

| 問題 | アルゴリズム | キーワード |
|------|-------------|-----------|
| 連結成分数 | Union-Find / DFS | 島の数 |
| 二部グラフ判定 | BFS/DFS で彩色 | 2色塗り |
| トポロジカルソート | DFS / Kahn | DAG、依存関係 |
| 最小全域木 | Kruskal / Prim | MST |
| 木の直径 | 2回BFS | 最遠点 |
| LCA | ダブリング | 最近共通祖先 |
| SCC | Kosaraju / Tarjan | 強連結成分 |

---

### 動的計画法 (DP)

#### DP パターン分類

| パターン | 例 | 状態設計 |
|---------|-----|----------|
| **ナップサック** | 部分和、価値最大 | dp[i][w] = i番目まで見て重さwの時の最大価値 |
| **区間DP** | 行列積、括弧付け | dp[l][r] = 区間[l,r]の最適値 |
| **bit DP** | 巡回セールスマン | dp[S][v] = 集合Sを訪問し現在vにいる最小コスト |
| **桁DP** | N以下で条件満たす数 | dp[i][tight][state] |
| **木DP** | 木の独立集合 | dp[v][0/1] = vを選ぶ/選ばない時の部分木の値 |
| **確率DP** | 期待値計算 | dp[state] = その状態からの期待値 |
| **LCS** | 最長共通部分列 | dp[i][j] = S[0..i]とT[0..j]のLCS長 |
| **LIS** | 最長増加部分列 | dp[i] = 長さiのLISの末尾最小値 |
| **編集距離** | 文字列変換 | dp[i][j] = S[0..i]→T[0..j]の最小操作 |

#### ナップサック DP

```rust
// 0-1 ナップサック
fn knapsack(n: usize, w: usize, items: &[(usize, i64)]) -> i64 {
    let mut dp = vec![0i64; w + 1];
    for &(weight, value) in items {
        for j in (weight..=w).rev() {  // 逆順！
            dp[j] = dp[j].max(dp[j - weight] + value);
        }
    }
    dp[w]
}
```

#### LIS (最長増加部分列)

```rust
use std::cmp::Ordering;

fn lis(a: &[i64]) -> usize {
    let mut dp = vec![];  // dp[i] = 長さi+1のLISの末尾最小値
    for &x in a {
        match dp.binary_search_by(|&y| if y < x { Ordering::Less } else { Ordering::Greater }) {
            Ok(i) | Err(i) => {
                if i == dp.len() { dp.push(x); }
                else { dp[i] = x; }
            }
        }
    }
    dp.len()
}
```

#### 累積和

```rust
// 1次元累積和
let mut prefix = vec![0i64; n + 1];
for i in 0..n {
    prefix[i + 1] = prefix[i] + a[i];
}
// 区間[l, r)の和 = prefix[r] - prefix[l]

// 2次元累積和
let mut sum = vec![vec![0i64; m + 1]; n + 1];
for i in 0..n {
    for j in 0..m {
        sum[i+1][j+1] = sum[i+1][j] + sum[i][j+1] - sum[i][j] + a[i][j];
    }
}
// 矩形[(r1,c1), (r2,c2))の和 = sum[r2][c2] - sum[r1][c2] - sum[r2][c1] + sum[r1][c1]
```

---

### 探索テクニック

#### 二分探索

```rust
// 条件を満たす最小のx (lower_bound)
fn binary_search<F: Fn(i64) -> bool>(lo: i64, hi: i64, f: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if f(mid) { hi = mid; }
        else { lo = mid + 1; }
    }
    lo
}

// 使用例: a[i] >= x となる最小の i
let idx = binary_search(0, n as i64, |i| a[i as usize] >= x);
```

#### bit全探索

```rust
// 2^n 通りの部分集合を列挙
for mask in 0..(1 << n) {
    let mut subset = vec![];
    for i in 0..n {
        if mask >> i & 1 == 1 {
            subset.push(i);
        }
    }
    // subsetを使った処理
}
```

#### 半分全列挙 (Meet in the Middle)

```rust
// N=40 程度の問題を O(2^(N/2)) で解く
fn meet_in_the_middle(a: &[i64], target: i64) -> bool {
    let n = a.len();
    let mid = n / 2;

    // 前半の全部分和
    let mut left = vec![];
    for mask in 0..(1 << mid) {
        let sum: i64 = (0..mid).filter(|&i| mask >> i & 1 == 1).map(|i| a[i]).sum();
        left.push(sum);
    }
    left.sort();

    // 後半を列挙しながら二分探索
    for mask in 0..(1 << (n - mid)) {
        let sum: i64 = (0..n-mid).filter(|&i| mask >> i & 1 == 1).map(|i| a[mid + i]).sum();
        if left.binary_search(&(target - sum)).is_ok() {
            return true;
        }
    }
    false
}
```

---

### 数学

#### 素数・約数

```rust
// エラトステネスの篩
fn sieve(n: usize) -> Vec<bool> {
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    for i in 2..=n {
        if is_prime[i] {
            for j in (i*i..=n).step_by(i) {
                is_prime[j] = false;
            }
        }
    }
    is_prime
}

// 約数列挙 O(√N)
fn divisors(n: i64) -> Vec<i64> {
    let mut res = vec![];
    let mut i = 1;
    while i * i <= n {
        if n % i == 0 {
            res.push(i);
            if i != n / i { res.push(n / i); }
        }
        i += 1;
    }
    res.sort();
    res
}
```

#### GCD・LCM

```rust
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a, b) * b
}
```

#### mod演算

```rust
const MOD: i64 = 998244353;  // or 1_000_000_007

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

// フェルマーの小定理による逆元 (mが素数の場合)
fn mod_inv(a: i64, m: i64) -> i64 {
    mod_pow(a, m - 2, m)
}
```

---

### Rust Tips

```rust
// 高速入力
use proconio::input;
use proconio::marker::{Chars, Usize1, Isize1};

// よく使うマクロ
macro_rules! chmin { ($a:expr, $b:expr) => { if $b < $a { $a = $b; true } else { false } } }
macro_rules! chmax { ($a:expr, $b:expr) => { if $b > $a { $a = $b; true } else { false } } }

// BinaryHeap (最小ヒープ)
use std::collections::BinaryHeap;
use std::cmp::Reverse;
let mut heap = BinaryHeap::new();
heap.push(Reverse(x));

// 隣接リスト
let mut graph: Vec<Vec<usize>> = vec![vec![]; n];
for (a, b) in edges {
    graph[a].push(b);
    graph[b].push(a);
}

// ソート (降順)
v.sort_by(|a, b| b.cmp(a));
// タプルでソート (2番目の要素で)
v.sort_by_key(|x| x.1);

// イテレータ
let sum: i64 = v.iter().sum();
let max = v.iter().max().unwrap();
let count = v.iter().filter(|&&x| x > 0).count();
```

### Python Tips

```python
# 高速化
import sys
input = sys.stdin.readline
sys.setrecursionlimit(10**6)

# よく使う
from collections import defaultdict, deque, Counter
from heapq import heappush, heappop, heapify
from bisect import bisect_left, bisect_right, insort
from itertools import permutations, combinations, accumulate
from functools import lru_cache, reduce
from math import gcd, lcm, isqrt, ceil, floor
import operator

# 無限大
INF = float('inf')

# 二分探索
from bisect import bisect_left
# a[i] >= x となる最小の i
idx = bisect_left(a, x)

# 累積和
from itertools import accumulate
prefix = list(accumulate(a, initial=0))

# defaultdict
graph = defaultdict(list)
counter = defaultdict(int)
```

---

### 学習リソース

#### 公式・準公式

- [競プロ典型90問](https://atcoder.jp/contests/typical90) - E869120氏による90問
- [Educational DP Contest](https://atcoder.jp/contests/dp) - AtCoder公式DP練習
- [CSES Problem Set](https://cses.fi/problemset/) - 300問以上の練習問題
- [競技プログラミングの鉄則](https://atcoder.jp/contests/tessoku-book) - 公式教材

#### アルゴリズム解説

- [CP-Algorithms](https://cp-algorithms.com/) - 英語の包括的リファレンス
- [アルゴリズムロジック](https://algo-logic.info/) - 日本語解説
- [けんちょんの競プロ精進記録](https://drken1215.hatenablog.com/) - 典型DP解説など

#### 実装集

- [AtCoder Library (ACL)](https://github.com/atcoder/ac-library) - 公式ライブラリ
- [Competitive Programmer's Handbook](https://cses.fi/book/book.pdf) - 無料PDF教科書

---

## CTF 攻略の学び

### Web セキュリティ脆弱性カテゴリ

| カテゴリ | 説明 | 対策 |
|---------|------|------|
| **SQLi** | SQL文への不正入力。`' OR 1=1--` でログインバイパス、UNION で情報抽出 | プリペアドステートメント、入力検証 |
| **XSS** | DOM/Reflected/Stored。`<iframe src="javascript:alert('xss')">` | 出力エスケープ、CSP |
| **NoSQLi** | MongoDB等で `{"$ne": -1}` により条件バイパス | 入力型検証、演算子フィルタ |
| **XXE** | XML外部エンティティで `/etc/passwd` 等を読み取り | 外部エンティティ無効化 |
| **IDOR** | `/api/basket/2` のようにIDを変えて他者データにアクセス | 認可チェック |
| **JWT操作** | `alg: none` で署名検証をバイパス | アルゴリズム固定、署名必須 |
| **CSRF** | 別オリジンからの偽造リクエスト | SameSite Cookie、CSRFトークン |
| **Path Traversal** | `../` やNull Byte (`%2500`) でファイルアクセス | パス正規化、拡張子検証 |

### 攻略のコツ

1. **DevTools を常に開く**: Network タブでリクエスト/レスポンスを監視
2. **API を直接叩く**: UI操作より `fetch()` の方が速い
3. **ソースマップを確認**: `main.js.map` から元のコードを復元
4. **エラーメッセージを活用**: スタックトレースから内部構造を推測
5. **認証トークン**: `localStorage.getItem('token')` でJWT取得

### Playwright MCP 自動化パターン

```
# 基本フロー
1. browser_navigate → ページ移動
2. browser_snapshot → 要素ref確認
3. browser_type/click → 操作実行
4. browser_evaluate → JavaScript実行（fetch等）

# SQLi ログイン
browser_type → email: "' OR 1=1--"
browser_type → password: "a"
browser_click → Login

# API 直接実行
browser_evaluate → () => fetch('/api/endpoint', {...}).then(r => r.json())
```

### チャレンジドキュメントの形式

```markdown
# チャレンジ名 ✅/❌

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi / XSS / 認証 など
**目標:** 具体的なゴール

## 思考プロセス
[なぜこの攻撃が効くのかの考察]

## 実行手順
[再現可能なステップ]

## コード/ペイロード
[攻撃コード]

## 解説
[脆弱性の原因と対策]
```

### 参考リソース

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [PayloadsAllTheThings](https://github.com/swisskyrepo/PayloadsAllTheThings)
- [HackTricks](https://book.hacktricks.xyz/)
- [PortSwigger Web Security Academy](https://portswigger.net/web-security)
