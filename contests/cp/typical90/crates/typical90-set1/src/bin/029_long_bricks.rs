// 029 - Long Bricks (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_ac
//
// ============================================================================
// 【物語で理解する問題】
// ============================================================================
//
// 建築現場でブロックを積み上げています。
//
// 幅 W マスの土地があります。
// N 個のブロックを順番に置いていきます。
//
// 各ブロック i は区間 [L_i, R_i] を覆います。
// ブロックは「その区間で一番高い場所」の上に積まれます。
//
// 例:
//   土地: 幅5マス、最初は高さ0
//
//   ブロック1: 区間[1,3]
//   → 高さ0の上に置く → 高さ1になる
//
//   ブロック2: 区間[2,4]
//   → 区間[2,4]の最大高さは1（位置2,3）
//   → 高さ1の上に置く → 高さ2になる
//
//   出力: 1, 2
//
// ============================================================================
// 【解法：遅延セグメント木】
// ============================================================================
//
// 【なぜセグメント木？】
//
// 各ブロックを置くとき:
// 1. 区間 [L, R] の最大高さを求める（区間最大クエリ）
// 2. 区間 [L, R] を新しい高さに更新する（区間更新）
//
// この2つの操作を効率的に行うには「遅延伝播セグメント木」が必要。
//
// 【座標圧縮】
//
// W は最大 10^9 だが、クエリに現れる座標は最大 2N 個。
// 座標圧縮で O(N) のセグメント木サイズに落とせる。
//
// 【遅延伝播（Lazy Propagation）とは？】
//
// 区間更新を効率化するテクニック。
// 更新を「必要になるまで」子ノードに伝播させない。
//
// 【アルゴリズム】
//
// 1. 全座標を圧縮
// 2. 各ブロックについて:
//    a. 区間の最大高さを取得
//    b. 新しい高さ = 最大高さ + 1 を出力
//    c. 区間を新しい高さに更新
//
// ============================================================================
// 【計算量】
// ============================================================================
//
// - 座標圧縮: O(N log N)
// - 各ブロックの処理: O(log N)
// - 合計: O(N log N)
//
// ============================================================================

use proconio::input;

fn main() {
    input! {
        w: usize,
        n: usize,
        bricks: [(usize, usize); n], // (L, R) 1-indexed
    }
    solve(w, &bricks);
}

fn solve(_w: usize, bricks: &[(usize, usize)]) {
    // -------------------------------------------------------------------------
    // 【座標圧縮】
    //
    // 使用する座標だけを列挙してインデックス化
    // R+1 も含めて半開区間 [L, R+1) で扱う
    // -------------------------------------------------------------------------
    let mut coords: Vec<usize> = bricks.iter().flat_map(|&(l, r)| [l, r + 1]).collect();
    coords.sort_unstable();
    coords.dedup();

    let compress = |x: usize| coords.binary_search(&x).unwrap();

    let m = coords.len();
    let mut seg = SegTree::new(m);

    for &(l, r) in bricks {
        let cl = compress(l);
        let cr = compress(r + 1);

        // ---------------------------------------------------------------------
        // 【区間の最大高さを取得】
        // ---------------------------------------------------------------------
        let max_h = seg.query(cl, cr);

        // ---------------------------------------------------------------------
        // 【新しい高さを出力】
        // ---------------------------------------------------------------------
        let new_h = max_h + 1;
        println!("{}", new_h);

        // ---------------------------------------------------------------------
        // 【区間を新しい高さに更新】
        // ---------------------------------------------------------------------
        seg.update_range(cl, cr, new_h);
    }
}

// =============================================================================
// 【遅延セグメント木】
//
// - 区間最大値取得
// - 区間一括更新（区間内の全要素を同じ値に置き換え）
// =============================================================================
struct SegTree {
    size: usize,
    data: Vec<i64>,
    lazy: Vec<i64>,
}

impl SegTree {
    fn new(n: usize) -> Self {
        let size = n.next_power_of_two();
        Self {
            size,
            data: vec![0; 2 * size],
            lazy: vec![-1; 2 * size], // -1 は「更新なし」を意味
        }
    }

    /// 遅延値を子ノードに伝播
    fn push(&mut self, k: usize) {
        if self.lazy[k] >= 0 {
            if k < self.size {
                // 子ノードに伝播
                self.lazy[2 * k] = self.lazy[k];
                self.lazy[2 * k + 1] = self.lazy[k];
            }
            // 自ノードを更新
            self.data[k] = self.lazy[k];
            self.lazy[k] = -1;
        }
    }

    /// 区間 [l, r) を val に更新
    fn update_range(&mut self, l: usize, r: usize, val: i64) {
        self.update_range_inner(l, r, val, 1, 0, self.size);
    }

    fn update_range_inner(
        &mut self,
        l: usize,
        r: usize,
        val: i64,
        k: usize,
        node_l: usize,
        node_r: usize,
    ) {
        self.push(k);

        // 範囲外
        if r <= node_l || node_r <= l {
            return;
        }

        // 完全に含まれる
        if l <= node_l && node_r <= r {
            self.lazy[k] = val;
            self.push(k);
            return;
        }

        // 部分的に重なる → 子ノードに分割
        let mid = (node_l + node_r) / 2;
        self.update_range_inner(l, r, val, 2 * k, node_l, mid);
        self.update_range_inner(l, r, val, 2 * k + 1, mid, node_r);
        self.data[k] = self.data[2 * k].max(self.data[2 * k + 1]);
    }

    /// 区間 [l, r) の最大値を取得
    fn query(&mut self, l: usize, r: usize) -> i64 {
        self.query_inner(l, r, 1, 0, self.size)
    }

    fn query_inner(&mut self, l: usize, r: usize, k: usize, node_l: usize, node_r: usize) -> i64 {
        self.push(k);

        // 範囲外
        if r <= node_l || node_r <= l {
            return 0;
        }

        // 完全に含まれる
        if l <= node_l && node_r <= r {
            return self.data[k];
        }

        // 部分的に重なる
        let mid = (node_l + node_r) / 2;
        let left = self.query_inner(l, r, 2 * k, node_l, mid);
        let right = self.query_inner(l, r, 2 * k + 1, mid, node_r);
        left.max(right)
    }
}

// =============================================================================
// 【テスト】
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segtree_basic() {
        let mut seg = SegTree::new(8);

        // 区間 [1, 4) を高さ5に更新
        seg.update_range(1, 4, 5);
        assert_eq!(seg.query(0, 8), 5); // 全体の最大
        assert_eq!(seg.query(1, 2), 5); // 区間内
        assert_eq!(seg.query(4, 6), 0); // 区間外

        // 区間 [3, 6) を高さ3に更新（5より低い）
        seg.update_range(3, 6, 3);
        assert_eq!(seg.query(0, 8), 5); // 最大は5のまま
        assert_eq!(seg.query(4, 5), 3); // 新しく更新された部分
    }

    #[test]
    fn test_segtree_overwrite() {
        let mut seg = SegTree::new(8);

        // 区間を上書き
        seg.update_range(0, 4, 10);
        seg.update_range(0, 4, 5); // 低い値で上書き
        assert_eq!(seg.query(0, 4), 5);
    }

    #[test]
    fn test_example() {
        // 手動でシミュレーション
        // ブロック1: [1, 3] → 高さ1
        // ブロック2: [2, 4] → 区間[2,4]の最大は1 → 高さ2
        // ブロック3: [1, 2] → 区間[1,2]の最大は2 → 高さ3

        let mut seg = SegTree::new(8);

        // ブロック1
        let max1 = seg.query(1, 4); // [1, 3] → [1, 4)
        assert_eq!(max1, 0);
        seg.update_range(1, 4, 1);

        // ブロック2
        let max2 = seg.query(2, 5); // [2, 4] → [2, 5)
        assert_eq!(max2, 1);
        seg.update_range(2, 5, 2);

        // ブロック3
        let max3 = seg.query(1, 3); // [1, 2] → [1, 3)
        assert_eq!(max3, 2);
    }
}
