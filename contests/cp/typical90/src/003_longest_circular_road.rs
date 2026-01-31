// 003 - Longest Circular Road (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_c
//
// 問題: N頂点の木に辺を1本追加してサイクルを作るとき、サイクルの最大長を求めよ。
//
// 解法: 木の直径 + 1
//       直径の両端を結べばサイクル最大。直径は2回のBFSで求まる。

use proconio::input;
use proconio::marker::Usize1;
use std::collections::VecDeque;

fn main() {
    input! {
        n: usize,
        edges: [(Usize1, Usize1); n - 1],
    }
    println!("{}", solve(n, &edges));
}

/// 隣接リストを構築
fn build_graph(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
        graph[b].push(a);
    }
    graph
}

/// BFSで最遠点とその距離を返す
fn bfs_farthest(graph: &[Vec<usize>], start: usize) -> (usize, usize) {
    let n = graph.len();
    let mut dist = vec![usize::MAX; n];
    let mut queue = VecDeque::from([start]);
    dist[start] = 0;

    while let Some(v) = queue.pop_front() {
        for &next in &graph[v] {
            if dist[next] == usize::MAX {
                dist[next] = dist[v] + 1;
                queue.push_back(next);
            }
        }
    }

    dist.into_iter()
        .enumerate()
        .filter(|&(_, d)| d != usize::MAX)
        .max_by_key(|&(_, d)| d)
        .unwrap_or((start, 0))
}

fn solve(n: usize, edges: &[(usize, usize)]) -> usize {
    let graph = build_graph(n, edges);

    // 2回BFSで木の直径を求める
    let (u, _) = bfs_farthest(&graph, 0);
    let (_, diameter) = bfs_farthest(&graph, u);

    diameter + 1 // 直径 + 追加辺
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_graph() {
        // 0-1-2-3 (直径3) → サイクル長4
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        assert_eq!(solve(4, &edges), 4);
    }

    #[test]
    fn star_graph() {
        // 中心から3本 (直径2) → サイクル長3
        let edges = vec![(0, 1), (0, 2), (0, 3)];
        assert_eq!(solve(4, &edges), 3);
    }

    #[test]
    fn single_edge() {
        let edges = vec![(0, 1)];
        assert_eq!(solve(2, &edges), 2);
    }
}
