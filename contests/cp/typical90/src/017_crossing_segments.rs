// 017 - Crossing Segments (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_q
//
// 問題: 円周上のN点と、M本の弦がある。交差する弦のペア数を求めよ。
//
// 解法: Lでソート + BIT
//       弦(L1,R1)と(L2,R2)が交差 ⟺ L1 < L2 < R1 < R2
//       Lでソートして処理、各弦のRをBITで管理
//       弦(L,R)を処理するとき、(L, R)の範囲にあるRの数が交差数

use proconio::input;
use proconio::marker::Usize1;

fn main() {
    input! {
        n: usize,
        m: usize,
        segments: [(Usize1, Usize1); m],
    }
    println!("{}", solve(n, &segments));
}

fn solve(n: usize, segments: &[(usize, usize)]) -> i64 {
    // L < R に正規化し、Lでソート
    let mut segs: Vec<(usize, usize)> = segments
        .iter()
        .map(|&(l, r)| if l < r { (l, r) } else { (r, l) })
        .collect();
    segs.sort_by_key(|&(l, _)| l);

    let mut bit = Bit::new(n);
    let mut ans = 0i64;

    for (l, r) in segs {
        // (L, R) の開区間にあるR'の数 = 交差する弦の数
        // L1 < L < R1 < R となる(L1,R1)の数
        if l + 1 < r {
            ans += bit.sum(l + 1, r - 1);
        }
        // この弦のRを追加
        bit.add(r, 1);
    }

    ans
}

#[allow(clippy::upper_case_acronyms)]
struct Bit {
    data: Vec<i64>,
}

impl Bit {
    fn new(n: usize) -> Self {
        Self {
            data: vec![0; n + 1],
        }
    }

    fn add(&mut self, mut i: usize, x: i64) {
        i += 1;
        while i < self.data.len() {
            self.data[i] += x;
            i += i & i.wrapping_neg();
        }
    }

    fn prefix_sum(&self, mut i: usize) -> i64 {
        let mut s = 0;
        while i > 0 {
            s += self.data[i];
            i -= i & i.wrapping_neg();
        }
        s
    }

    fn sum(&self, l: usize, r: usize) -> i64 {
        if l > r {
            return 0;
        }
        self.prefix_sum(r + 1) - self.prefix_sum(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit() {
        let mut bit = Bit::new(10);
        bit.add(2, 1);
        bit.add(5, 1);
        bit.add(7, 1);

        assert_eq!(bit.sum(0, 9), 3);
        assert_eq!(bit.sum(3, 6), 1);
        assert_eq!(bit.sum(2, 5), 2);
    }

    #[test]
    fn example_crossing() {
        // 円周上に0,1,2,3,4,5の6点
        // 弦: (0,3), (1,4), (2,5)
        // 交差条件: L1 < L2 < R1 < R2
        //
        // (0,3)と(1,4): 0<1<3<4 ✓
        // (0,3)と(2,5): 0<2<3<5 ✓
        // (1,4)と(2,5): 1<2<4<5 ✓
        // 答え: 3
        assert_eq!(solve(6, &[(0, 3), (1, 4), (2, 5)]), 3);
    }

    #[test]
    fn no_crossing() {
        // (0,1), (2,3) は交差しない: 0<2 だが 1<2 なので条件不成立
        assert_eq!(solve(4, &[(0, 1), (2, 3)]), 0);
    }

    #[test]
    fn nested_no_crossing() {
        // (0,3), (1,2) は交差しない: 0<1<2<3 (含まれている)
        assert_eq!(solve(4, &[(0, 3), (1, 2)]), 0);
    }
}
