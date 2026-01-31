// 069 - Colorful Blocks 2 (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_bq
//
// 制約: |i-j| <= 2 なら異なる色
// つまり連続する3つのブロックは全て異なる色でなければならない
//
// K < 3 のとき:
// - K = 1: N >= 2 なら 0、N = 1 なら 1
// - K = 2: N >= 3 なら 0、N = 1 なら 2、N = 2 なら 2
//
// K >= 3 のとき:
// - 1番目: K通り
// - 2番目: K-1通り (1番目と異なる)
// - 3番目以降: K-2通り (前の2つと異なる)
//
// 答え: K * (K-1) * (K-2)^(N-2)  (N >= 3)

use proconio::input;

const MOD: u64 = 1_000_000_007;

fn main() {
    input! {
        n: u64,
        k: u64,
    }
    println!("{}", solve(n, k));
}

fn solve(n: u64, k: u64) -> u64 {
    if n == 1 {
        return k % MOD;
    }
    if n == 2 {
        return k % MOD * ((k - 1) % MOD) % MOD;
    }

    // N >= 3
    if k < 3 {
        return 0;
    }

    // K * (K-1) * (K-2)^(N-2)
    let k_mod = k % MOD;
    let k1_mod = (k - 1) % MOD;
    let k2_mod = (k - 2) % MOD;

    let power = mod_pow(k2_mod, n - 2);
    k_mod * k1_mod % MOD * power % MOD
}

fn mod_pow(mut base: u64, mut exp: u64) -> u64 {
    let mut result = 1u64;
    base %= MOD;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % MOD;
        }
        exp >>= 1;
        base = base * base % MOD;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve(2, 3), 6);
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve(10, 2), 0);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(2021, 617), 53731843);
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(solve(1, 1), 1);
        assert_eq!(solve(1, 5), 5);
        assert_eq!(solve(3, 3), 6); // 3 * 2 * 1
    }
}
