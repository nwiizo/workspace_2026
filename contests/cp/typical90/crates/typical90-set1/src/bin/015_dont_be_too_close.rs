// 015 - Don't be too close (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_o
//
// 問題: 1からNまでの整数からK個選ぶとき、隣り合う2数の差が2以上となる
//       選び方の数を各K (1≤K≤N) について求めよ。
//
// 解法: 組み合わせの変換
//       K個選んで差が2以上 ⟺ N-(K-1)個からK個選ぶ（重複なし）
//       答え = C(N-K+1, K)

use proconio::input;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! { n: usize }

    // 階乗とその逆元を前計算
    let (fact, inv_fact) = precompute_factorial(n + 1);

    for k in 1..=n {
        if n + 1 < 2 * k {
            // N - K + 1 < K の場合は選べない
            println!("0");
        } else {
            let ans = comb(n - k + 1, k, &fact, &inv_fact);
            println!("{}", ans);
        }
    }
}

fn precompute_factorial(n: usize) -> (Vec<i64>, Vec<i64>) {
    let mut fact = vec![1i64; n + 1];
    for i in 1..=n {
        fact[i] = fact[i - 1] * i as i64 % MOD;
    }

    let mut inv_fact = vec![1i64; n + 1];
    inv_fact[n] = mod_pow(fact[n], MOD - 2);
    for i in (0..n).rev() {
        inv_fact[i] = inv_fact[i + 1] * (i + 1) as i64 % MOD;
    }

    (fact, inv_fact)
}

fn comb(n: usize, r: usize, fact: &[i64], inv_fact: &[i64]) -> i64 {
    if n < r {
        return 0;
    }
    fact[n] * inv_fact[r] % MOD * inv_fact[n - r] % MOD
}

fn mod_pow(mut base: i64, mut exp: i64) -> i64 {
    let mut result = 1i64;
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
    fn test_factorial() {
        let (fact, _) = precompute_factorial(5);
        assert_eq!(fact[0], 1);
        assert_eq!(fact[1], 1);
        assert_eq!(fact[5], 120);
    }

    #[test]
    fn test_comb() {
        let (fact, inv_fact) = precompute_factorial(10);
        assert_eq!(comb(5, 2, &fact, &inv_fact), 10);
        assert_eq!(comb(10, 3, &fact, &inv_fact), 120);
    }

    #[test]
    fn n_equals_3() {
        // N=3, K=1: C(3,1)=3 (選べる: 1,2,3)
        // N=3, K=2: C(2,2)=1 (選べる: {1,3})
        // N=3, K=3: C(1,3)=0 (選べない)
        let (fact, inv_fact) = precompute_factorial(4);
        assert_eq!(comb(3, 1, &fact, &inv_fact), 3);
        assert_eq!(comb(2, 2, &fact, &inv_fact), 1);
        assert_eq!(comb(1, 3, &fact, &inv_fact), 0);
    }
}
