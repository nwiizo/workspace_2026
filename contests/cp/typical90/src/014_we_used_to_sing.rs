// 014 - We Used to Sing a Song Together (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_n
//
// 問題: N人の子供とN軒の家がある。各子供の位置A_i、各家の位置B_i。
//       子供と家を1対1で対応させ、移動距離の総和を最小化せよ。
//
// 解法: ソートして対応させる
//       ソート後に同じインデックス同士を対応させるのが最適

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; n],
        b: [i64; n],
    }
    println!("{}", solve(&a, &b));
}

fn solve(a: &[i64], b: &[i64]) -> i64 {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_unstable();
    b.sort_unstable();

    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 子供: [2, 3, 4] → ソート済み
        // 家: [1, 2, 6] → ソート済み
        // 対応: 2-1, 3-2, 4-6 → |1| + |1| + |2| = 4
        assert_eq!(solve(&[2, 3, 4], &[1, 2, 6]), 4);
    }

    #[test]
    fn example2() {
        // 子供: [1, 5]
        // 家: [3, 4]
        // 対応: 1-3, 5-4 → |2| + |1| = 3
        assert_eq!(solve(&[1, 5], &[3, 4]), 3);
    }

    #[test]
    fn same_positions() {
        assert_eq!(solve(&[1, 2, 3], &[1, 2, 3]), 0);
    }
}
