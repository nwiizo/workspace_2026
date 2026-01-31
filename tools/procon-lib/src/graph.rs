//! Graph Algorithms
//!
//! - BFS, Dijkstra, Bellman-Ford, Floyd-Warshall
//! - SCC, 2-SAT
//! - Union-Find
//! - Topological Sort
//! - 0-1 BFS

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// BFS for unweighted graph
///
/// # Returns
/// Distance from start (-1 if unreachable)
///
/// # Example
/// ```
/// use procon_lib::graph::bfs;
///
/// let graph = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
/// assert_eq!(bfs(&graph, 0), vec![0, 1, 2, 3]);
/// ```
pub fn bfs(graph: &[Vec<usize>], start: usize) -> Vec<i64> {
    let n = graph.len();
    let mut dist = vec![-1i64; n];
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

/// BFS returning farthest point and its distance
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

/// Tree diameter
///
/// # Returns
/// (diameter, endpoint1, endpoint2)
///
/// # Example
/// ```
/// use procon_lib::graph::tree_diameter;
///
/// let graph = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
/// let (diameter, _, _) = tree_diameter(&graph);
/// assert_eq!(diameter, 3);
/// ```
pub fn tree_diameter(graph: &[Vec<usize>]) -> (usize, usize, usize) {
    let (u, _) = bfs_farthest(graph, 0);
    let (v, diameter) = bfs_farthest(graph, u);
    (diameter, u, v)
}

/// Union-Find (Disjoint Set Union)
///
/// # Example
/// ```
/// use procon_lib::graph::UnionFind;
///
/// let mut uf = UnionFind::new(5);
/// uf.unite(0, 1);
/// uf.unite(2, 3);
/// assert!(uf.same(0, 1));
/// assert!(!uf.same(0, 2));
/// uf.unite(1, 2);
/// assert!(uf.same(0, 3));
/// assert_eq!(uf.size(0), 4);
/// ```
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
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    pub fn unite(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry {
            return false;
        }
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

    pub fn size(&mut self, x: usize) -> usize {
        let root = self.find(x);
        self.size[root]
    }

    /// Get all groups
    pub fn groups(&mut self) -> Vec<Vec<usize>> {
        let n = self.parent.len();
        let mut groups: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for i in 0..n {
            let root = self.find(i);
            groups.entry(root).or_default().push(i);
        }
        groups.into_values().collect()
    }
}

/// Dijkstra's algorithm
///
/// # Returns
/// Distance from start (i64::MAX if unreachable)
///
/// # Complexity
/// O((V + E) log V)
///
/// # Example
/// ```
/// use procon_lib::graph::dijkstra;
///
/// let graph = vec![
///     vec![(1, 1), (2, 4)],
///     vec![(2, 2)],
///     vec![],
/// ];
/// let dist = dijkstra(&graph, 0);
/// assert_eq!(dist, vec![0, 1, 3]);
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

/// Bellman-Ford algorithm (handles negative edges)
///
/// Returns None if negative cycle exists.
///
/// # Complexity
/// O(VE)
///
/// # Example
/// ```
/// use procon_lib::graph::bellman_ford;
///
/// let edges = vec![(0, 1, 1), (1, 2, -2), (0, 2, 3)];
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
            return None;
        }
    }
    Some(dist)
}

/// Floyd-Warshall algorithm (all pairs shortest path)
///
/// # Complexity
/// O(V^3)
///
/// # Example
/// ```
/// use procon_lib::graph::floyd_warshall;
///
/// let mut dist = vec![
///     vec![0, 1, i64::MAX],
///     vec![i64::MAX, 0, 2],
///     vec![i64::MAX, i64::MAX, 0],
/// ];
/// floyd_warshall(&mut dist);
/// assert_eq!(dist[0][2], 3);
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

/// Strongly Connected Components (Kosaraju's algorithm)
///
/// # Returns
/// Component ID for each vertex (topologically ordered)
///
/// # Example
/// ```
/// use procon_lib::graph::scc;
///
/// let graph = vec![vec![1], vec![2], vec![0, 3], vec![]];
/// let comp = scc(&graph);
/// assert_eq!(comp[0], comp[1]);
/// assert_eq!(comp[1], comp[2]);
/// assert_ne!(comp[0], comp[3]);
/// ```
pub fn scc(graph: &[Vec<usize>]) -> Vec<usize> {
    let n = graph.len();

    let mut rev_graph = vec![vec![]; n];
    for (u, edges) in graph.iter().enumerate() {
        for &v in edges {
            rev_graph[v].push(u);
        }
    }

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

    let mut comp = vec![usize::MAX; n];
    let mut comp_id = 0;

    fn dfs2(v: usize, rev_graph: &[Vec<usize>], comp: &mut [usize], comp_id: usize) {
        comp[v] = comp_id;
        for &next in &rev_graph[v] {
            if comp[next] == usize::MAX {
                dfs2(next, rev_graph, comp, comp_id);
            }
        }
    }

    for &v in order.iter().rev() {
        if comp[v] == usize::MAX {
            dfs2(v, &rev_graph, &mut comp, comp_id);
            comp_id += 1;
        }
    }

    comp
}

/// 2-SAT solver
///
/// # Example
/// ```
/// use procon_lib::graph::TwoSat;
///
/// let mut sat = TwoSat::new(2);
/// sat.add_clause(0, true, 1, true);   // x0 OR x1
/// sat.add_clause(0, false, 1, false); // NOT x0 OR NOT x1
///
/// assert!(sat.satisfiable());
/// let answer = sat.answer();
/// assert_ne!(answer[0], answer[1]);  // x0 XOR x1
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

    /// Add clause (x_i = f) OR (x_j = g)
    pub fn add_clause(&mut self, i: usize, f: bool, j: usize, g: bool) {
        let ni = 2 * i + !f as usize;
        let nj = 2 * j + !g as usize;
        let pi = 2 * i + f as usize;
        let pj = 2 * j + g as usize;
        self.graph[ni].push(pj);
        self.graph[nj].push(pi);
    }

    /// Check if satisfiable
    pub fn satisfiable(&self) -> bool {
        let comp = scc(&self.graph);
        for i in 0..self.n {
            if comp[2 * i] == comp[2 * i + 1] {
                return false;
            }
        }
        true
    }

    /// Get satisfying assignment
    pub fn answer(&self) -> Vec<bool> {
        let comp = scc(&self.graph);
        (0..self.n).map(|i| comp[2 * i] > comp[2 * i + 1]).collect()
    }
}

/// Topological sort (Kahn's algorithm)
///
/// Returns None if graph has a cycle.
///
/// # Example
/// ```
/// use procon_lib::graph::topological_sort;
///
/// let graph = vec![vec![1, 2], vec![3], vec![3], vec![]];
/// let order = topological_sort(&graph).unwrap();
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
        None
    }
}

/// 0-1 BFS (for graphs with edge weights 0 or 1)
///
/// # Complexity
/// O(V + E)
///
/// # Example
/// ```
/// use procon_lib::graph::bfs_01;
///
/// let graph = vec![
///     vec![(1, 0), (2, 1)],
///     vec![(3, 1)],
///     vec![(3, 0)],
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

/// Grid graph helper
///
/// Returns adjacent positions in a grid.
pub fn grid_neighbors(r: usize, c: usize, h: usize, w: usize) -> Vec<(usize, usize)> {
    const DR: [i64; 4] = [-1, 0, 1, 0];
    const DC: [i64; 4] = [0, 1, 0, -1];

    let mut result = Vec::with_capacity(4);
    for d in 0..4 {
        let nr = r as i64 + DR[d];
        let nc = c as i64 + DC[d];
        if nr >= 0 && nr < h as i64 && nc >= 0 && nc < w as i64 {
            result.push((nr as usize, nc as usize));
        }
    }
    result
}

/// Grid graph helper (8 directions)
pub fn grid_neighbors_8(r: usize, c: usize, h: usize, w: usize) -> Vec<(usize, usize)> {
    const DR: [i64; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    const DC: [i64; 8] = [0, 1, 1, 1, 0, -1, -1, -1];

    let mut result = Vec::with_capacity(8);
    for d in 0..8 {
        let nr = r as i64 + DR[d];
        let nc = c as i64 + DC[d];
        if nr >= 0 && nr < h as i64 && nc >= 0 && nc < w as i64 {
            result.push((nr as usize, nc as usize));
        }
    }
    result
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
        assert_eq!(uf.size(0), 4);
    }

    #[test]
    fn test_dijkstra() {
        let graph = vec![vec![(1, 1), (2, 4)], vec![(2, 2)], vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist, vec![0, 1, 3]);
    }

    #[test]
    fn test_bellman_ford() {
        let edges = vec![(0, 1, 1), (1, 2, -2), (0, 2, 3)];
        let dist = bellman_ford(3, &edges, 0).unwrap();
        assert_eq!(dist, vec![0, 1, -1]);
    }

    #[test]
    fn test_bellman_ford_negative_cycle() {
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
    fn test_scc() {
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

        assert!(sat.satisfiable());
        let answer = sat.answer();
        assert_ne!(answer[0], answer[1]);
    }

    #[test]
    fn test_topological_sort() {
        let graph = vec![vec![1, 2], vec![3], vec![3], vec![]];
        let order = topological_sort(&graph).unwrap();
        assert_eq!(order[0], 0);
        assert_eq!(order[3], 3);
    }

    #[test]
    fn test_bfs_01() {
        let graph = vec![
            vec![(1, 0), (2, 1)],
            vec![(3, 1)],
            vec![(3, 0)],
            vec![],
        ];
        let dist = bfs_01(&graph, 0);
        assert_eq!(dist, vec![0, 0, 1, 1]);
    }

    #[test]
    fn test_grid_neighbors() {
        let neighbors = grid_neighbors(1, 1, 3, 3);
        assert_eq!(neighbors.len(), 4);

        let corner = grid_neighbors(0, 0, 3, 3);
        assert_eq!(corner.len(), 2);
    }
}
