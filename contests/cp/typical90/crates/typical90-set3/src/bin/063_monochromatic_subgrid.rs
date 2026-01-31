// 063 - Monochromatic Subgrid (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_bk
//
// H ≤ 8 なので行の選び方を bit 全探索
// 各列について、選んだ行すべてで同じ値ならカウント

use proconio::input;
use std::collections::HashMap;

fn main() {
    input! {
        h: usize,
        w: usize,
        grid: [[i64; w]; h],
    }
    println!("{}", solve(h, w, &grid));
}

fn solve(h: usize, w: usize, grid: &[Vec<i64>]) -> usize {
    let mut ans = 0;

    // 行の部分集合を全探索
    for mask in 1..(1 << h) {
        let row_count = (mask as usize).count_ones() as usize;

        // 各列について、選んだ行すべてで同じ値かチェック
        let mut value_count: HashMap<i64, usize> = HashMap::new();

        for col in 0..w {
            let mut all_same = true;
            let mut val = 0i64;

            for row in 0..h {
                if (mask >> row) & 1 == 1 {
                    if val == 0 {
                        val = grid[row][col];
                    } else if grid[row][col] != val {
                        all_same = false;
                        break;
                    }
                }
            }

            if all_same {
                *value_count.entry(val).or_insert(0) += 1;
            }
        }

        // 最大の列数を取得
        if let Some(&max_cols) = value_count.values().max() {
            ans = ans.max(row_count * max_cols);
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let grid = vec![
            vec![1, 1, 1, 1, 1, 2],
            vec![1, 2, 2, 2, 2, 2],
            vec![1, 2, 2, 3, 2, 3],
            vec![1, 2, 3, 2, 2, 3],
        ];
        assert_eq!(solve(4, 6, &grid), 6);
    }

    #[test]
    fn test_example2() {
        let grid = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        assert_eq!(solve(3, 3, &grid), 1);
    }

    #[test]
    fn test_example3() {
        let grid = vec![
            vec![7, 7, 7],
            vec![7, 7, 7],
            vec![7, 7, 7],
            vec![7, 7, 7],
            vec![7, 7, 7],
        ];
        assert_eq!(solve(5, 3, &grid), 15);
    }
}
