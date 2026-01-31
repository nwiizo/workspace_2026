//! Tree Algorithms
//!
//! - LCA (Lowest Common Ancestor) with binary lifting
//! - HLD (Heavy-Light Decomposition)
//! - Rerooting DP

/// LCA with binary lifting
///
/// # Example
/// ```
/// use procon_lib::tree::Lca;
///
/// // Tree:     0
/// //          / \
/// //         1   2
/// //        / \
/// //       3   4
/// let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
/// let lca = Lca::new(5, &edges, 0);
///
/// assert_eq!(lca.lca(3, 4), 1);
/// assert_eq!(lca.lca(3, 2), 0);
/// assert_eq!(lca.dist(3, 4), 2);
/// ```
pub struct Lca {
    parent: Vec<Vec<usize>>,
    depth: Vec<usize>,
    log: usize,
}

impl Lca {
    /// Create LCA structure
    ///
    /// # Arguments
    /// - `n`: Number of vertices
    /// - `edges`: Undirected edges
    /// - `root`: Root vertex
    ///
    /// # Complexity
    /// - Construction: O(N log N)
    /// - Query: O(log N)
    pub fn new(n: usize, edges: &[(usize, usize)], root: usize) -> Self {
        let mut graph = vec![vec![]; n];
        for &(u, v) in edges {
            graph[u].push(v);
            graph[v].push(u);
        }

        let log = (usize::BITS - n.leading_zeros()) as usize;
        let mut parent = vec![vec![n; n]; log];
        let mut depth = vec![0; n];

        // BFS to compute depth and immediate parent
        let mut visited = vec![false; n];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root);
        visited[root] = true;
        parent[0][root] = root;

        while let Some(v) = queue.pop_front() {
            for &u in &graph[v] {
                if !visited[u] {
                    visited[u] = true;
                    parent[0][u] = v;
                    depth[u] = depth[v] + 1;
                    queue.push_back(u);
                }
            }
        }

        // Binary lifting
        for k in 1..log {
            for v in 0..n {
                let p = parent[k - 1][v];
                parent[k][v] = if p < n { parent[k - 1][p] } else { n };
            }
        }

        Self { parent, depth, log }
    }

    /// Get LCA of u and v
    pub fn lca(&self, mut u: usize, mut v: usize) -> usize {
        if self.depth[u] > self.depth[v] {
            std::mem::swap(&mut u, &mut v);
        }

        // Bring v to same depth as u
        let diff = self.depth[v] - self.depth[u];
        for k in 0..self.log {
            if (diff >> k) & 1 == 1 {
                v = self.parent[k][v];
            }
        }

        if u == v {
            return u;
        }

        // Binary search for LCA
        for k in (0..self.log).rev() {
            if self.parent[k][u] != self.parent[k][v] {
                u = self.parent[k][u];
                v = self.parent[k][v];
            }
        }

        self.parent[0][u]
    }

    /// Get distance between u and v
    pub fn dist(&self, u: usize, v: usize) -> usize {
        self.depth[u] + self.depth[v] - 2 * self.depth[self.lca(u, v)]
    }

    /// Get depth of vertex v
    pub fn depth(&self, v: usize) -> usize {
        self.depth[v]
    }

    /// Get k-th ancestor of v (0-indexed, 0 = v itself)
    pub fn kth_ancestor(&self, mut v: usize, k: usize) -> Option<usize> {
        if k > self.depth[v] {
            return None;
        }
        for i in 0..self.log {
            if (k >> i) & 1 == 1 {
                v = self.parent[i][v];
            }
        }
        Some(v)
    }

    /// Get vertex on path from u to v at distance d from u
    pub fn jump(&self, u: usize, v: usize, d: usize) -> Option<usize> {
        let l = self.lca(u, v);
        let dist_ul = self.depth[u] - self.depth[l];
        let dist_vl = self.depth[v] - self.depth[l];

        if d <= dist_ul {
            self.kth_ancestor(u, d)
        } else if d <= dist_ul + dist_vl {
            self.kth_ancestor(v, dist_ul + dist_vl - d)
        } else {
            None
        }
    }
}

/// Heavy-Light Decomposition
///
/// Decomposes a tree into chains for efficient path queries.
///
/// # Example
/// ```
/// use procon_lib::tree::Hld;
///
/// let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
/// let hld = Hld::new(5, &edges, 0);
///
/// // Get path decomposition for queries
/// let path = hld.path(3, 4, false);
/// assert!(!path.is_empty());
/// ```
pub struct Hld {
    parent: Vec<usize>,
    depth: Vec<usize>,
    heavy: Vec<usize>,
    head: Vec<usize>,
    pos: Vec<usize>,
    size: Vec<usize>,
}

impl Hld {
    /// Create HLD structure
    ///
    /// # Complexity
    /// - Construction: O(N)
    /// - Path query: O(log N) chains
    pub fn new(n: usize, edges: &[(usize, usize)], root: usize) -> Self {
        let mut graph = vec![vec![]; n];
        for &(u, v) in edges {
            graph[u].push(v);
            graph[v].push(u);
        }

        let mut parent = vec![n; n];
        let mut depth = vec![0; n];
        let mut size = vec![1; n];
        let mut heavy = vec![n; n];

        // DFS to compute size and heavy child
        fn dfs_size(
            v: usize,
            p: usize,
            graph: &[Vec<usize>],
            parent: &mut [usize],
            depth: &mut [usize],
            size: &mut [usize],
            heavy: &mut [usize],
        ) {
            parent[v] = p;
            let mut max_size = 0;
            for &u in &graph[v] {
                if u != p {
                    depth[u] = depth[v] + 1;
                    dfs_size(u, v, graph, parent, depth, size, heavy);
                    size[v] += size[u];
                    if size[u] > max_size {
                        max_size = size[u];
                        heavy[v] = u;
                    }
                }
            }
        }

        dfs_size(root, n, &graph, &mut parent, &mut depth, &mut size, &mut heavy);

        // DFS to compute head and position
        let mut head = vec![0; n];
        let mut pos = vec![0; n];
        let mut cnt = 0;

        fn dfs_decompose(
            v: usize,
            h: usize,
            graph: &[Vec<usize>],
            parent: &[usize],
            heavy: &[usize],
            head: &mut [usize],
            pos: &mut [usize],
            cnt: &mut usize,
        ) {
            head[v] = h;
            pos[v] = *cnt;
            *cnt += 1;

            // Visit heavy child first
            if heavy[v] < parent.len() {
                dfs_decompose(heavy[v], h, graph, parent, heavy, head, pos, cnt);
            }

            // Visit light children
            for &u in &graph[v] {
                if u != parent[v] && u != heavy[v] {
                    dfs_decompose(u, u, graph, parent, heavy, head, pos, cnt);
                }
            }
        }

        dfs_decompose(root, root, &graph, &parent, &heavy, &mut head, &mut pos, &mut cnt);

        Self {
            parent,
            depth,
            heavy,
            head,
            pos,
            size,
        }
    }

    /// Get LCA of u and v
    pub fn lca(&self, mut u: usize, mut v: usize) -> usize {
        while self.head[u] != self.head[v] {
            if self.depth[self.head[u]] > self.depth[self.head[v]] {
                u = self.parent[self.head[u]];
            } else {
                v = self.parent[self.head[v]];
            }
        }
        if self.depth[u] < self.depth[v] {
            u
        } else {
            v
        }
    }

    /// Get path decomposition as ranges [l, r] in DFS order
    ///
    /// If `vertex` is true, includes LCA; if false, excludes LCA (for edge queries).
    pub fn path(&self, mut u: usize, mut v: usize, vertex: bool) -> Vec<(usize, usize)> {
        let mut result = Vec::new();

        while self.head[u] != self.head[v] {
            if self.depth[self.head[u]] > self.depth[self.head[v]] {
                result.push((self.pos[self.head[u]], self.pos[u]));
                u = self.parent[self.head[u]];
            } else {
                result.push((self.pos[self.head[v]], self.pos[v]));
                v = self.parent[self.head[v]];
            }
        }

        if self.depth[u] > self.depth[v] {
            std::mem::swap(&mut u, &mut v);
        }

        let l = if vertex { self.pos[u] } else { self.pos[u] + 1 };
        if l <= self.pos[v] {
            result.push((l, self.pos[v]));
        }

        result
    }

    /// Get position in DFS order
    pub fn pos(&self, v: usize) -> usize {
        self.pos[v]
    }

    /// Get subtree range [l, r) in DFS order
    pub fn subtree(&self, v: usize) -> (usize, usize) {
        (self.pos[v], self.pos[v] + self.size[v])
    }
}

/// Rerooting DP template
///
/// Compute DP values for all vertices as root.
///
/// # Type Parameters
/// - `V`: Vertex DP value type
/// - `E`: Edge contribution type
///
/// # Example
/// ```
/// use procon_lib::tree::rerooting;
///
/// // Tree with edge weights, compute sum of distances from each vertex
/// let edges = vec![(0, 1, 1i64), (0, 2, 2), (1, 3, 3)];
/// let n = 4;
///
/// // For distance sum: merge = add, add_edge = value + weight, identity = 0
/// let result = rerooting(
///     n,
///     &edges,
///     || 0i64,           // identity
///     |a, b| a + b,      // merge
///     |v, e| v + e + 1,  // add_edge: add 1 for each descendant
/// );
///
/// // result[i] = sum of distances from vertex i
/// ```
pub fn rerooting<V, E, Identity, Merge, AddEdge>(
    n: usize,
    edges: &[(usize, usize, E)],
    identity: Identity,
    merge: Merge,
    add_edge: AddEdge,
) -> Vec<V>
where
    V: Clone,
    E: Clone,
    Identity: Fn() -> V,
    Merge: Fn(V, V) -> V,
    AddEdge: Fn(V, E) -> V,
{
    let mut graph: Vec<Vec<(usize, E)>> = vec![vec![]; n];
    for (u, v, e) in edges {
        graph[*u].push((*v, e.clone()));
        graph[*v].push((*u, e.clone()));
    }

    let mut dp = vec![identity(); n];
    let mut result = vec![identity(); n];

    // First DFS: compute dp[v] = contribution from subtree rooted at v
    fn dfs1<V, E, Identity, Merge, AddEdge>(
        v: usize,
        p: usize,
        graph: &[Vec<(usize, E)>],
        dp: &mut [V],
        identity: &Identity,
        merge: &Merge,
        add_edge: &AddEdge,
    ) where
        V: Clone,
        E: Clone,
        Identity: Fn() -> V,
        Merge: Fn(V, V) -> V,
        AddEdge: Fn(V, E) -> V,
    {
        dp[v] = identity();
        for (u, e) in &graph[v] {
            if *u != p {
                dfs1(*u, v, graph, dp, identity, merge, add_edge);
                dp[v] = merge(dp[v].clone(), add_edge(dp[*u].clone(), e.clone()));
            }
        }
    }

    // Second DFS: compute result[v] with v as root
    fn dfs2<V, E, Identity, Merge, AddEdge>(
        v: usize,
        p: usize,
        from_parent: V,
        graph: &[Vec<(usize, E)>],
        dp: &[V],
        result: &mut [V],
        identity: &Identity,
        merge: &Merge,
        add_edge: &AddEdge,
    ) where
        V: Clone,
        E: Clone,
        Identity: Fn() -> V,
        Merge: Fn(V, V) -> V,
        AddEdge: Fn(V, E) -> V,
    {
        let children: Vec<_> = graph[v]
            .iter()
            .filter(|(u, _)| *u != p)
            .cloned()
            .collect();

        let m = children.len();

        // Compute prefix and suffix products
        let mut prefix = vec![identity(); m + 1];
        let mut suffix = vec![identity(); m + 1];

        for i in 0..m {
            let (u, e) = &children[i];
            prefix[i + 1] = merge(prefix[i].clone(), add_edge(dp[*u].clone(), e.clone()));
        }
        for i in (0..m).rev() {
            let (u, e) = &children[i];
            suffix[i] = merge(add_edge(dp[*u].clone(), e.clone()), suffix[i + 1].clone());
        }

        // Result for v
        result[v] = merge(from_parent.clone(), prefix[m].clone());

        // Recurse to children
        for i in 0..m {
            let (u, e) = &children[i];
            let contribution =
                add_edge(merge(from_parent.clone(), merge(prefix[i].clone(), suffix[i + 1].clone())), e.clone());
            dfs2(*u, v, contribution, graph, dp, result, identity, merge, add_edge);
        }
    }

    if n > 0 {
        dfs1(0, n, &graph, &mut dp, &identity, &merge, &add_edge);
        dfs2(0, n, identity(), &graph, &dp, &mut result, &identity, &merge, &add_edge);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lca() {
        let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
        let lca = Lca::new(5, &edges, 0);

        assert_eq!(lca.lca(3, 4), 1);
        assert_eq!(lca.lca(3, 2), 0);
        assert_eq!(lca.lca(0, 0), 0);
        assert_eq!(lca.dist(3, 4), 2);
        assert_eq!(lca.dist(3, 2), 3);
    }

    #[test]
    fn test_lca_kth_ancestor() {
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let lca = Lca::new(4, &edges, 0);

        assert_eq!(lca.kth_ancestor(3, 0), Some(3));
        assert_eq!(lca.kth_ancestor(3, 1), Some(2));
        assert_eq!(lca.kth_ancestor(3, 2), Some(1));
        assert_eq!(lca.kth_ancestor(3, 3), Some(0));
        assert_eq!(lca.kth_ancestor(3, 4), None);
    }

    #[test]
    fn test_hld() {
        let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
        let hld = Hld::new(5, &edges, 0);

        assert_eq!(hld.lca(3, 4), 1);
        assert_eq!(hld.lca(3, 2), 0);

        let path = hld.path(3, 4, true);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_hld_subtree() {
        let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
        let hld = Hld::new(5, &edges, 0);

        let (l, r) = hld.subtree(0);
        assert_eq!(r - l, 5); // entire tree

        let (l1, r1) = hld.subtree(1);
        assert_eq!(r1 - l1, 3); // subtree of 1 has 3 nodes
    }
}
