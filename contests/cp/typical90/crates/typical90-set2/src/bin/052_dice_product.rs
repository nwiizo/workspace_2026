// 052 - Dice Product (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_az
//
// ============================================================
// 積の分配法則
// ============================================================
//
// 全パターンの積の総和を求める問題
//
// 【重要な観察】
// Σ (R_1 × R_2 × ... × R_N) = (Σ R_1) × (Σ R_2) × ... × (Σ R_N)
//
// 例: 2つのサイコロ [1,2] と [3,4] の場合
// 全パターン: 1×3 + 1×4 + 2×3 + 2×4 = 3 + 4 + 6 + 8 = 21
// 分配法則:   (1+2) × (3+4) = 3 × 7 = 21
//
// したがって、各サイコロの6面の合計を求めて掛け合わせるだけ！
//
// 計算量: O(N)
//
// ============================================================

use proconio::input;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        n: usize,
        dice: [[i64; 6]; n],
    }
    println!("{}", solve(&dice));
}

fn solve(dice: &[Vec<i64>]) -> i64 {
    let mut result = 1i64;

    for die in dice {
        // このサイコロの6面の合計
        let sum: i64 = die.iter().sum();
        result = (result * sum) % MOD;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // Die 1: 1+2+3+5+7+11 = 29
        // Die 2: 4+6+8+9+10+12 = 49
        // 29 × 49 = 1421
        let dice = vec![vec![1, 2, 3, 5, 7, 11], vec![4, 6, 8, 9, 10, 12]];
        assert_eq!(solve(&dice), 1421);
    }

    #[test]
    fn test_example2() {
        // 11+13+17+19+23+29 = 112
        let dice = vec![vec![11, 13, 17, 19, 23, 29]];
        assert_eq!(solve(&dice), 112);
    }
}
