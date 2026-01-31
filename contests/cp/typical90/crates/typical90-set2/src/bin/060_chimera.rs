// 060 - Chimera (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_bh
//
// 山型の部分列の最大長を求める
//
// 1. 左からのLIS: lis_left[i] = A[0..=i] での最長増加部分列長
// 2. 右からのLIS: lis_right[i] = A[i..] での最長減少部分列長
// 3. 答え = max(lis_left[i] + lis_right[i] - 1)

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [usize; n],
    }
    println!("{}", solve(n, &a));
}

fn solve(n: usize, a: &[usize]) -> usize {
    // 左からのLIS
    let lis_left = compute_lis_lengths(a);

    // 右からのLIS（配列を反転してLISを計算）
    let a_rev: Vec<usize> = a.iter().rev().copied().collect();
    let mut lis_right = compute_lis_lengths(&a_rev);
    lis_right.reverse();

    // 各位置を頂点とした山型部分列の長さ
    let mut ans = 0;
    for i in 0..n {
        ans = ans.max(lis_left[i] + lis_right[i] - 1);
    }

    ans
}

// 各位置で終わるLISの長さを計算
fn compute_lis_lengths(a: &[usize]) -> Vec<usize> {
    let n = a.len();
    let mut dp = Vec::new(); // dp[i] = 長さ(i+1)のLISの最小末尾値
    let mut lengths = vec![0; n];

    for (i, &x) in a.iter().enumerate() {
        let pos = dp.partition_point(|&v| v < x);
        if pos == dp.len() {
            dp.push(x);
        } else {
            dp[pos] = x;
        }
        lengths[i] = pos + 1;
    }

    lengths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // B = (1, 2, 3, 2, 1) で長さ5
        assert_eq!(solve(6, &[1, 2, 3, 3, 2, 1]), 5);
    }

    #[test]
    fn test_example2() {
        // 単調増加なので全部使える
        assert_eq!(solve(4, &[1, 2, 3, 4]), 4);
    }

    #[test]
    fn test_example3() {
        // 全部同じ値なので1つしか使えない
        assert_eq!(solve(5, &[3, 3, 3, 3, 3]), 1);
    }
}
