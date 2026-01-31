// 076 - Cake Cut (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_bx
//
// 円環上の連続部分列で合計が total/10 になるものを探す
// total が 10 で割り切れない場合は No
// しゃくとり法で O(N) で解ける

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; n],
    }
    println!("{}", if solve(n, &a) { "Yes" } else { "No" });
}

fn solve(n: usize, a: &[i64]) -> bool {
    let total: i64 = a.iter().sum();

    // total が 10 で割り切れない場合は No
    if total % 10 != 0 {
        return false;
    }

    let target = total / 10;

    // 累積和を作成（円環なので 2N まで）
    let mut prefix = vec![0i64; 2 * n + 1];
    for i in 0..2 * n {
        prefix[i + 1] = prefix[i] + a[i % n];
    }

    // しゃくとり法または二分探索
    // prefix[j] - prefix[i] = target となる (i, j) を探す (j - i <= n)

    // 二分探索で探す
    for i in 0..n {
        // prefix[j] = prefix[i] + target を探す
        let target_sum = prefix[i] + target;

        // 二分探索
        let pos = prefix[i..i + n + 1].binary_search(&target_sum);

        if let Ok(_) = pos {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        assert!(solve(10, &a));
    }

    #[test]
    fn test_example2() {
        let a = vec![1, 1, 1];
        assert!(!solve(3, &a));
    }

    #[test]
    fn test_example3() {
        let a = vec![1, 18, 1];
        assert!(solve(3, &a));
    }

    #[test]
    fn test_example4() {
        let a = vec![1, 9, 1, 9];
        assert!(!solve(4, &a));
    }
}
