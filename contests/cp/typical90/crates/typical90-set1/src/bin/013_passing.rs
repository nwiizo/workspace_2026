// 013 - Passing (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_m
//
// 問題: N頂点M辺のグラフで、頂点1からNへの最短経路のうち、
//       各頂点kを通る最短経路の長さを求めよ。
//
// 解法: 両端からダイクストラ
//       dist1[k] = 1からkへの最短距離
//       distN[k] = Nからkへの最短距離
//       答え = dist1[k] + distN[k]

use proconio::input;
use proconio::marker::Usize1;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

fn main() {
    input! {
        n: usize,
        m: usize,
        edges: [(Usize1, Usize1, i64); m],
    }

    // 隣接リスト構築
    let mut graph = vec![vec![]; n];
    for &(a, b, c) in &edges {
        graph[a].push((b, c));
        graph[b].push((a, c));
    }

    let dist1 = dijkstra(&graph, 0);
    let dist_n = dijkstra(&graph, n - 1);

    for k in 0..n {
        println!("{}", dist1[k] + dist_n[k]);
    }
}

fn dijkstra(graph: &[Vec<(usize, i64)>], start: usize) -> Vec<i64> {
    let n = graph.len();
    let mut dist = vec![i64::MAX; n];
    let mut heap = BinaryHeap::new();

    dist[start] = 0;
    heap.push(Reverse((0i64, start)));

    while let Some(Reverse((d, v))) = heap.pop() {
        if d > dist[v] {
            continue;
        }
        for &(next, cost) in &graph[v] {
            let new_dist = d + cost;
            if new_dist < dist[next] {
                dist[next] = new_dist;
                heap.push(Reverse((new_dist, next)));
            }
        }
    }

    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dijkstra() {
        // 0 --1-- 1 --1-- 2
        let graph = vec![vec![(1, 1)], vec![(0, 1), (2, 1)], vec![(1, 1)]];

        let dist = dijkstra(&graph, 0);
        assert_eq!(dist, vec![0, 1, 2]);
    }

    #[test]
    fn test_passing() {
        // 0 --1-- 1 --1-- 2
        // dist1 = [0, 1, 2]
        // distN = [2, 1, 0]
        // 答え = [2, 2, 2]
        let graph = vec![vec![(1, 1)], vec![(0, 1), (2, 1)], vec![(1, 1)]];

        let dist1 = dijkstra(&graph, 0);
        let dist_n = dijkstra(&graph, 2);

        assert_eq!(dist1[0] + dist_n[0], 2);
        assert_eq!(dist1[1] + dist_n[1], 2);
        assert_eq!(dist1[2] + dist_n[2], 2);
    }
}
