// 026 - Independent Set on a Tree (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_z
//
// ============================================================================
// 【物語で理解する問題】
// ============================================================================
//
// 会社のパーティーを計画しています。
//
// N 人の社員がいて、上下関係が「木」の形をしています（直属の上司がいる）。
// パーティーでは「直接の上下関係にある2人」を同時に招待すると気まずい。
//
// N/2 人を招待したい。気まずくない組み合わせを1つ見つけてください。
//
// 例: 4人の会社
//
//     0
//    /|\
//   1 2 3
//
// - 0 はボス
// - 1, 2, 3 は 0 の部下
//
// 0 を招待すると 1, 2, 3 は招待できない（直属の関係）
// 1, 2 を招待: OK! 2人で N/2 = 2 を満たす
//
// ============================================================================
// 【解法：木の二部グラフ性質】
// ============================================================================
//
// 【重要な性質】
//
// 木は必ず「二部グラフ」になる！
//
// 二部グラフとは、頂点を2色で塗り分けられるグラフ。
// 同じ色の頂点同士は隣接しない。
//
// 【アルゴリズム】
//
// 1. BFS/DFS で木を2色に塗り分ける
// 2. 色0の頂点集合と色1の頂点集合に分ける
// 3. 大きい方の集合から N/2 個選ぶ
//
// 【なぜこれでうまくいく？】
//
// - 二部グラフでは、同じ色の頂点は隣接しない
// - 木は N 頂点、N-1 辺なので、必ずどちらかの色が N/2 以上ある
//   （2色合わせて N 頂点なので、少なくとも一方は N/2 以上）
//
// ============================================================================
// 【計算量】
// ============================================================================
//
// - BFS: O(N)
// - 合計: O(N)
//
// ============================================================================

use proconio::input;
use proconio::marker::Usize1;
use std::collections::VecDeque;

fn main() {
    input! {
        n: usize,
        edges: [(Usize1, Usize1); n - 1],
    }
    let result = solve(n, &edges);

    // 1-indexed で出力
    let output: Vec<String> = result.iter().map(|&x| (x + 1).to_string()).collect();
    println!("{}", output.join(" "));
}

fn solve(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    // -------------------------------------------------------------------------
    // 【隣接リストの構築】
    // -------------------------------------------------------------------------
    let mut graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
        graph[b].push(a);
    }

    // -------------------------------------------------------------------------
    // 【BFSで2色に塗り分け】
    //
    // color[v] = 頂点 v の色（0 または 1）
    // 隣接する頂点は異なる色にする
    // -------------------------------------------------------------------------
    let mut color = vec![usize::MAX; n];
    let mut queue = VecDeque::new();

    color[0] = 0;
    queue.push_back(0);

    // 各色のグループ
    let mut groups: [Vec<usize>; 2] = [vec![], vec![]];
    groups[0].push(0);

    while let Some(v) = queue.pop_front() {
        for &next in &graph[v] {
            if color[next] == usize::MAX {
                // 隣接頂点は反対の色
                color[next] = 1 - color[v];
                groups[color[next]].push(next);
                queue.push_back(next);
            }
        }
    }

    // -------------------------------------------------------------------------
    // 【多い方のグループから N/2 個選ぶ】
    //
    // どちらか一方は必ず N/2 以上の頂点を持つ
    // -------------------------------------------------------------------------
    let target = n / 2;
    if groups[0].len() >= target {
        groups[0][..target].to_vec()
    } else {
        groups[1][..target].to_vec()
    }
}

// =============================================================================
// 【テスト】
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// 結果が有効かチェック
    fn is_valid(n: usize, edges: &[(usize, usize)], result: &[usize]) -> bool {
        // 正しい個数か
        if result.len() != n / 2 {
            return false;
        }

        // 隣接グラフを構築
        let mut adj = vec![vec![false; n]; n];
        for &(a, b) in edges {
            adj[a][b] = true;
            adj[b][a] = true;
        }

        // 選ばれた頂点同士が隣接していないか
        for i in 0..result.len() {
            for j in i + 1..result.len() {
                if adj[result[i]][result[j]] {
                    return false;
                }
            }
        }

        true
    }

    #[test]
    fn path_4_vertices() {
        // パスグラフ: 0 - 1 - 2 - 3
        //
        // 色0: {0, 2}
        // 色1: {1, 3}
        //
        // どちらを選んでも隣接しない
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let result = solve(4, &edges);
        assert!(is_valid(4, &edges, &result));
    }

    #[test]
    fn star_graph() {
        // 星型グラフ: 中心0から1,2,3,4への辺
        //
        // 色0: {0}
        // 色1: {1, 2, 3, 4}
        //
        // 色1から2個選べば隣接しない
        let edges = vec![(0, 1), (0, 2), (0, 3), (0, 4)];
        let result = solve(5, &edges);
        assert!(is_valid(5, &edges, &result));
    }

    #[test]
    fn binary_tree() {
        // 二分木:
        //       0
        //      / \
        //     1   2
        //    / \
        //   3   4
        //
        // 色0: {0, 3, 4}
        // 色1: {1, 2}
        let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4)];
        let result = solve(5, &edges);
        assert!(is_valid(5, &edges, &result));
    }

    #[test]
    fn two_vertices() {
        // 最小ケース: 2頂点
        // 0 - 1
        //
        // N/2 = 1 個選ぶ
        let edges = vec![(0, 1)];
        let result = solve(2, &edges);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn long_path() {
        // 長いパス: 0-1-2-3-4-5
        //
        // 色0: {0, 2, 4}
        // 色1: {1, 3, 5}
        //
        // N/2 = 3 個選ぶ
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
        let result = solve(6, &edges);
        assert!(is_valid(6, &edges, &result));
    }
}
