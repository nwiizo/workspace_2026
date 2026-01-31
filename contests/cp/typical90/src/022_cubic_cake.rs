// 022 - Cubic Cake (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_v
//
// 問題: A×B×Cの直方体を立方体に切り分ける。最小カット数を求めよ。
//
// 解法: GCDで最大の立方体サイズを求める
//       1辺の長さ = gcd(A, B, C)
//       各辺のカット数 = (辺の長さ / gcd) - 1

use proconio::input;

fn main() {
    input! {
        a: i64,
        b: i64,
        c: i64,
    }
    println!("{}", solve(a, b, c));
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn solve(a: i64, b: i64, c: i64) -> i64 {
    let g = gcd(gcd(a, b), c);

    // 各辺のカット数
    let cuts_a = a / g - 1;
    let cuts_b = b / g - 1;
    let cuts_c = c / g - 1;

    cuts_a + cuts_b + cuts_c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 2×2×2 → gcd=2, カット数=0
        assert_eq!(solve(2, 2, 2), 0);
    }

    #[test]
    fn example2() {
        // 2×6×4 → gcd=2, カット数=(1-1)+(3-1)+(2-1)=3
        assert_eq!(solve(2, 6, 4), 3);
    }

    #[test]
    fn example3() {
        // 1×1×1 → gcd=1, カット数=0
        assert_eq!(solve(1, 1, 1), 0);
    }

    #[test]
    fn large_gcd() {
        // 12×18×24 → gcd=6, カット数=(2-1)+(3-1)+(4-1)=1+2+3=6
        assert_eq!(solve(12, 18, 24), 6);
    }
}
