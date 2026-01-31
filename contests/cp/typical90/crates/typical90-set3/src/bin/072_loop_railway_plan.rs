// 072 - Loop Railway Plan (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_bt
//
// H * W <= 16 なので DFS でハミルトン閉路を探索
// 始点を固定して、全てのマスを訪問して始点に戻れるかチェック
// 最大のサイクル長を求める

use proconio::input;
use proconio::marker::Chars;

fn main() {
    input! {
        h: usize,
        w: usize,
        grid: [Chars; h],
    }
    println!("{}", solve(h, w, &grid));
}

fn solve(h: usize, w: usize, grid: &[Vec<char>]) -> i32 {
    let dx = [0, 1, 0, -1];
    let dy = [1, 0, -1, 0];

    let mut ans = -1;

    // 始点を全ての平地マスで試す
    for start_i in 0..h {
        for start_j in 0..w {
            if grid[start_i][start_j] == '#' {
                continue;
            }

            // DFSで閉路を探索
            let mut visited = vec![vec![false; w]; h];
            visited[start_i][start_j] = true;

            fn dfs(
                i: usize,
                j: usize,
                start_i: usize,
                start_j: usize,
                count: i32,
                h: usize,
                w: usize,
                grid: &[Vec<char>],
                visited: &mut Vec<Vec<bool>>,
                dx: &[i32; 4],
                dy: &[i32; 4],
                ans: &mut i32,
            ) {
                // 隣接マスを探索
                for d in 0..4 {
                    let ni = i as i32 + dx[d];
                    let nj = j as i32 + dy[d];

                    if ni < 0 || ni >= h as i32 || nj < 0 || nj >= w as i32 {
                        continue;
                    }

                    let ni = ni as usize;
                    let nj = nj as usize;

                    // 始点に戻ってきた場合（3マス以上訪問していれば閉路）
                    if ni == start_i && nj == start_j {
                        if count >= 3 {
                            *ans = (*ans).max(count);
                        }
                        continue;
                    }

                    if grid[ni][nj] == '#' || visited[ni][nj] {
                        continue;
                    }

                    visited[ni][nj] = true;
                    dfs(
                        ni,
                        nj,
                        start_i,
                        start_j,
                        count + 1,
                        h,
                        w,
                        grid,
                        visited,
                        dx,
                        dy,
                        ans,
                    );
                    visited[ni][nj] = false;
                }
            }

            dfs(
                start_i,
                start_j,
                start_i,
                start_j,
                1,
                h,
                w,
                grid,
                &mut visited,
                &dx,
                &dy,
                &mut ans,
            );
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
            vec!['.', '.', '.'],
            vec!['.', '#', '.'],
            vec!['.', '.', '.'],
        ];
        assert_eq!(solve(3, 3, &grid), 8);
    }

    #[test]
    fn test_example2() {
        let grid = vec![vec!['.', '.', '.', '.', '.', '.']];
        assert_eq!(solve(1, 6, &grid), -1);
    }

    #[test]
    fn test_example3() {
        let grid = vec![
            vec!['.', '.', '.', '.'],
            vec!['#', '.', '.', '.'],
            vec!['.', '.', '.', '.'],
            vec!['.', '.', '.', '#'],
        ];
        assert_eq!(solve(4, 4, &grid), 12);
    }
}
