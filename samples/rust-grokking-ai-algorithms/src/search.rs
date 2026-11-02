use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// An unweighted directed graph indexed by node number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    edges: Vec<Vec<usize>>,
}

impl Graph {
    pub fn new(node_count: usize) -> Self {
        Self {
            edges: vec![Vec::new(); node_count],
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize) -> Result<(), &'static str> {
        if from >= self.edges.len() || to >= self.edges.len() {
            return Err("node index is outside the graph");
        }
        self.edges[from].push(to);
        Ok(())
    }

    pub fn breadth_first_path(&self, start: usize, goal: usize) -> Option<Vec<usize>> {
        self.find_path(start, goal)
    }

    pub fn depth_first_path(&self, start: usize, goal: usize) -> Option<Vec<usize>> {
        self.validate_nodes(start, goal)?;
        let mut parent = vec![None; self.edges.len()];
        let mut visited = vec![false; self.edges.len()];
        let mut stack = vec![(start, None)];
        while let Some((node, previous)) = stack.pop() {
            if visited[node] {
                continue;
            }
            visited[node] = true;
            parent[node] = previous;
            if node == goal {
                return Some(rebuild_path(&parent, goal));
            }
            for &next in self.edges[node].iter().rev() {
                if !visited[next] {
                    stack.push((next, Some(node)));
                }
            }
        }
        None
    }

    fn find_path(&self, start: usize, goal: usize) -> Option<Vec<usize>> {
        self.validate_nodes(start, goal)?;
        let mut parent = vec![None; self.edges.len()];
        let mut visited = vec![false; self.edges.len()];
        let mut frontier = VecDeque::from([start]);
        visited[start] = true;

        while let Some(node) = frontier.pop_front() {
            if node == goal {
                return Some(rebuild_path(&parent, goal));
            }
            for &next in &self.edges[node] {
                if !visited[next] {
                    visited[next] = true;
                    parent[next] = Some(node);
                    frontier.push_back(next);
                }
            }
        }
        None
    }

    fn validate_nodes(&self, start: usize, goal: usize) -> Option<()> {
        (start < self.edges.len() && goal < self.edges.len()).then_some(())
    }
}

fn rebuild_path(parent: &[Option<usize>], mut node: usize) -> Vec<usize> {
    let mut path = vec![node];
    while let Some(previous) = parent[node] {
        node = previous;
        path.push(node);
    }
    path.reverse();
    path
}

/// Finds a minimum-cost path with A* when the heuristic never overestimates the
/// remaining cost (including h(goal) = 0). Costs beyond u32::MAX are skipped.
/// Returns `None` for invalid endpoints, unreachable goals, or invalid edge data.
pub fn a_star(
    edges: &[Vec<(usize, u32)>],
    heuristic: &[u32],
    start: usize,
    goal: usize,
) -> Option<(u32, Vec<usize>)> {
    if edges.len() != heuristic.len() || start >= edges.len() || goal >= edges.len() {
        return None;
    }
    if edges.iter().flatten().any(|(next, _)| *next >= edges.len()) {
        return None;
    }

    let mut best = vec![None; edges.len()];
    let mut parent = vec![None; edges.len()];
    let mut frontier = BinaryHeap::new();
    best[start] = Some(0);
    frontier.push(Reverse((u64::from(heuristic[start]), 0_u32, start)));

    while let Some(Reverse((_, cost, node))) = frontier.pop() {
        if Some(cost) != best[node] {
            continue;
        }
        if node == goal {
            return Some((cost, rebuild_path(&parent, goal)));
        }
        for &(next, edge_cost) in &edges[node] {
            let Some(candidate) = cost.checked_add(edge_cost) else {
                continue;
            };
            if best[next].is_none_or(|known| candidate < known) {
                let priority = u64::from(candidate) + u64::from(heuristic[next]);
                best[next] = Some(candidate);
                parent[next] = Some(node);
                frontier.push(Reverse((priority, candidate, next)));
            }
        }
    }
    None
}

/// Minimax for a complete binary tree with exactly 2^depth leaves in left-to-right order.
/// Invalid shapes return None; depth zero evaluates the single leaf.
pub fn minimax(values: &[i32], depth: usize, maximizing: bool) -> Option<i32> {
    if depth >= usize::BITS as usize || values.len() != 1usize << depth {
        return None;
    }
    fn visit(values: &[i32], depth: usize, maximizing: bool) -> i32 {
        if depth == 0 {
            return values[0];
        }
        let (left, right) = values.split_at(values.len() / 2);
        let a = visit(left, depth - 1, !maximizing);
        let b = visit(right, depth - 1, !maximizing);
        if maximizing { a.max(b) } else { a.min(b) }
    }
    Some(visit(values, depth, maximizing))
}

/// Alpha-beta on a complete binary tree; returns value and visited leaf count.
pub fn alpha_beta(values: &[i32], depth: usize, maximizing: bool) -> Option<(i32, usize)> {
    if depth >= usize::BITS as usize || values.len() != 1usize << depth {
        return None;
    }
    fn visit(v: &[i32], max: bool, mut alpha: i32, mut beta: i32, leaves: &mut usize) -> i32 {
        if v.len() == 1 {
            *leaves += 1;
            return v[0];
        }
        let mut best = if max { i32::MIN } else { i32::MAX };
        for child in v.chunks(v.len() / 2) {
            let score = visit(child, !max, alpha, beta, leaves);
            if max {
                best = best.max(score);
                alpha = alpha.max(best);
            } else {
                best = best.min(score);
                beta = beta.min(best);
            }
            if alpha >= beta {
                break;
            }
        }
        best
    }
    let mut leaves = 0;
    let value = visit(values, maximizing, i32::MIN, i32::MAX, &mut leaves);
    Some((value, leaves))
}

/// Chapter 1's number guessing: sorted input is required.
pub fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    let (mut low, mut high) = (0, values.len());
    while low < high {
        let middle = low + (high - low) / 2;
        match values[middle].cmp(&target) {
            std::cmp::Ordering::Equal => return Some(middle),
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfs_returns_shortest_path_and_handles_unreachable_nodes() {
        let mut graph = Graph::new(5);
        graph.add_edge(0, 1).unwrap();
        graph.add_edge(0, 2).unwrap();
        graph.add_edge(1, 3).unwrap();
        graph.add_edge(2, 4).unwrap();
        graph.add_edge(4, 3).unwrap();
        assert_eq!(graph.breadth_first_path(0, 3), Some(vec![0, 1, 3]));
        assert_eq!(graph.breadth_first_path(0, 8), None);
    }

    #[test]
    fn dfs_terminates_on_cycles() {
        let mut graph = Graph::new(3);
        graph.add_edge(0, 1).unwrap();
        graph.add_edge(1, 0).unwrap();
        graph.add_edge(1, 2).unwrap();
        assert_eq!(graph.depth_first_path(0, 2), Some(vec![0, 1, 2]));
    }

    #[test]
    fn astar_uses_edge_costs_and_skips_overflow() {
        let edges = vec![
            vec![(1, 8), (2, 1)],
            vec![(3, 1)],
            vec![(1, 1), (3, 7)],
            vec![],
        ];
        let result = a_star(&edges, &[2, 1, 1, 0], 0, 3).unwrap();
        assert_eq!(result, (3, vec![0, 2, 1, 3]));
    }

    #[test]
    fn minimax_selects_best_worst_case() {
        assert_eq!(minimax(&[3, 5, 2, 9], 2, true), Some(3));
        assert_eq!(minimax(&[], 1, true), None);
    }
}
