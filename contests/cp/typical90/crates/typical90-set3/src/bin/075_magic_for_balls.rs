// 075 - Magic For Balls (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_bw
//
// N を素因数分解して、素因数の個数を k とする（重複込み）
// 1回の魔法で全てのボールが同時に分裂
// 最終的に k 個の素数ボールになる
//
// 最適戦略: 毎回できるだけ均等に分割
// k 個のボールにするのに必要な回数 = ceil(log2(k))
//
// k = 1 (N が素数) → 0 回
// k = 2 → 1 回
// k = 3, 4 → 2 回
// k = 5, 6, 7, 8 → 3 回
// ...

use proconio::input;

fn main() {
    input! {
        n: u64,
    }
    println!("{}", solve(n));
}

fn solve(n: u64) -> u32 {
    // 素因数の個数を数える（重複込み）
    let mut k = 0u32;
    let mut m = n;

    // 2で割れるだけ割る
    while m % 2 == 0 {
        k += 1;
        m /= 2;
    }

    // 3以上の奇数で割る
    let mut i = 3u64;
    while i * i <= m {
        while m % i == 0 {
            k += 1;
            m /= i;
        }
        i += 2;
    }

    // 残りが1より大きければ素数
    if m > 1 {
        k += 1;
    }

    // k 個のボールにするのに必要な魔法の回数
    if k <= 1 {
        0
    } else {
        // ceil(log2(k)) = 32 - (k-1).leading_zeros()
        32 - (k - 1).leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // 42 = 2 * 3 * 7 → k = 3 → ceil(log2(3)) = 2
        assert_eq!(solve(42), 2);
    }

    #[test]
    fn test_example2() {
        // 48 = 2^4 * 3 → k = 5 → ceil(log2(5)) = 3
        assert_eq!(solve(48), 3);
    }

    #[test]
    fn test_example3() {
        // 54 = 2 * 3^3 → k = 4 → ceil(log2(4)) = 2
        assert_eq!(solve(54), 2);
    }

    #[test]
    fn test_example4() {
        // 53 は素数 → k = 1 → 0 回
        assert_eq!(solve(53), 0);
    }
}
