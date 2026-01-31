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

/// ベルマンフォード法（負辺対応）
///
/// 負閉路がある場合は None を返す
///
/// # Example
/// ```
/// use typical90::graph::bellman_ford;
///
/// let edges = vec![
///     (0, 1, 1),
///     (1, 2, -2),
///     (0, 2, 3),
/// ];
/// let dist = bellman_ford(3, &edges, 0).unwrap();
/// assert_eq!(dist, vec![0, 1, -1]);
/// ```
pub fn bellman_ford(n: usize, edges: &[(usize, usize, i64)], start: usize) -> Option<Vec<i64>> {
    let mut dist = vec![i64::MAX; n];
    dist[start] = 0;

    for i in 0..n {
        let mut updated = false;
        for &(from, to, cost) in edges {
            if dist[from] != i64::MAX && dist[from] + cost < dist[to] {
                dist[to] = dist[from] + cost;
                updated = true;
            }
        }
        if !updated {
            return Some(dist);
        }
        if i == n - 1 && updated {
            return None; // 負閉路あり
        }
    }
    Some(dist)
}

/// フロイドワーシャル法（全点対最短経路）
///
/// # Example
/// ```
/// use typical90::graph::floyd_warshall;
///
/// let mut dist = vec![
///     vec![0, 1, i64::MAX],
///     vec![i64::MAX, 0, 2],
///     vec![i64::MAX, i64::MAX, 0],
/// ];
/// floyd_warshall(&mut dist);
/// assert_eq!(dist[0][2], 3); // 0 -> 1 -> 2
/// ```
#[allow(clippy::needless_range_loop)]
pub fn floyd_warshall(dist: &mut [Vec<i64>]) {
    let n = dist.len();
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if dist[i][k] != i64::MAX && dist[k][j] != i64::MAX {
                    dist[i][j] = dist[i][j].min(dist[i][k] + dist[k][j]);
                }
            }
        }
    }
}

/// トポロジカルソート（Kahn's algorithm）
///
/// DAGでない場合は None を返す
///
/// # Example
/// ```
/// use typical90::graph::topological_sort;
///
/// let graph = vec![
///     vec![1, 2],  // 0 -> 1, 2
///     vec![3],     // 1 -> 3
///     vec![3],     // 2 -> 3
///     vec![],      // 3
/// ];
/// let order = topological_sort(&graph).unwrap();
/// // 0 が最初、3 が最後
/// assert_eq!(order[0], 0);
/// assert_eq!(order[3], 3);
/// ```
pub fn topological_sort(graph: &[Vec<usize>]) -> Option<Vec<usize>> {
    let n = graph.len();
    let mut in_degree = vec![0usize; n];
    for edges in graph {
        for &to in edges {
            in_degree[to] += 1;
        }
    }

    let mut queue: VecDeque<usize> = in_degree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
        .collect();

    let mut result = Vec::with_capacity(n);
    while let Some(v) = queue.pop_front() {
        result.push(v);
        for &next in &graph[v] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    if result.len() == n {
        Some(result)
    } else {
        None // 閉路あり
    }
}

/// 0-1 BFS（辺の重みが0か1のみの最短経路）
///
/// # Example
/// ```
/// use typical90::graph::bfs_01;
///
/// let graph = vec![
///     vec![(1, 0), (2, 1)],  // 0 -> 1(0), 0 -> 2(1)
///     vec![(3, 1)],          // 1 -> 3(1)
///     vec![(3, 0)],          // 2 -> 3(0)
///     vec![],
/// ];
/// let dist = bfs_01(&graph, 0);
/// assert_eq!(dist, vec![0, 0, 1, 1]);
/// ```
pub fn bfs_01(graph: &[Vec<(usize, i64)>], start: usize) -> Vec<i64> {
    let n = graph.len();
    let mut dist = vec![i64::MAX; n];
    let mut deque = VecDeque::new();

    dist[start] = 0;
    deque.push_front(start);

    while let Some(v) = deque.pop_front() {
        for &(next, cost) in &graph[v] {
            let new_dist = dist[v] + cost;
            if new_dist < dist[next] {
                dist[next] = new_dist;
                if cost == 0 {
                    deque.push_front(next);
                } else {
                    deque.push_back(next);
                }
            }
        }
    }
    dist
}

/// 最小全域木（クラスカル法）
///
/// # Returns
/// (最小コスト, 使用した辺のインデックス)
///
/// # Example
/// ```
/// use typical90::graph::kruskal;
///
/// let edges = vec![
///     (0, 1, 1),
///     (1, 2, 2),
///     (0, 2, 3),
/// ];
/// let (cost, used) = kruskal(3, &edges);
/// assert_eq!(cost, 3);
/// assert_eq!(used.len(), 2);
/// ```
pub fn kruskal(n: usize, edges: &[(usize, usize, i64)]) -> (i64, Vec<usize>) {
    let mut indexed_edges: Vec<_> = edges.iter().enumerate().collect();
    indexed_edges.sort_by_key(|(_, (_, _, c))| *c);

    let mut uf = UnionFind::new(n);
    let mut total_cost = 0;
    let mut used = Vec::new();

    for (idx, &(u, v, cost)) in indexed_edges {
        if uf.unite(u, v) {
            total_cost += cost;
            used.push(idx);
        }
    }

    (total_cost, used)
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

    #[test]
    fn test_bellman_ford() {
        let edges = vec![(0, 1, 1), (1, 2, -2), (0, 2, 3)];
        let dist = bellman_ford(3, &edges, 0).unwrap();
        assert_eq!(dist, vec![0, 1, -1]);
    }

    #[test]
    fn test_bellman_ford_negative_cycle() {
        // 0 -> 1 -> 2 -> 0 で合計 -1 の負閉路
        let edges = vec![(0, 1, 1), (1, 2, 1), (2, 0, -3)];
        assert!(bellman_ford(3, &edges, 0).is_none());
    }

    #[test]
    fn test_floyd_warshall() {
        let mut dist = vec![
            vec![0, 1, i64::MAX],
            vec![i64::MAX, 0, 2],
            vec![i64::MAX, i64::MAX, 0],
        ];
        floyd_warshall(&mut dist);
        assert_eq!(dist[0][2], 3);
    }

    #[test]
    fn test_topological_sort() {
        let graph = vec![vec![1, 2], vec![3], vec![3], vec![]];
        let order = topological_sort(&graph).unwrap();
        assert_eq!(order[0], 0);
        assert_eq!(order[3], 3);
    }

    #[test]
    fn test_topological_sort_cycle() {
        // 0 -> 1 -> 2 -> 0 の閉路
        let graph = vec![vec![1], vec![2], vec![0]];
        assert!(topological_sort(&graph).is_none());
    }

    #[test]
    fn test_bfs_01() {
        let graph = vec![vec![(1, 0), (2, 1)], vec![(3, 1)], vec![(3, 0)], vec![]];
        let dist = bfs_01(&graph, 0);
        assert_eq!(dist, vec![0, 0, 1, 1]);
    }

    #[test]
    fn test_kruskal() {
        let edges = vec![(0, 1, 1), (1, 2, 2), (0, 2, 3)];
        let (cost, used) = kruskal(3, &edges);
        assert_eq!(cost, 3);
        assert_eq!(used.len(), 2);
    }
}
