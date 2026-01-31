//! 037 - Don't Leave the Spice (★5)
//!
//! スライド最大値を使ったDP
//!
//! W グラムの容器に N 種類の香辛料を入れる。
//! 香辛料 i は L[i] から R[i] グラムの範囲で入れられ、価値は V[i]。
//! 容器をちょうど W グラムにしたときの価値の最大値を求める。
//!
//! dp[w] = ちょうど w グラムにしたときの最大価値
//! 遷移: dp[w] = max(dp[w-R[i]..w-L[i]]) + V[i]
//!
//! スライド最大値（単調デック）で O(NW) に高速化。

use proconio::input;
use std::collections::VecDeque;

fn main() {
    input! {
        w: usize,
        n: usize,
        spices: [(usize, usize, i64); n], // (L, R, V)
    }
    let result = solve(w, &spices);
    if result == i64::MIN {
        println!("-1");
    } else {
        println!("{}", result);
    }
}

#[allow(clippy::needless_range_loop)]
fn solve(w: usize, spices: &[(usize, usize, i64)]) -> i64 {
    const NEG_INF: i64 = i64::MIN / 2;
    let mut dp = vec![NEG_INF; w + 1];
    dp[0] = 0;

    for &(l, r, v) in spices {
        // 新しいDPテーブル
        let mut new_dp = dp.clone();

        // スライド最大値用のデック: (index, value)
        let mut deque: VecDeque<(usize, i64)> = VecDeque::new();

        for weight in l..=w {
            // weight グラムにするには、前の状態が weight-r から weight-l の範囲
            // つまり dp[weight-r..=weight-l] の最大値 + v

            // 新しい要素を追加 (weight - l の位置)
            let add_idx = weight - l;
            if dp[add_idx] != NEG_INF {
                while !deque.is_empty() && deque.back().unwrap().1 <= dp[add_idx] {
                    deque.pop_back();
                }
                deque.push_back((add_idx, dp[add_idx]));
            }

            // 範囲外の要素を削除
            if weight > r {
                let remove_idx = weight - r - 1;
                while !deque.is_empty() && deque.front().unwrap().0 <= remove_idx {
                    deque.pop_front();
                }
            }

            // 最大値を取得
            if !deque.is_empty() {
                let max_val = deque.front().unwrap().1;
                new_dp[weight] = new_dp[weight].max(max_val + v);
            }
        }

        dp = new_dp;
    }

    if dp[w] == NEG_INF { i64::MIN } else { dp[w] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // W=10, 香辛料: (2,5,3), (3,6,2), (1,4,5)
        // 2+3+5=10グラム: 3+2+5=10価値
        // or 5+4+1=10: 不可
        let spices = vec![(2, 5, 3), (3, 6, 2), (1, 4, 5)];
        assert_eq!(solve(10, &spices), 10);
    }

    #[test]
    fn example2() {
        // 不可能な場合
        let spices = vec![(5, 5, 100)];
        assert_eq!(solve(3, &spices), i64::MIN);
    }

    #[test]
    fn single_spice() {
        let spices = vec![(5, 10, 100)];
        assert_eq!(solve(7, &spices), 100);
        assert_eq!(solve(5, &spices), 100);
        assert_eq!(solve(10, &spices), 100);
        assert_eq!(solve(4, &spices), i64::MIN);
    }

    #[test]
    fn exact_fit() {
        let spices = vec![(3, 3, 10), (2, 2, 20)];
        assert_eq!(solve(5, &spices), 30);
    }
}
