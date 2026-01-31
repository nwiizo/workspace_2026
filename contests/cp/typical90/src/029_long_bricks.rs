// 029 - Long Bricks (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_ac
//
// 問題: 1×Wのグリッドにブロックを積む。各ブロックは区間[L,R]を覆う。
//       各ブロックについて、置いた後の最大高さを出力。
//
// 解法: 遅延セグメント木（区間max取得、区間更新）
//       または座標圧縮 + セグ木

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
    // 座標圧縮
    let mut coords: Vec<usize> = bricks.iter().flat_map(|&(l, r)| [l, r + 1]).collect();
    coords.sort_unstable();
    coords.dedup();

    let compress = |x: usize| coords.binary_search(&x).unwrap();

    let m = coords.len();
    let mut seg = SegTree::new(m);

    for &(l, r) in bricks {
        let cl = compress(l);
        let cr = compress(r + 1);

        // 区間の最大高さを取得
        let max_h = seg.query(cl, cr);
        let new_h = max_h + 1;

        println!("{}", new_h);

        // 区間を更新
        seg.update_range(cl, cr, new_h);
    }
}

/// 区間max更新・区間max取得のセグメント木
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
            lazy: vec![-1; 2 * size],
        }
    }

    fn push(&mut self, k: usize) {
        if self.lazy[k] >= 0 {
            if k < self.size {
                self.lazy[2 * k] = self.lazy[k];
                self.lazy[2 * k + 1] = self.lazy[k];
            }
            self.data[k] = self.lazy[k];
            self.lazy[k] = -1;
        }
    }

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
        if r <= node_l || node_r <= l {
            return;
        }
        if l <= node_l && node_r <= r {
            self.lazy[k] = val;
            self.push(k);
            return;
        }
        let mid = (node_l + node_r) / 2;
        self.update_range_inner(l, r, val, 2 * k, node_l, mid);
        self.update_range_inner(l, r, val, 2 * k + 1, mid, node_r);
        self.data[k] = self.data[2 * k].max(self.data[2 * k + 1]);
    }

    fn query(&mut self, l: usize, r: usize) -> i64 {
        self.query_inner(l, r, 1, 0, self.size)
    }

    fn query_inner(&mut self, l: usize, r: usize, k: usize, node_l: usize, node_r: usize) -> i64 {
        self.push(k);
        if r <= node_l || node_r <= l {
            return 0;
        }
        if l <= node_l && node_r <= r {
            return self.data[k];
        }
        let mid = (node_l + node_r) / 2;
        let left = self.query_inner(l, r, 2 * k, node_l, mid);
        let right = self.query_inner(l, r, 2 * k + 1, mid, node_r);
        left.max(right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segtree() {
        let mut seg = SegTree::new(8);

        seg.update_range(1, 4, 5);
        assert_eq!(seg.query(0, 8), 5);
        assert_eq!(seg.query(1, 2), 5);
        assert_eq!(seg.query(4, 6), 0);

        seg.update_range(3, 6, 3);
        assert_eq!(seg.query(0, 8), 5);
        assert_eq!(seg.query(4, 5), 3);
    }
}
