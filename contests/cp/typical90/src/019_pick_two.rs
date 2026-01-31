// 019 - Pick Two (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_s
//
// 問題: 2N個のボールが一列に並んでいる。隣接する2個を取り除く操作をN回行う。
//       取り除くペアの値の差の絶対値の総和を最小化せよ。
//
// 解法: 区間DP
//       dp[l][r] = 区間[l,r)のボールを全て取り除くときのコスト最小値
//       遷移: 両端を取るか、間で分割するか

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; 2 * n],
    }
    println!("{}", solve(&a));
}

#[allow(clippy::needless_range_loop)]
fn solve(a: &[i64]) -> i64 {
    let n = a.len();
    // dp[l][r] = 区間[l,r)を処理するコスト（rはexclusive）
    let mut dp = vec![vec![i64::MAX; n + 1]; n + 1];

    // 長さ0は0
    for i in 0..=n {
        dp[i][i] = 0;
    }

    // 長さ2から順に計算
    for len in (2..=n).step_by(2) {
        for l in 0..=n - len {
            let r = l + len;

            // パターン1: 両端をペアにする
            let cost1 = (a[l] - a[r - 1]).abs() + dp[l + 1][r - 1];
            dp[l][r] = dp[l][r].min(cost1);

            // パターン2: 区間を2つに分割
            for mid in (l + 2..r).step_by(2) {
                if dp[l][mid] != i64::MAX && dp[mid][r] != i64::MAX {
                    dp[l][r] = dp[l][r].min(dp[l][mid] + dp[mid][r]);
                }
            }
        }
    }

    dp[0][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // [1, 2] → ペア(1,2)で |1-2|=1
        assert_eq!(solve(&[1, 2]), 1);
    }

    #[test]
    fn example2() {
        // [1, 3, 2, 4]
        // 隣接ペアのみ削除可能
        // 選択肢1: (3,2)を消して[1,4]、次に(1,4) → |3-2|+|1-4| = 1+3 = 4
        // 選択肢2: (1,3)を消して[2,4]、次に(2,4) → |1-3|+|2-4| = 2+2 = 4
        // 区間DPの答え: 両端ペア |1-4|=3 + 内側|3-2|=1 = 4
        assert_eq!(solve(&[1, 3, 2, 4]), 4);
    }

    #[test]
    fn example3() {
        // [1, 5, 2, 6, 3, 7]
        // 区間DP: 様々な分割を試す
        let a = vec![1, 5, 2, 6, 3, 7];
        let ans = solve(&a);
        // 最適解は複数分割を試して求まる
        assert!(ans > 0); // 正の値であることを確認
    }
}
