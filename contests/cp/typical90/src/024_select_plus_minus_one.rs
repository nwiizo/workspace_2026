// 024 - Select +/- One (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_x
//
// 問題: 数列Aを数列Bに変える。各操作で1つの要素を+1か-1できる。
//       ちょうどK回の操作で変換可能か判定せよ。
//
// 解法: 差の絶対値の和を計算
//       diff = Σ|A_i - B_i|
//       可能条件: diff ≤ K かつ (K - diff) が偶数

use proconio::input;

fn main() {
    input! {
        n: usize,
        k: i64,
        a: [i64; n],
        b: [i64; n],
    }

    if solve(&a, &b, k) {
        println!("Yes");
    } else {
        println!("No");
    }
}

fn solve(a: &[i64], b: &[i64], k: i64) -> bool {
    let diff: i64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();

    // 必要な操作回数以上か、余りが偶数か
    diff <= k && (k - diff) % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // A=[3,4,1], B=[5,2,3], diff=|3-5|+|4-2|+|1-3|=2+2+2=6
        // K=6, 6≤6 かつ (6-6)%2=0 → Yes
        assert!(solve(&[3, 4, 1], &[5, 2, 3], 6));
    }

    #[test]
    fn example2() {
        // A=[1], B=[2], diff=1
        // K=2, 1≤2 かつ (2-1)%2=1 → No
        assert!(!solve(&[1], &[2], 2));
    }

    #[test]
    fn example3() {
        // A=[1], B=[2], diff=1
        // K=3, 1≤3 かつ (3-1)%2=0 → Yes
        assert!(solve(&[1], &[2], 3));
    }

    #[test]
    fn not_enough() {
        // diff > K
        assert!(!solve(&[1], &[10], 5));
    }
}
