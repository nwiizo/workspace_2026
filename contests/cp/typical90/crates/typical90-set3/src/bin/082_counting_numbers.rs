// 082 - Counting Numbers (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_cd
//
// L から R まで、x を x 回書く → 桁数 × x の総和
// 桁数ごとに分けて計算:
// d 桁の数: 10^(d-1) から 10^d - 1 まで
// その範囲で [L, R] との共通部分を計算
//
// Σ x for x in [a, b] = (a + b) * (b - a + 1) / 2

use proconio::input;

const MOD: u64 = 1_000_000_007;

fn main() {
    input! {
        l: u64,
        r: u64,
    }
    println!("{}", solve(l, r));
}

fn solve(l: u64, r: u64) -> u64 {
    let mut ans = 0u64;

    // 桁数ごとに計算
    for d in 1..=19 {
        // d 桁の範囲: [10^(d-1), 10^d - 1]
        let lo = if d == 1 { 1 } else { 10u64.pow(d - 1) };
        let hi = 10u64.pow(d) - 1;

        // [L, R] との共通部分
        let a = lo.max(l);
        let b = hi.min(r);

        if a > b {
            continue;
        }

        // d * Σ x for x in [a, b]
        // = d * (a + b) * (b - a + 1) / 2
        //
        // MOD 演算に注意

        let d_mod = d as u64 % MOD;
        let sum = sum_range(a, b);
        ans = (ans + d_mod * sum) % MOD;
    }

    ans
}

// Σ x for x in [a, b] を mod MOD で計算
fn sum_range(a: u64, b: u64) -> u64 {
    // (a + b) * (b - a + 1) / 2
    // = ((a mod MOD) + (b mod MOD)) * ((b - a + 1) mod MOD) / 2

    let a_mod = a % MOD;
    let b_mod = b % MOD;
    let sum_mod = (a_mod + b_mod) % MOD;
    let cnt = (b - a + 1) % MOD;

    // sum_mod * cnt は偶数になるはず
    // 2 の逆元を使う
    let inv2 = mod_inv(2);
    sum_mod * cnt % MOD * inv2 % MOD
}

fn mod_inv(a: u64) -> u64 {
    mod_pow(a, MOD - 2)
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
        // 3*1 + 4*1 + 5*1 = 12 (各数は1桁)
        assert_eq!(solve(3, 5), 12);
    }

    #[test]
    fn test_example2() {
        // 98*2 + 99*2 + 100*3 = 196 + 198 + 300 = 694
        assert_eq!(solve(98, 100), 694);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(1001, 869120), 59367733);
    }

    #[test]
    fn test_example4() {
        assert_eq!(solve(381453331666495446, 746254773042091083), 584127830);
    }
}
