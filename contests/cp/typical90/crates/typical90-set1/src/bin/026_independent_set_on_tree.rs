// 026 - Independent Set on a Tree (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_z
//
// 問題: N頂点の木からN/2個の頂点を選ぶ。隣接する頂点を選ばない。
//       そのような選び方の例を1つ出力せよ。
//
// 解法: 二部グラフの性質を利用
//       木は二部グラフ → 2色で塗り分けられる
//       多い方の色を選べばN/2個以上取れる

use proconio::input;
use proconio::marker::Usize1;
use std::collections::VecDeque;

fn main() {
    input! {
        n: usize,
        edges: [(Usize1, Usize1); n - 1],
    }
    let result = solve(n, &edges);
    let output: Vec<String> = result.iter().map(|&x| (x + 1).to_string()).collect();
    println!("{}", output.join(" "));
}

fn solve(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    // 隣接リスト構築
    let mut graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
        graph[b].push(a);
    }

    // BFSで2色に塗り分け
    let mut color = vec![usize::MAX; n];
    let mut queue = VecDeque::new();
    color[0] = 0;
    queue.push_back(0);

    let mut groups: [Vec<usize>; 2] = [vec![], vec![]];
    groups[0].push(0);

    while let Some(v) = queue.pop_front() {
        for &next in &graph[v] {
            if color[next] == usize::MAX {
                color[next] = 1 - color[v];
                groups[color[next]].push(next);
                queue.push_back(next);
            }
        }
    }

    // 多い方のグループからN/2個選ぶ
    let target = n / 2;
    if groups[0].len() >= target {
        groups[0][..target].to_vec()
    } else {
        groups[1][..target].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_4_vertices() {
        // 木: 0-1-2-3 (パス)
        // 色0: {0, 2}, 色1: {1, 3}
        // N/2 = 2
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let result = solve(4, &edges);
        assert_eq!(result.len(), 2);

        // 隣接していないことを確認
        for i in 0..result.len() {
            for j in i + 1..result.len() {
                let diff = (result[i] as i32 - result[j] as i32).abs();
                assert_ne!(diff, 1);
            }
        }
    }

    #[test]
    fn star_graph() {
        // 中心0から1,2,3,4への星型
        // 色0: {0}, 色1: {1,2,3,4}
        // N/2 = 2
        let edges = vec![(0, 1), (0, 2), (0, 3), (0, 4)];
        let result = solve(5, &edges);
        assert_eq!(result.len(), 2);
    }
}
