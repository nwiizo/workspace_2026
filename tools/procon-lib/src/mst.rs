//! Minimum Spanning Tree
//!
//! - Kruskal's algorithm
//! - Prim's algorithm

use crate::graph::UnionFind;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Kruskal's algorithm for MST
///
/// # Returns
/// (total_cost, list of edge indices used)
///
/// # Complexity
/// O(E log E)
///
/// # Example
/// ```
/// use procon_lib::mst::kruskal;
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

/// Prim's algorithm for MST
///
/// # Returns
/// (total_cost, parent array where parent[root] = root)
///
/// # Complexity
/// O((V + E) log V)
///
/// # Example
/// ```
/// use procon_lib::mst::prim;
///
/// let graph = vec![
///     vec![(1, 1), (2, 3)],
///     vec![(0, 1), (2, 2)],
///     vec![(0, 3), (1, 2)],
/// ];
/// let (cost, parent) = prim(&graph, 0);
/// assert_eq!(cost, 3);
/// assert_eq!(parent[0], 0);  // root
/// ```
pub fn prim(graph: &[Vec<(usize, i64)>], start: usize) -> (i64, Vec<usize>) {
    let n = graph.len();
    let mut visited = vec![false; n];
    let mut parent = vec![usize::MAX; n];
    let mut total_cost = 0;

    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0i64, start, start)));

    while let Some(Reverse((cost, v, p))) = heap.pop() {
        if visited[v] {
            continue;
        }
        visited[v] = true;
        parent[v] = p;
        total_cost += cost;

        for &(next, c) in &graph[v] {
            if !visited[next] {
                heap.push(Reverse((c, next, v)));
            }
        }
    }

    (total_cost, parent)
}

/// Minimum Spanning Tree with edge list representation
///
/// Returns the MST as a list of edges.
///
/// # Example
/// ```
/// use procon_lib::mst::mst_edges;
///
/// let edges = vec![
///     (0, 1, 1),
///     (1, 2, 2),
///     (0, 2, 3),
/// ];
/// let mst = mst_edges(3, &edges);
/// assert_eq!(mst.len(), 2);
/// ```
pub fn mst_edges(n: usize, edges: &[(usize, usize, i64)]) -> Vec<(usize, usize, i64)> {
    let (_, used) = kruskal(n, edges);
    used.into_iter().map(|i| edges[i]).collect()
}

/// Check if graph is connected
pub fn is_connected(n: usize, edges: &[(usize, usize)]) -> bool {
    if n == 0 {
        return true;
    }
    let mut uf = UnionFind::new(n);
    for &(u, v) in edges {
        uf.unite(u, v);
    }
    let root = uf.find(0);
    (0..n).all(|i| uf.find(i) == root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kruskal() {
        let edges = vec![(0, 1, 1), (1, 2, 2), (0, 2, 3)];
        let (cost, used) = kruskal(3, &edges);
        assert_eq!(cost, 3);
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn test_kruskal_disconnected() {
        let edges = vec![(0, 1, 1)];
        let (cost, used) = kruskal(3, &edges);
        assert_eq!(cost, 1);
        assert_eq!(used.len(), 1);
    }

    #[test]
    fn test_prim() {
        let graph = vec![
            vec![(1, 1), (2, 3)],
            vec![(0, 1), (2, 2)],
            vec![(0, 3), (1, 2)],
        ];
        let (cost, parent) = prim(&graph, 0);
        assert_eq!(cost, 3);
        assert_eq!(parent[0], 0);
    }

    #[test]
    fn test_is_connected() {
        assert!(is_connected(3, &[(0, 1), (1, 2)]));
        assert!(!is_connected(3, &[(0, 1)]));
        assert!(is_connected(1, &[]));
    }
}
