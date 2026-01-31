// 085 - Multiplication 085 (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_cg
//
// a ≤ b ≤ c, a * b * c = K となる (a, b, c) の組数を数える
// a ≤ K^(1/3), b ≤ K^(1/2) / a なので列挙可能

use proconio::input;

fn main() {
    input! {
        k: u64,
    }
    println!("{}", solve(k));
}

fn solve(k: u64) -> u64 {
    let mut count = 0u64;

    // a: 1 から K^(1/3) まで
    let mut a = 1u64;
    while a * a * a <= k {
        if k % a == 0 {
            let ka = k / a; // b * c = ka

            // b: a から ka^(1/2) まで (b ≤ c より b^2 ≤ ka)
            let mut b = a;
            while b * b <= ka {
                if ka % b == 0 {
                    let c = ka / b;
                    // a ≤ b ≤ c を確認
                    if b <= c {
                        count += 1;
                    }
                }
                b += 1;
            }
        }
        a += 1;
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // (1,1,42), (1,2,21), (1,3,14), (1,6,7), (2,3,7)
        assert_eq!(solve(42), 5);
    }

    #[test]
    fn test_example2() {
        // (1,1,7)
        assert_eq!(solve(7), 1);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(192), 16);
    }
}
