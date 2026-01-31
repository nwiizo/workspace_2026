//! 023 - Avoid War (★7)
//!
//! グリッドDP（ビットマスク）
//!
//! H×W のグリッドにキングを置く。
//! キング同士は隣接してはいけない（8方向）。
//! 一部のマスには障害物がありキングを置けない。
//! キングの配置方法を数える。
//!
//! dp[i][mask] = i 行目まで見て、i 行目の配置が mask のときの場合の数
//! mask は W ビットで、1 ならキングあり。
//!
//! 遷移条件:
//! 1. mask 内で隣接するビットがない
//! 2. 障害物のあるマスにビットが立っていない
//! 3. 前の行の mask との間で対角・縦方向に隣接がない

use proconio::input;
use proconio::marker::Chars;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        h: usize,
        w: usize,
        grid: [Chars; h],
    }
    println!("{}", solve(h, w, &grid));
}

fn solve(h: usize, w: usize, grid: &[Vec<char>]) -> i64 {
    // 障害物マスク: obstacle[i] の j ビット目が 1 なら障害物
    let obstacle: Vec<u32> = grid
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|&(_, &c)| c == '#')
                .fold(0u32, |acc, (j, _)| acc | (1 << j))
        })
        .collect();

    // 有効な行マスク（隣接ビットなし）を前計算
    let valid_row_masks: Vec<u32> = (0..(1 << w))
        .filter(|&mask| (mask & (mask >> 1)) == 0)
        .collect();

    // dp[mask] = 現在の行で mask を選んだときの場合の数
    let mut dp = vec![0i64; 1 << w];
    dp[0] = 1; // 初期状態（0行目の前）

    for &obs in obstacle.iter().take(h) {
        let mut new_dp = vec![0i64; 1 << w];

        for &curr_mask in &valid_row_masks {
            // 障害物チェック
            if (curr_mask & obs) != 0 {
                continue;
            }

            for &prev_mask in &valid_row_masks {
                if dp[prev_mask as usize] == 0 {
                    continue;
                }

                // 縦・斜め隣接チェック
                // prev_mask と curr_mask が縦に隣接: prev_mask & curr_mask != 0
                // prev_mask と curr_mask が斜めに隣接:
                //   (prev_mask << 1) & curr_mask != 0 または
                //   (prev_mask >> 1) & curr_mask != 0
                if (prev_mask & curr_mask) != 0 {
                    continue;
                }
                if ((prev_mask << 1) & curr_mask) != 0 {
                    continue;
                }
                if ((prev_mask >> 1) & curr_mask) != 0 {
                    continue;
                }

                new_dp[curr_mask as usize] =
                    (new_dp[curr_mask as usize] + dp[prev_mask as usize]) % MOD;
            }
        }

        dp = new_dp;
    }

    dp.iter().fold(0, |acc, &x| (acc + x) % MOD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_grid(s: &[&str]) -> Vec<Vec<char>> {
        s.iter().map(|row| row.chars().collect()).collect()
    }

    #[test]
    fn example1() {
        // 2x2 全て空: 配置パターン
        // キングは8方向に隣接不可なので、2x2では2個置けない
        // 0個: 1, 1個: 4 → 計5
        let grid = to_grid(&["..", ".."]);
        assert_eq!(solve(2, 2, &grid), 5);
    }

    #[test]
    fn example2() {
        // 障害物あり: .# / ..
        // 0個: 1, 1個: (0,0),(1,0),(1,1) = 3
        // 2個: 全て隣接するため不可
        // 計 4
        let grid = to_grid(&[".#", ".."]);
        assert_eq!(solve(2, 2, &grid), 4);
    }

    #[test]
    fn all_blocked() {
        let grid = to_grid(&["##", "##"]);
        // 置けるのは0個のみ
        assert_eq!(solve(2, 2, &grid), 1);
    }

    #[test]
    fn single_cell() {
        let grid = to_grid(&["."]);
        // 0個または1個
        assert_eq!(solve(1, 1, &grid), 2);
    }

    #[test]
    fn row_1xn() {
        // 1x3: 隣接不可
        // 000, 100, 010, 001, 101 → 5通り
        let grid = to_grid(&["..."]);
        assert_eq!(solve(1, 3, &grid), 5);
    }
}
