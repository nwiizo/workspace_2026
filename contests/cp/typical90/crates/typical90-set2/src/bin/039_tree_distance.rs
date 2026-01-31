//! 039 - Tree Distance (★5)
//!
//! 木の辺の寄与計算
//!
//! 全頂点ペア (i, j) の距離の総和を求める。
//!
//! 各辺 e について、その辺を通るパスの数を数える。
//! 辺 e を取り除くと木が2つの部分木に分かれる。
//! サイズを s, n-s とすると、辺 e を通るパスは s * (n-s) 本。
//! 答え = Σ s * (n-s) for all edges

use proconio::input;
use proconio::marker::Usize1;

fn main() {
    input! {
        n: usize,
        edges: [(Usize1, Usize1); n - 1],
    }
    println!("{}", solve(n, &edges));
}

fn solve(n: usize, edges: &[(usize, usize)]) -> i64 {
    if n == 1 {
        return 0;
    }

    // 隣接リスト構築
    let mut graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
        graph[b].push(a);
    }

    // DFSで各頂点を根とする部分木のサイズを計算
    let mut subtree_size = vec![0usize; n];
    let mut visited = vec![false; n];
    let mut stack = vec![(0usize, false)]; // (node, processed)

    while let Some((v, processed)) = stack.pop() {
        if processed {
            subtree_size[v] = 1;
            for &next in &graph[v] {
                if visited[next] {
                    // next は v の子
                    // (親方向ではなく子方向のみカウント)
                }
            }
            // 子の部分木サイズを合算
            for &next in &graph[v] {
                if subtree_size[next] > 0 && next != 0 {
                    // next が v の子であることを確認
                    // 実際には親判定が必要
                }
            }
        } else {
            if visited[v] {
                continue;
            }
            visited[v] = true;
            stack.push((v, true));
            for &next in &graph[v] {
                if !visited[next] {
                    stack.push((next, false));
                }
            }
        }
    }

    // 再帰で書き直す
    fn dfs(v: usize, parent: isize, graph: &[Vec<usize>], size: &mut [usize]) {
        size[v] = 1;
        for &next in &graph[v] {
            if next as isize != parent {
                dfs(next, v as isize, graph, size);
                size[v] += size[next];
            }
        }
    }

    let mut size = vec![0usize; n];
    dfs(0, -1, &graph, &mut size);

    // 各辺の寄与を計算
    // 辺 (a, b) で b が a の子なら、部分木サイズは size[b]
    // 寄与 = size[b] * (n - size[b])
    let mut total = 0i64;
    for &(a, b) in edges {
        // どちらが親か判定: size が小さい方が子
        let s = size[a].min(size[b]);
        total += s as i64 * (n - s) as i64;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 3頂点のパス: 0-1-2
        // 距離: d(0,1)=1, d(0,2)=2, d(1,2)=1 → 合計4
        assert_eq!(solve(3, &[(0, 1), (1, 2)]), 4);
    }

    #[test]
    fn example2() {
        // 4頂点のスター: 0が中心
        // 距離: d(0,1)=d(0,2)=d(0,3)=1, d(1,2)=d(1,3)=d(2,3)=2
        // 合計 = 3*1 + 3*2 = 9
        assert_eq!(solve(4, &[(0, 1), (0, 2), (0, 3)]), 9);
    }

    #[test]
    fn single_edge() {
        assert_eq!(solve(2, &[(0, 1)]), 1);
    }

    #[test]
    fn path_4() {
        // 0-1-2-3
        // d(0,1)=1, d(0,2)=2, d(0,3)=3
        // d(1,2)=1, d(1,3)=2
        // d(2,3)=1
        // 合計 = 1+2+3+1+2+1 = 10
        assert_eq!(solve(4, &[(0, 1), (1, 2), (2, 3)]), 10);
    }
}
