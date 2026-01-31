// 016 - Minimum Coins (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_p
//
// 問題: A円、B円、C円の硬貨がある。合計N円をちょうど支払うとき、
//       硬貨の枚数の最小値を求めよ。
//
// 解法: 全探索
//       N ≤ 10^6, A,B,C ≥ 1 なので各硬貨は最大10^6枚
//       2つの硬貨の枚数を全探索し、残りを計算 → O(N²/A/B)
//       ただし N/min(A,B,C) ≤ 9999 の制約があるので O(10^8) 以内

use proconio::input;

fn main() {
    input! {
        n: i64,
        a: i64,
        b: i64,
        c: i64,
    }
    println!("{}", solve(n, a, b, c));
}

fn solve(n: i64, a: i64, b: i64, c: i64) -> i64 {
    let max_coins = 9999; // 制約より
    let mut ans = i64::MAX;

    for i in 0..=max_coins {
        if a * i > n {
            break;
        }
        for j in 0..=max_coins {
            let used = a * i + b * j;
            if used > n {
                break;
            }
            let remaining = n - used;
            if remaining % c == 0 {
                let k = remaining / c;
                if k <= max_coins {
                    ans = ans.min(i + j + k);
                }
            }
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 15円を1円・5円・10円で支払う → 10+5 = 2枚
        assert_eq!(solve(15, 1, 5, 10), 2);
    }

    #[test]
    fn example2() {
        // 20円を1円・5円・10円で支払う → 10+10 = 2枚
        assert_eq!(solve(20, 1, 5, 10), 2);
    }

    #[test]
    fn all_same() {
        // 30円を10円×3で支払う
        assert_eq!(solve(30, 10, 10, 10), 3);
    }
}
