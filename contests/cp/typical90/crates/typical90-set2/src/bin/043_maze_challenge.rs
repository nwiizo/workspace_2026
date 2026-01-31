// 043 - Maze Challenge with Lack of Sleep (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_aq
//
// 0-1 BFS
// - 同じ方向に進む: コスト0
// - 方向を変える: コスト1
//
// 状態: (row, col, direction)
// 0-1 BFS: コスト0は先頭に、コスト1は末尾に追加

use proconio::input;
use proconio::marker::Chars;
use std::collections::VecDeque;

const INF: i32 = std::i32::MAX;
const DR: [i32; 4] = [-1, 1, 0, 0]; // 上下左右
const DC: [i32; 4] = [0, 0, -1, 1];

fn main() {
    input! {
        h: usize,
        w: usize,
        rs: usize,
        cs: usize,
        rt: usize,
        ct: usize,
        grid: [Chars; h],
    }
    println!("{}", solve(h, w, rs - 1, cs - 1, rt - 1, ct - 1, &grid));
}

fn solve(
    h: usize,
    w: usize,
    rs: usize,
    cs: usize,
    rt: usize,
    ct: usize,
    grid: &[Vec<char>],
) -> i32 {
    // dist[r][c][d] = (r,c)に方向dで到達する最小コスト
    let mut dist = vec![vec![vec![INF; 4]; w]; h];
    let mut deque = VecDeque::new();

    // 開始点から4方向への初期移動
    for d in 0..4 {
        let nr = rs as i32 + DR[d];
        let nc = cs as i32 + DC[d];
        if nr >= 0 && nr < h as i32 && nc >= 0 && nc < w as i32 {
            let nr = nr as usize;
            let nc = nc as usize;
            if grid[nr][nc] == '.' {
                dist[nr][nc][d] = 0;
                deque.push_front((nr, nc, d, 0));
            }
        }
    }

    while let Some((r, c, dir, cost)) = deque.pop_front() {
        if cost > dist[r][c][dir] {
            continue;
        }

        for d in 0..4 {
            let nr = r as i32 + DR[d];
            let nc = c as i32 + DC[d];
            if nr >= 0 && nr < h as i32 && nc >= 0 && nc < w as i32 {
                let nr = nr as usize;
                let nc = nc as usize;
                if grid[nr][nc] == '.' {
                    let new_cost = if d == dir { cost } else { cost + 1 };
                    if new_cost < dist[nr][nc][d] {
                        dist[nr][nc][d] = new_cost;
                        if d == dir {
                            deque.push_front((nr, nc, d, new_cost));
                        } else {
                            deque.push_back((nr, nc, d, new_cost));
                        }
                    }
                }
            }
        }
    }

    // ゴールへの最小コスト
    *dist[rt][ct].iter().min().unwrap_or(&INF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // ..#
        // #.#
        // #..
        let grid = vec![
            vec!['.', '.', '#'],
            vec!['#', '.', '#'],
            vec!['#', '.', '.'],
        ];
        // Start (1,1) -> (0,0), Goal (3,3) -> (2,2)
        assert_eq!(solve(3, 3, 0, 0, 2, 2, &grid), 2);
    }

    #[test]
    fn test_example2() {
        // #.#
        // ...
        // #.#
        let grid = vec![
            vec!['#', '.', '#'],
            vec!['.', '.', '.'],
            vec!['#', '.', '#'],
        ];
        // Start (2,1) -> (1,0), Goal (2,3) -> (1,2)
        assert_eq!(solve(3, 3, 1, 0, 1, 2, &grid), 0);
    }

    #[test]
    fn test_example3() {
        // ...#..
        // .#.##.
        // .#....
        // ...##.
        let grid = vec![
            vec!['.', '.', '.', '#', '.', '.'],
            vec!['.', '#', '.', '#', '#', '.'],
            vec!['.', '#', '.', '.', '.', '.'],
            vec!['.', '.', '.', '#', '#', '.'],
        ];
        // Start (2,1) -> (1,0), Goal (1,5) -> (0,4)
        assert_eq!(solve(4, 6, 1, 0, 0, 4, &grid), 5);
    }
}
