//! グラフアルゴリズム

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
}
