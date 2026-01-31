//! 030 - K Factors (★5)
//!
//! エラトステネスの篩の応用
//!
//! N以下の整数のうち、素因数の種類数がちょうどK個のものを数える。
//! 篩を使って各数の素因数の種類数を前計算する。

use proconio::input;

fn main() {
    input! {
        n: usize,
        k: usize,
    }
    println!("{}", solve(n, k));
}

fn solve(n: usize, k: usize) -> usize {
    // factor_count[i] = i の素因数の種類数
    let mut factor_count = vec![0usize; n + 1];

    // エラトステネスの篩の要領で、各素数pの倍数に+1
    for p in 2..=n {
        if factor_count[p] == 0 {
            // p は素数
            for multiple in (p..=n).step_by(p) {
                factor_count[multiple] += 1;
            }
        }
    }

    // 素因数の種類数がちょうどKの数をカウント
    factor_count[2..=n].iter().filter(|&&c| c == k).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // N=10, K=2: 素因数2種類の数
        // 6 = 2*3, 10 = 2*5 → 2個
        assert_eq!(solve(10, 2), 2);
    }

    #[test]
    fn example2() {
        // N=20, K=2
        // 6,10,12,14,15,18,20 → 7個
        assert_eq!(solve(20, 2), 7);
    }

    #[test]
    fn k_equals_1() {
        // 素因数1種類 = 素数のべき乗
        // 2,3,4,5,7,8,9 (2,3,2^2,5,7,2^3,3^2) → 7個
        assert_eq!(solve(10, 1), 7);
    }

    #[test]
    fn large_k() {
        // 大きなKだと該当なし
        assert_eq!(solve(10, 5), 0);
    }

    #[test]
    fn k_equals_3() {
        // 30 = 2*3*5 が最小の素因数3種類
        assert_eq!(solve(29, 3), 0);
        assert_eq!(solve(30, 3), 1);
    }
}
