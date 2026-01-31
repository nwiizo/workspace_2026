// 049 - Flip Digits 2 (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_aw
//
// ============================================================
// 問題の理解
// ============================================================
//
// 区間[L, R]を反転する操作について考える。
//
// 【XOR的な視点】
// ビット列の代わりに「境界」で考える:
// - N個のビットには N+1 個の境界がある (0, 1, 2, ..., N)
// - 区間[L, R]の反転 = 境界(L-1)と境界Rをトグル
//
// 例: ビット列 "0101" の場合
// 境界: 0 | 0 | 1 | 0 | 1
//       ^   ^   ^   ^   ^
//       0   1   2   3   4
//
// 区間[2,3]を反転すると境界1と境界3がトグル
//
// ============================================================
// グラフ問題への変換
// ============================================================
//
// - 頂点: 0, 1, ..., N (N+1個の境界点)
// - 辺: アイテムiは頂点(L_i-1)と頂点R_iを結ぶ辺、重みC_i
//
// 「任意のビット列に変換できる」条件:
// = すべての隣接頂点ペア(0-1, 1-2, ..., (N-1)-N)が連結
// = 頂点0からNまでがすべて連結
//
// したがって、最小全域木を求める問題！
// Kruskal法で O(M log M) で解ける
//
// ============================================================

use proconio::input;

fn main() {
    input! {
        n: usize,
        m: usize,
        items: [(i64, usize, usize); m], // (C, L, R)
    }
    println!("{}", solve(n, &items));
}

fn solve(n: usize, items: &[(i64, usize, usize)]) -> i64 {
    // 辺のリストを作成: (コスト, 頂点u, 頂点v)
    let mut edges: Vec<(i64, usize, usize)> = Vec::with_capacity(items.len());

    for &(cost, l, r) in items {
        // アイテム(C, L, R)は頂点(L-1)と頂点Rを結ぶ辺
        edges.push((cost, l - 1, r));
    }

    // コストで昇順ソート
    edges.sort_by_key(|e| e.0);

    // Union-Find で最小全域木を構築 (Kruskal法)
    let mut uf = UnionFind::new(n + 1);
    let mut total_cost = 0i64;
    let mut edge_count = 0;

    for (cost, u, v) in edges {
        if !uf.same(u, v) {
            uf.unite(u, v);
            total_cost += cost;
            edge_count += 1;
        }
    }

    // すべての頂点(0からN)が連結か確認
    // 連結なら辺の数は N 本
    if edge_count == n { total_cost } else { -1 }
}

// Union-Find (素集合データ構造)
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // 経路圧縮
        }
        self.parent[x]
    }

    fn unite(&mut self, x: usize, y: usize) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return false;
        }
        // ランクによる結合
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else {
            self.parent[ry] = rx;
            if self.rank[rx] == self.rank[ry] {
                self.rank[rx] += 1;
            }
        }
        true
    }

    fn same(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // アイテム1,2を購入: 1 + 1 = 2
        let items = vec![(1, 1, 1), (1, 2, 2), (10, 1, 2)];
        assert_eq!(solve(2, &items), 2);
    }

    #[test]
    fn test_example2() {
        // アイテム1,3を購入: 1 + 1 = 2
        let items = vec![(1, 1, 1), (10, 2, 2), (1, 1, 2)];
        assert_eq!(solve(2, &items), 2);
    }

    #[test]
    fn test_example3() {
        // N=4, M=5
        // 辺: (0,2), (1,4), (2,4), (0,4), (1,4)
        // 頂点3が孤立 → 連結にできない → -1
        let items = vec![
            (3, 1, 2), // 頂点0-2
            (5, 2, 4), // 頂点1-4
            (9, 3, 4), // 頂点2-4
            (4, 1, 4), // 頂点0-4
            (8, 2, 4), // 頂点1-4
        ];
        assert_eq!(solve(4, &items), -1);
    }
}
