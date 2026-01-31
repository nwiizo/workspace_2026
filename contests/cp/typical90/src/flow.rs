//! フローアルゴリズム
//!
//! - 最大フロー (Dinic法)
//! - 最小費用流

use std::collections::VecDeque;

/// 最大フロー (Dinic法)
///
/// 時間計算量: O(V^2 * E)
///
/// # Example
/// ```
/// use typical90::flow::MaxFlow;
///
/// let mut mf = MaxFlow::new(4);
/// mf.add_edge(0, 1, 10);
/// mf.add_edge(0, 2, 2);
/// mf.add_edge(1, 2, 6);
/// mf.add_edge(1, 3, 6);
/// mf.add_edge(2, 3, 8);
///
/// // 0->1->3: 6, 0->2->3: 2, 0->1->2->3: 4 = 12
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

    /// 辺を追加 (from -> to, 容量 cap)
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

    /// BFS でレベルグラフを構築
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

    /// DFS で増加パスを探す
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

    /// s から t への最大フローを計算
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

    /// 最小カットを構成する頂点集合を返す
    /// (s側の頂点集合)
    pub fn min_cut(&mut self, s: usize, t: usize) -> Vec<usize> {
        self.max_flow(s, t);
        self.bfs(s);
        (0..self.n).filter(|&i| self.level[i] >= 0).collect()
    }
}

/// 最小費用流 (Primal-Dual法)
///
/// # Example
/// ```
/// use typical90::flow::MinCostFlow;
///
/// let mut mcf = MinCostFlow::new(4);
/// mcf.add_edge(0, 1, 2, 1);  // cap=2, cost=1
/// mcf.add_edge(0, 2, 1, 2);  // cap=1, cost=2
/// mcf.add_edge(1, 2, 1, 1);  // cap=1, cost=1
/// mcf.add_edge(1, 3, 1, 3);  // cap=1, cost=3
/// mcf.add_edge(2, 3, 2, 1);  // cap=2, cost=1
///
/// // 2単位のフローを流す最小費用
/// let (flow, cost) = mcf.min_cost_flow(0, 3, 2);
/// assert_eq!(flow, 2);
/// assert_eq!(cost, 6);  // 0->1->2->3 (cost=3) + 0->2->3 (cost=3) = 6
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

    /// 辺を追加 (from -> to, 容量 cap, コスト cost)
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

    /// s から t へ最大 max_flow 単位のフローを流す最小費用
    ///
    /// # Returns
    /// (実際に流れたフロー, コスト)
    pub fn min_cost_flow(&mut self, s: usize, t: usize, max_flow: i64) -> (i64, i64) {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let mut flow = 0i64;
        let mut cost = 0i64;
        let mut potential = vec![0i64; self.n];
        let mut prev_v = vec![0usize; self.n];
        let mut prev_e = vec![0usize; self.n];

        while flow < max_flow {
            // ダイクストラでポテンシャルを更新
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

            // ポテンシャルを更新
            for v in 0..self.n {
                if dist[v] < i64::MAX {
                    potential[v] += dist[v];
                }
            }

            // パス上の最小容量を求める
            let mut d = max_flow - flow;
            let mut v = t;
            while v != s {
                d = d.min(self.graph[prev_v[v]][prev_e[v]].cap);
                v = prev_v[v];
            }

            // フローを流す
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

    /// 最大流かつ最小費用のフローを計算
    pub fn min_cost_max_flow(&mut self, s: usize, t: usize) -> (i64, i64) {
        self.min_cost_flow(s, t, i64::MAX)
    }
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

        // 0->1->3: 6, 0->2->3: 2, 0->1->2->3: 4 = 12
        assert_eq!(mf.max_flow(0, 3), 12);
    }

    #[test]
    fn test_max_flow_simple() {
        let mut mf = MaxFlow::new(2);
        mf.add_edge(0, 1, 5);
        assert_eq!(mf.max_flow(0, 1), 5);
    }

    #[test]
    fn test_max_flow_no_path() {
        let mut mf = MaxFlow::new(3);
        mf.add_edge(0, 1, 5);
        // 1 -> 2 への辺がない
        assert_eq!(mf.max_flow(0, 2), 0);
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
    fn test_min_cost_max_flow() {
        let mut mcf = MinCostFlow::new(3);
        mcf.add_edge(0, 1, 2, 1);
        mcf.add_edge(1, 2, 2, 1);

        let (flow, cost) = mcf.min_cost_max_flow(0, 2);
        assert_eq!(flow, 2);
        assert_eq!(cost, 4);
    }

    #[test]
    fn test_min_cost_flow_no_path() {
        let mut mcf = MinCostFlow::new(3);
        mcf.add_edge(0, 1, 5, 1);

        let (flow, _cost) = mcf.min_cost_flow(0, 2, 10);
        assert_eq!(flow, 0);
    }
}
