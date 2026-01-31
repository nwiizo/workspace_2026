// 068 - Paired Information (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_bp
//
// 重み付きUnion-Find (ポテンシャル付きUnion-Find)
// A[x] + A[y] = v という情報から、A[y] - A[x] = v - 2*A[x] ではなく
// 隣接要素同士の関係を使う
//
// X_i + 1 = Y_i なので、A[i+1] - A[i] = v - 2*A[i] というのは使えない
// 代わりに、A[i] + A[i+1] = v を使う
//
// 偶奇で分けて考える:
// - 偶数インデックスと奇数インデックスは独立
// - A[i] + A[i+1] = v から、同じ偶奇のインデックス間の差が分かる
//
// weighted Union-Find で diff[x] = A[x] - A[root] を管理

use proconio::input;

fn main() {
    input! {
        n: usize,
        q: usize,
        queries: [(usize, usize, usize, i64); q],
    }

    let mut results = Vec::new();
    // 偶数インデックスと奇数インデックスを分けてUnion-Find
    let mut uf = WeightedUnionFind::new(n + 1);

    for (t, x, y, v) in queries {
        if t == 0 {
            // A[x] + A[y] = v (x + 1 = y)
            // 偶奇が異なるので、次の偶数/奇数との差を管理
            // A[x] + A[x+1] = v
            // A[x+2] - A[x] = (A[x+2] + A[x+3]) - (A[x] + A[x+1]) - A[x+3] + A[x+1]
            // これは複雑なので、別のアプローチ：
            // 累積和的に考える: S[i] = A[1] + A[2] + ... + A[i]
            // A[x] + A[x+1] = v → S[x+1] - S[x-1] = v
            // つまり S[y] - S[x-1] = v

            // weighted union-find で S の差を管理
            // S[y] - S[x-1] = v
            uf.unite(x - 1, y, v);
        } else {
            // A[x] = v のとき A[y] = ?
            // S[x] - S[x-1] = A[x] = v
            // S[y] - S[y-1] = A[y]
            //
            // S[x] = S[x-1] + v
            // A[y] = S[y] - S[y-1]
            //
            // x と y の S が連結なら計算可能

            if uf.same(x - 1, y) && uf.same(x - 1, y - 1) {
                // S[y] - S[x-1] = diff(x-1, y)
                // S[y-1] - S[x-1] = diff(x-1, y-1)
                // A[y] = S[y] - S[y-1] = diff(x-1, y) - diff(x-1, y-1)
                //
                // 一方、S[x] = S[x-1] + v なので、
                // S[x] - S[x-1] = v → これは A[x] = v という仮定

                // diff(a, b) = S[b] - S[a] (aがルート側)
                // uf.diff(x-1, y) で S[y] - S[x-1] がわかる
                // uf.diff(x-1, y-1) で S[y-1] - S[x-1] がわかる
                // ただし x-1 が同じ連結成分にあることを確認済み

                // S[x-1] を基準点とする
                // S[x] = S[x-1] + v (仮定)
                // A[y] を求めたい

                // x と x-1 が連結なら、S[x] - S[x-1] の関係がわかる（= A[x]）
                // 実際には x-1 と x は A[x] = S[x] - S[x-1] という関係
                // ただし情報としては与えられていない

                // 再考：
                // 情報 A[x] + A[x+1] = v は S[x+1] - S[x-1] = v と同等
                // クエリ: A[x] = v のとき A[y] = ?
                //
                // y - 1 と x - 1 が連結なら、S[y-1] - S[x-1] がわかる
                // y と x - 1 が連結なら、S[y] - S[x-1] がわかる
                // y と x が連結なら、S[y] - S[x] がわかる
                //
                // A[y] = S[y] - S[y-1]
                // A[x] = S[x] - S[x-1] = v (仮定)
                //
                // x-1 と y-1 と y がすべて連結ならば：
                // S[y] - S[x-1] と S[y-1] - S[x-1] がわかる
                // A[y] = (S[y] - S[x-1]) - (S[y-1] - S[x-1])

                let diff_y = uf.diff(x - 1, y);
                let diff_y1 = uf.diff(x - 1, y - 1);
                results.push(format!("{}", diff_y - diff_y1));
            } else if uf.same(x, y) && uf.same(x, y - 1) {
                // x と y と y-1 が連結
                // S[y] - S[x] と S[y-1] - S[x] がわかる
                // A[y] = S[y] - S[y-1] = (S[y] - S[x]) - (S[y-1] - S[x])
                //
                // ただし A[x] = v という仮定をどう使う？
                // → 実はこのケースでは A[x] の仮定は必要ない（x と y が連結なら）

                let diff_y = uf.diff(x, y);
                let diff_y1 = uf.diff(x, y - 1);
                results.push(format!("{}", diff_y - diff_y1));
            } else {
                results.push("Ambiguous".to_string());
            }
        }
    }

    for r in results {
        println!("{}", r);
    }
}

struct WeightedUnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
    diff_weight: Vec<i64>, // diff_weight[x] = S[x] - S[parent[x]]
}

impl WeightedUnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            diff_weight: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] == x {
            x
        } else {
            let root = self.find(self.parent[x]);
            self.diff_weight[x] += self.diff_weight[self.parent[x]];
            self.parent[x] = root;
            root
        }
    }

    // weight(y) - weight(x) = w となるように unite
    fn unite(&mut self, x: usize, y: usize, w: i64) {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry {
            return;
        }

        // weight(y) - weight(x) = w
        // weight(x) = diff_weight[x] (from root)
        // weight(y) = diff_weight[y] (from root)
        // weight(ry) - weight(rx) を設定

        let w_adjusted = w + self.diff_weight[x] - self.diff_weight[y];

        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
            self.diff_weight[rx] = -w_adjusted;
        } else {
            self.parent[ry] = rx;
            self.diff_weight[ry] = w_adjusted;
            if self.rank[rx] == self.rank[ry] {
                self.rank[rx] += 1;
            }
        }
    }

    fn same(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }

    // weight(y) - weight(x) を返す（同じ連結成分にあることが前提）
    fn diff(&mut self, x: usize, y: usize) -> i64 {
        self.find(x);
        self.find(y);
        self.diff_weight[y] - self.diff_weight[x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_union_find() {
        let mut uf = WeightedUnionFind::new(5);
        // S[2] - S[0] = 3
        uf.unite(0, 2, 3);
        // S[4] - S[2] = 6
        uf.unite(2, 4, 6);

        assert!(uf.same(0, 4));
        assert_eq!(uf.diff(0, 4), 9); // S[4] - S[0] = 9
    }
}
