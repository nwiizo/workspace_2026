// 077 - Planes on a 2D Plane (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_by
//
// 二部マッチング問題
// 各始点から各終点への移動が可能か判定し、マッチングを求める
// 方向: 1=右, 2=右上, 3=上, 4=左上, 5=左, 6=左下, 7=下, 8=右下
// 移動ベクトル: (dx, dy) * T

use proconio::input;
use std::collections::HashMap;

fn main() {
    input! {
        n: usize,
        t: i64,
        starts: [(i64, i64); n],
        ends: [(i64, i64); n],
    }

    match solve(n, t, &starts, &ends) {
        Some(directions) => {
            println!("Yes");
            let s: Vec<String> = directions.iter().map(|d| d.to_string()).collect();
            println!("{}", s.join(" "));
        }
        None => println!("No"),
    }
}

fn solve(n: usize, t: i64, starts: &[(i64, i64)], ends: &[(i64, i64)]) -> Option<Vec<usize>> {
    // 方向ベクトル (1-indexed: 1=右, 2=右上, 3=上, ...)
    let dirs: [(i64, i64); 8] = [
        (1, 0),   // 1: 右
        (1, 1),   // 2: 右上
        (0, 1),   // 3: 上
        (-1, 1),  // 4: 左上
        (-1, 0),  // 5: 左
        (-1, -1), // 6: 左下
        (0, -1),  // 7: 下
        (1, -1),  // 8: 右下
    ];

    // 終点の位置 -> インデックス のマップ
    let mut end_map: HashMap<(i64, i64), usize> = HashMap::new();
    for (i, &pos) in ends.iter().enumerate() {
        end_map.insert(pos, i);
    }

    // 各始点から到達可能な終点とその方向を求める
    // graph[i] = vec![(end_idx, direction), ...]
    let mut graph: Vec<Vec<(usize, usize)>> = vec![vec![]; n];

    for (i, &(sx, sy)) in starts.iter().enumerate() {
        for (d, &(dx, dy)) in dirs.iter().enumerate() {
            let ex = sx + dx * t;
            let ey = sy + dy * t;

            if let Some(&end_idx) = end_map.get(&(ex, ey)) {
                graph[i].push((end_idx, d + 1)); // 方向は 1-indexed
            }
        }
    }

    // 二部マッチング（Kuhn's algorithm）
    let mut match_end: Vec<Option<usize>> = vec![None; n]; // end -> start
    let mut result_dir: Vec<usize> = vec![0; n]; // start -> direction

    fn try_kuhn(
        v: usize,
        graph: &[Vec<(usize, usize)>],
        used: &mut [bool],
        match_end: &mut [Option<usize>],
        result_dir: &mut [usize],
    ) -> bool {
        if used[v] {
            return false;
        }
        used[v] = true;

        for &(to, dir) in &graph[v] {
            if match_end[to].is_none()
                || try_kuhn(match_end[to].unwrap(), graph, used, match_end, result_dir)
            {
                match_end[to] = Some(v);
                result_dir[v] = dir;
                return true;
            }
        }
        false
    }

    let mut matched = 0;
    for v in 0..n {
        let mut used = vec![false; n];
        if try_kuhn(v, &graph, &mut used, &mut match_end, &mut result_dir) {
            matched += 1;
        }
    }

    if matched == n { Some(result_dir) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let starts = vec![(3, 3), (5, 5), (9, 2)];
        let ends = vec![(11, 2), (5, 5), (3, 3)];
        let result = solve(3, 2, &starts, &ends);
        assert!(result.is_some());
        // 検証: 各始点から方向に移動して終点に到達するか
        let dirs: [(i64, i64); 8] = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];
        let directions = result.unwrap();
        for (i, &(sx, sy)) in starts.iter().enumerate() {
            let d = directions[i] - 1;
            let ex = sx + dirs[d].0 * 2;
            let ey = sy + dirs[d].1 * 2;
            assert!(ends.contains(&(ex, ey)));
        }
    }

    #[test]
    fn test_example2() {
        let starts = vec![(3, 3), (5, 5), (9, 2)];
        let ends = vec![(11, 1_000_000_000), (5, 5), (3, 3)];
        let result = solve(3, 2, &starts, &ends);
        assert!(result.is_none());
    }
}
