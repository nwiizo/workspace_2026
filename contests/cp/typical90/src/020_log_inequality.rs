// 020 - Log Inequality (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_t
//
// 問題: a, b, c が与えられる。log_2(a) < b * log_2(c) を満たすか判定せよ。
//
// 解法: 対数を使わずに指数で比較
//       log_2(a) < b * log_2(c)
//       ⟺ log_2(a) < log_2(c^b)
//       ⟺ a < c^b
//       オーバーフロー注意: c^b が大きくなりすぎる場合は a より大きいと判定

use proconio::input;

fn main() {
    input! {
        a: u64,
        b: u32,
        c: u64,
    }

    if check(a, b, c) {
        println!("Yes");
    } else {
        println!("No");
    }
}

/// a < c^b を判定（オーバーフロー対策付き）
fn check(a: u64, b: u32, c: u64) -> bool {
    // c^b を計算（オーバーフローしたら a より大きい）
    let mut result: u64 = 1;
    for _ in 0..b {
        // オーバーフローチェック
        if result > u64::MAX / c {
            return true; // c^b > a は確実
        }
        result *= c;
        if result > a {
            return true;
        }
    }
    a < result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // log_2(2) = 1 < 2 * log_2(2) = 2 → Yes
        assert!(check(2, 2, 2));
    }

    #[test]
    fn example2() {
        // log_2(4) = 2 < 1 * log_2(4) = 2 → No (等しい)
        assert!(!check(4, 1, 4));
    }

    #[test]
    fn large_power() {
        // 2^64 > 10^18 なので、c^b がオーバーフロー
        // a=10^18, b=100, c=2 → 2^100 >> 10^18 → Yes
        assert!(check(1_000_000_000_000_000_000, 100, 2));
    }

    #[test]
    fn c_is_one() {
        // c=1 → c^b=1、a=2 > 1 → No
        assert!(!check(2, 100, 1));
    }
}
