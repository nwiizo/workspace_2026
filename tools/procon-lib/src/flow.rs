//! Flow Algorithms
//!
//! - Maximum Flow (Dinic's algorithm)
//! - Minimum Cost Flow

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// Maximum Flow (Dinic's algorithm)
///
/// # Complexity
/// O(V^2 * E)
///
/// # Example
/// ```
/// use procon_lib::flow::MaxFlow;
///
/// let mut mf = MaxFlow::new(4);
/// mf.add_edge(0, 1, 10);
/// mf.add_edge(0, 2, 2);
/// mf.add_edge(1, 2, 6);
/// mf.add_edge(1, 3, 6);
/// mf.add_edge(2, 3, 8);
///
/// assert_eq!(mf.max_flow(0, 3), 12);
/// ```
pub struct MaxFlow {
    n: usize,
    graph: Vec<Vec<Edge>>,
    level: Vec<i32>,
    iter: Vec<usize>,
}

#[derive(Clone, Copy)]
struct Edge {
    to: usize,
    cap: i64,
    rev: usize,
}

impl MaxFlow {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            graph: vec![vec![]; n],
            level: vec![0; n],
            iter: vec![0; n],
        }
    }

    /// Add edge from -> to with capacity cap
    pub fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        let from_len = self.graph[from].len();
        let to_len = self.graph[to].len();
        self.graph[from].push(Edge {
            to,
            cap,
            rev: to_len,
        });
        self.graph[to].push(Edge {
            to: from,
            cap: 0,
            rev: from_len,
        });
    }

    fn bfs(&mut self, s: usize) {
        self.level.fill(-1);
        let mut queue = VecDeque::new();
        self.level[s] = 0;
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            for e in &self.graph[v] {
                if e.cap > 0 && self.level[e.to] < 0 {
                    self.level[e.to] = self.level[v] + 1;
                    queue.push_back(e.to);
                }
            }
        }
    }

    fn dfs(&mut self, v: usize, t: usize, f: i64) -> i64 {
        if v == t {
            return f;
        }

        while self.iter[v] < self.graph[v].len() {
            let i = self.iter[v];
            let e = self.graph[v][i];

            if e.cap > 0 && self.level[v] < self.level[e.to] {
                let d = self.dfs(e.to, t, f.min(e.cap));
                if d > 0 {
                    self.graph[v][i].cap -= d;
                    self.graph[e.to][e.rev].cap += d;
                    return d;
                }
            }
            self.iter[v] += 1;
        }
        0
    }

    /// Calculate maximum flow from s to t
    pub fn max_flow(&mut self, s: usize, t: usize) -> i64 {
        let mut flow = 0;
        loop {
            self.bfs(s);
            if self.level[t] < 0 {
                return flow;
            }
            self.iter.fill(0);
            loop {
                let f = self.dfs(s, t, i64::MAX);
                if f == 0 {
                    break;
                }
                flow += f;
            }
        }
    }

    /// Get minimum cut (set of vertices on source side)
    pub fn min_cut(&mut self, s: usize, t: usize) -> Vec<usize> {
        self.max_flow(s, t);
        self.bfs(s);
        (0..self.n).filter(|&i| self.level[i] >= 0).collect()
    }

    /// Get flow on each edge
    pub fn get_edge_flow(&self, from: usize, idx: usize) -> i64 {
        let e = &self.graph[from][idx];
        self.graph[e.to][e.rev].cap
    }
}

/// Minimum Cost Flow (Primal-Dual algorithm)
///
/// # Example
/// ```
/// use procon_lib::flow::MinCostFlow;
///
/// let mut mcf = MinCostFlow::new(4);
/// mcf.add_edge(0, 1, 2, 1);
/// mcf.add_edge(0, 2, 1, 2);
/// mcf.add_edge(1, 2, 1, 1);
/// mcf.add_edge(1, 3, 1, 3);
/// mcf.add_edge(2, 3, 2, 1);
///
/// let (flow, cost) = mcf.min_cost_flow(0, 3, 2);
/// assert_eq!(flow, 2);
/// assert_eq!(cost, 6);
/// ```
pub struct MinCostFlow {
    n: usize,
    graph: Vec<Vec<MCFEdge>>,
}

#[derive(Clone, Copy)]
struct MCFEdge {
    to: usize,
    cap: i64,
    cost: i64,
    rev: usize,
}

impl MinCostFlow {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            graph: vec![vec![]; n],
        }
    }

    /// Add edge from -> to with capacity cap and cost cost
    pub fn add_edge(&mut self, from: usize, to: usize, cap: i64, cost: i64) {
        let from_len = self.graph[from].len();
        let to_len = self.graph[to].len();
        self.graph[from].push(MCFEdge {
            to,
            cap,
            cost,
            rev: to_len,
        });
        self.graph[to].push(MCFEdge {
            to: from,
            cap: 0,
            cost: -cost,
            rev: from_len,
        });
    }

    /// Calculate minimum cost flow
    ///
    /// # Returns
    /// (actual_flow, cost)
    pub fn min_cost_flow(&mut self, s: usize, t: usize, max_flow: i64) -> (i64, i64) {
        let mut flow = 0i64;
        let mut cost = 0i64;
        let mut potential = vec![0i64; self.n];
        let mut prev_v = vec![0usize; self.n];
        let mut prev_e = vec![0usize; self.n];

        while flow < max_flow {
            let mut dist = vec![i64::MAX; self.n];
            dist[s] = 0;
            let mut heap = BinaryHeap::new();
            heap.push(Reverse((0i64, s)));

            while let Some(Reverse((d, v))) = heap.pop() {
                if d > dist[v] {
                    continue;
                }
                for (i, e) in self.graph[v].iter().enumerate() {
                    if e.cap > 0 {
                        let new_dist = d + e.cost + potential[v] - potential[e.to];
                        if new_dist < dist[e.to] {
                            dist[e.to] = new_dist;
                            prev_v[e.to] = v;
                            prev_e[e.to] = i;
                            heap.push(Reverse((new_dist, e.to)));
                        }
                    }
                }
            }

            if dist[t] == i64::MAX {
                break;
            }

            for v in 0..self.n {
                if dist[v] < i64::MAX {
                    potential[v] += dist[v];
                }
            }

            let mut d = max_flow - flow;
            let mut v = t;
            while v != s {
                d = d.min(self.graph[prev_v[v]][prev_e[v]].cap);
                v = prev_v[v];
            }

            flow += d;
            cost += d * potential[t];
            v = t;
            while v != s {
                let pv = prev_v[v];
                let pe = prev_e[v];
                self.graph[pv][pe].cap -= d;
                let rev = self.graph[pv][pe].rev;
                self.graph[v][rev].cap += d;
                v = pv;
            }
        }

        (flow, cost)
    }

    /// Calculate maximum flow with minimum cost
    pub fn min_cost_max_flow(&mut self, s: usize, t: usize) -> (i64, i64) {
        self.min_cost_flow(s, t, i64::MAX)
    }
}

/// Bipartite Matching
///
/// Find maximum matching in a bipartite graph using augmenting paths.
///
/// # Example
/// ```
/// use procon_lib::flow::bipartite_matching;
///
/// // Left: 0, 1, 2
/// // Right: 0, 1, 2
/// // Edges: (0,0), (0,1), (1,1), (2,2)
/// let edges = vec![(0, 0), (0, 1), (1, 1), (2, 2)];
/// let matching = bipartite_matching(3, 3, &edges);
/// assert_eq!(matching, 3);
/// ```
pub fn bipartite_matching(left_n: usize, right_n: usize, edges: &[(usize, usize)]) -> usize {
    let mut graph = vec![vec![]; left_n];
    for &(l, r) in edges {
        graph[l].push(r);
    }

    let mut match_l = vec![None; left_n];
    let mut match_r = vec![None; right_n];
    let mut result = 0;

    for start in 0..left_n {
        let mut visited = vec![false; right_n];

        fn dfs(
            v: usize,
            graph: &[Vec<usize>],
            match_r: &mut [Option<usize>],
            visited: &mut [bool],
        ) -> bool {
            for &u in &graph[v] {
                if visited[u] {
                    continue;
                }
                visited[u] = true;

                match match_r[u] {
                    None => {
                        match_r[u] = Some(v);
                        return true;
                    }
                    Some(w) => {
                        if dfs(w, graph, match_r, visited) {
                            match_r[u] = Some(v);
                            return true;
                        }
                    }
                }
            }
            false
        }

        if dfs(start, &graph, &mut match_r, &mut visited) {
            result += 1;
        }
    }

    // Fill match_l
    for (r, &opt) in match_r.iter().enumerate() {
        if let Some(l) = opt {
            match_l[l] = Some(r);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_flow() {
        let mut mf = MaxFlow::new(4);
        mf.add_edge(0, 1, 10);
        mf.add_edge(0, 2, 2);
        mf.add_edge(1, 2, 6);
        mf.add_edge(1, 3, 6);
        mf.add_edge(2, 3, 8);

        assert_eq!(mf.max_flow(0, 3), 12);
    }

    #[test]
    fn test_max_flow_simple() {
        let mut mf = MaxFlow::new(2);
        mf.add_edge(0, 1, 5);
        assert_eq!(mf.max_flow(0, 1), 5);
    }

    #[test]
    fn test_min_cut() {
        let mut mf = MaxFlow::new(4);
        mf.add_edge(0, 1, 3);
        mf.add_edge(0, 2, 2);
        mf.add_edge(1, 2, 1);
        mf.add_edge(1, 3, 2);
        mf.add_edge(2, 3, 3);

        let cut = mf.min_cut(0, 3);
        assert!(cut.contains(&0));
        assert!(!cut.contains(&3));
    }

    #[test]
    fn test_min_cost_flow() {
        let mut mcf = MinCostFlow::new(4);
        mcf.add_edge(0, 1, 2, 1);
        mcf.add_edge(0, 2, 1, 2);
        mcf.add_edge(1, 2, 1, 1);
        mcf.add_edge(1, 3, 1, 3);
        mcf.add_edge(2, 3, 2, 1);

        let (flow, cost) = mcf.min_cost_flow(0, 3, 2);
        assert_eq!(flow, 2);
        assert_eq!(cost, 6);
    }

    #[test]
    fn test_bipartite_matching() {
        let edges = vec![(0, 0), (0, 1), (1, 1), (2, 2)];
        assert_eq!(bipartite_matching(3, 3, &edges), 3);

        let edges2 = vec![(0, 0), (1, 0)];
        assert_eq!(bipartite_matching(2, 1, &edges2), 1);
    }
}
