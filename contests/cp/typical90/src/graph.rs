//! グラフアルゴリズム

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::VecDeque;

/// BFS で最短距離を計算
pub fn bfs(graph: &[Vec<usize>], start: usize) -> Vec<i32> {
    let n = graph.len();
    let mut dist = vec![-1; n];
    let mut queue = VecDeque::from([start]);
    dist[start] = 0;

    while let Some(v) = queue.pop_front() {
        for &next in &graph[v] {
            if dist[next] == -1 {
                dist[next] = dist[v] + 1;
                queue.push_back(next);
            }
        }
    }
    dist
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

/// 木の直径を計算（直径, 端点1, 端点2）
pub fn tree_diameter(graph: &[Vec<usize>]) -> (usize, usize, usize) {
    let (u, _) = bfs_farthest(graph, 0);
    let (v, diameter) = bfs_farthest(graph, u);
    (diameter, u, v)
}

/// Union-Find (Disjoint Set Union)
#[derive(Clone)]
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            size: vec![1; n],
        }
    }

    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // 経路圧縮
        }
        self.parent[x]
    }

    pub fn unite(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry {
            return false;
        }
        // ランクによるマージ
        let (rx, ry) = if self.rank[rx] < self.rank[ry] {
            (ry, rx)
        } else {
            (rx, ry)
        };
        self.parent[ry] = rx;
        self.size[rx] += self.size[ry];
        if self.rank[rx] == self.rank[ry] {
            self.rank[rx] += 1;
        }
        true
    }

    pub fn same(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }

    pub fn group_size(&mut self, x: usize) -> usize {
        let root = self.find(x);
        self.size[root]
    }
}

/// ダイクストラ法で単一始点最短経路を計算
///
/// # Arguments
/// * `graph` - 隣接リスト（(隣接頂点, コスト)のペア）
/// * `start` - 始点
///
/// # Returns
/// 各頂点への最短距離（到達不能なら i64::MAX）
///
/// # Example
/// ```
/// use typical90::graph::dijkstra;
///
/// let graph = vec![
///     vec![(1, 1), (2, 4)],  // 0 -> 1(1), 0 -> 2(4)
///     vec![(2, 2)],          // 1 -> 2(2)
///     vec![],                // 2 -> (なし)
/// ];
/// let dist = dijkstra(&graph, 0);
/// assert_eq!(dist, vec![0, 1, 3]);  // 0->1->2 の経路が最短
/// ```
pub fn dijkstra(graph: &[Vec<(usize, i64)>], start: usize) -> Vec<i64> {
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

/// 強連結成分分解 (SCC: Strongly Connected Components)
///
/// Kosaraju のアルゴリズムを使用
///
/// # Returns
/// 各頂点が属する成分番号（トポロジカル順）
///
/// # Example
/// ```
/// use typical90::graph::scc;
///
/// // 0 -> 1 -> 2 -> 0 (1つのSCC)
/// // 2 -> 3 (3は別のSCC)
/// let graph = vec![
///     vec![1],     // 0 -> 1
///     vec![2],     // 1 -> 2
///     vec![0, 3],  // 2 -> 0, 2 -> 3
///     vec![],      // 3 -> (なし)
/// ];
/// let comp = scc(&graph);
/// // comp[0] == comp[1] == comp[2] (同じSCC)
/// // comp[3] は異なる
/// assert_eq!(comp[0], comp[1]);
/// assert_eq!(comp[1], comp[2]);
/// assert_ne!(comp[0], comp[3]);
/// ```
pub fn scc(graph: &[Vec<usize>]) -> Vec<usize> {
    let n = graph.len();

    // 逆グラフを構築
    let mut rev_graph = vec![vec![]; n];
    for (u, edges) in graph.iter().enumerate() {
        for &v in edges {
            rev_graph[v].push(u);
        }
    }

    // 帰りがけ順で頂点を記録
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

    for i in 0..n {
        if !visited[i] {
            dfs1(i, graph, &mut visited, &mut order);
        }
    }

    // 逆順で逆グラフをDFS
    let mut comp = vec![0; n];
    let mut comp_id = 0;

    fn dfs2(v: usize, rev_graph: &[Vec<usize>], comp: &mut [usize], comp_id: usize) {
        comp[v] = comp_id;
        for &next in &rev_graph[v] {
            if comp[next] == usize::MAX {
                dfs2(next, rev_graph, comp, comp_id);
            }
        }
    }

    comp.fill(usize::MAX);
    for &v in order.iter().rev() {
        if comp[v] == usize::MAX {
            dfs2(v, &rev_graph, &mut comp, comp_id);
            comp_id += 1;
        }
    }

    comp
}

/// 2-SAT ソルバー
///
/// 「x または y」の形の条件を満たす割り当てを求める
///
/// # Example
/// ```
/// use typical90::graph::TwoSat;
///
/// let mut sat = TwoSat::new(2);
/// // x0 または x1
/// sat.add_clause(0, true, 1, true);
/// // NOT x0 または NOT x1
/// sat.add_clause(0, false, 1, false);
///
/// assert!(sat.solve());
/// let answer = sat.answer();
/// // x0 XOR x1 が成り立つ
/// assert_ne!(answer[0], answer[1]);
/// ```
pub struct TwoSat {
    n: usize,
    graph: Vec<Vec<usize>>,
}

impl TwoSat {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            graph: vec![vec![]; 2 * n],
        }
    }

    /// (x_i = f) OR (x_j = g) を追加
    pub fn add_clause(&mut self, i: usize, f: bool, j: usize, g: bool) {
        // NOT f => g と NOT g => f を追加
        let ni = 2 * i + !f as usize;
        let nj = 2 * j + !g as usize;
        let pi = 2 * i + f as usize;
        let pj = 2 * j + g as usize;
        self.graph[ni].push(pj);
        self.graph[nj].push(pi);
    }

    /// 充足可能かどうかを判定
    pub fn solve(&self) -> bool {
        let comp = scc(&self.graph);
        for i in 0..self.n {
            if comp[2 * i] == comp[2 * i + 1] {
                return false;
            }
        }
        true
    }

    /// 充足可能な割り当てを返す
    pub fn answer(&self) -> Vec<bool> {
        let comp = scc(&self.graph);
        (0..self.n).map(|i| comp[2 * i] > comp[2 * i + 1]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bfs() {
        let graph = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        assert_eq!(bfs(&graph, 0), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_tree_diameter() {
        let graph = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        let (diameter, _, _) = tree_diameter(&graph);
        assert_eq!(diameter, 3);
    }

    #[test]
    fn test_union_find() {
        let mut uf = UnionFind::new(5);
        assert!(!uf.same(0, 1));
        uf.unite(0, 1);
        assert!(uf.same(0, 1));
        uf.unite(2, 3);
        uf.unite(1, 2);
        assert!(uf.same(0, 3));
        assert_eq!(uf.group_size(0), 4);
    }

    #[test]
    fn test_dijkstra() {
        let graph = vec![vec![(1, 1), (2, 4)], vec![(2, 2)], vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist, vec![0, 1, 3]);
    }

    #[test]
    fn test_dijkstra_unreachable() {
        let graph = vec![vec![(1, 1)], vec![], vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 1);
        assert_eq!(dist[2], i64::MAX);
    }

    #[test]
    fn test_scc() {
        // 0 -> 1 -> 2 -> 0 (1つのSCC)、2 -> 3 (別のSCC)
        let graph = vec![vec![1], vec![2], vec![0, 3], vec![]];
        let comp = scc(&graph);
        assert_eq!(comp[0], comp[1]);
        assert_eq!(comp[1], comp[2]);
        assert_ne!(comp[0], comp[3]);
    }

    #[test]
    fn test_two_sat() {
        let mut sat = TwoSat::new(2);
        sat.add_clause(0, true, 1, true);
        sat.add_clause(0, false, 1, false);

        assert!(sat.solve());
        let answer = sat.answer();
        assert_ne!(answer[0], answer[1]);
    }

    #[test]
    fn test_two_sat_unsatisfiable() {
        let mut sat = TwoSat::new(1);
        // x0 AND NOT x0 は矛盾
        sat.add_clause(0, true, 0, true); // x0
        sat.add_clause(0, false, 0, false); // NOT x0

        assert!(!sat.solve());
    }
}
