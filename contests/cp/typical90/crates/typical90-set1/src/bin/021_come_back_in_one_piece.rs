// 021 - Come Back in One Piece (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_u
//
// 問題: 有向グラフで、同じ強連結成分に属する頂点ペアの数を求めよ。
//
// 解法: 強連結成分分解 (SCC)
//       各SCCのサイズをsとすると、ペア数は s*(s-1)/2

use proconio::input;
use proconio::marker::Usize1;

fn main() {
    input! {
        n: usize,
        m: usize,
        edges: [(Usize1, Usize1); m],
    }
    println!("{}", solve(n, &edges));
}

fn solve(n: usize, edges: &[(usize, usize)]) -> i64 {
    // 隣接リスト（正方向と逆方向）
    let mut graph = vec![vec![]; n];
    let mut rev_graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
        rev_graph[b].push(a);
    }

    // 1回目のDFS: 帰りがけ順を記録
    let mut order = Vec::with_capacity(n);
    let mut visited = vec![false; n];

    fn dfs1(v: usize, graph: &[Vec<usize>], visited: &mut [bool], order: &mut Vec<usize>) {
        visited[v] = true;
        for &next in &graph[v] {
            if !visited[next] {
                dfs1(next, graph, visited, order);
            }
        }
        order.push(v);
    }

    for v in 0..n {
        if !visited[v] {
            dfs1(v, &graph, &mut visited, &mut order);
        }
    }

    // 2回目のDFS: 逆グラフで帰りがけ順の逆順に探索
    let mut component_id = vec![0usize; n];
    let mut component_sizes = vec![];

    fn dfs2(
        v: usize,
        id: usize,
        rev_graph: &[Vec<usize>],
        component_id: &mut [usize],
        size: &mut usize,
    ) {
        component_id[v] = id;
        *size += 1;
        for &next in &rev_graph[v] {
            if component_id[next] == usize::MAX {
                dfs2(next, id, rev_graph, component_id, size);
            }
        }
    }

    component_id.fill(usize::MAX);
    for &v in order.iter().rev() {
        if component_id[v] == usize::MAX {
            let mut size = 0;
            dfs2(
                v,
                component_sizes.len(),
                &rev_graph,
                &mut component_id,
                &mut size,
            );
            component_sizes.push(size);
        }
    }

    // 各SCCからペア数を計算
    component_sizes
        .iter()
        .map(|&s| s as i64 * (s as i64 - 1) / 2)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 3頂点、3辺: 0→1, 1→2, 2→0 (1つのSCC)
        // ペア数: 3*2/2 = 3
        let edges = vec![(0, 1), (1, 2), (2, 0)];
        assert_eq!(solve(3, &edges), 3);
    }

    #[test]
    fn example2() {
        // 3頂点、2辺: 0→1, 1→2 (3つのSCC、各サイズ1)
        // ペア数: 0
        let edges = vec![(0, 1), (1, 2)];
        assert_eq!(solve(3, &edges), 0);
    }

    #[test]
    fn two_sccs() {
        // 4頂点: 0↔1, 2↔3
        // SCC1: {0,1}, SCC2: {2,3}
        // ペア数: 1 + 1 = 2
        let edges = vec![(0, 1), (1, 0), (2, 3), (3, 2)];
        assert_eq!(solve(4, &edges), 2);
    }
}
