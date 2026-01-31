// 054 - Takahashi Number (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_bb
//
// ============================================================
// 論文を頂点として追加したBFS
// ============================================================
//
// 研究者と論文の両方を頂点とするグラフを作成
// - 研究者: 頂点 0 ~ N-1
// - 論文: 頂点 N ~ N+M-1
//
// 論文jの著者リストに研究者iがいれば、辺(i, N+j)を張る
//
// 研究者1（頂点0）からBFSを実行
// 研究者への距離は必ず偶数になる（論文を経由するため）
// 高橋数 = 距離 / 2
//
// ============================================================

use proconio::input;
use std::collections::VecDeque;

fn main() {
    input! {
        n: usize,
        m: usize,
    }

    // 論文ごとの著者リスト
    let mut papers = Vec::with_capacity(m);
    for _ in 0..m {
        input! {
            k: usize,
            authors: [usize; k],
        }
        // 0-indexed に変換
        let authors: Vec<usize> = authors.iter().map(|&x| x - 1).collect();
        papers.push(authors);
    }

    solve(n, m, &papers);
}

fn solve(n: usize, m: usize, papers: &[Vec<usize>]) {
    // グラフ構築
    // 頂点 0..n: 研究者
    // 頂点 n..n+m: 論文
    let total = n + m;
    let mut graph = vec![vec![]; total];

    for (paper_idx, authors) in papers.iter().enumerate() {
        let paper_node = n + paper_idx;
        for &author in authors {
            graph[author].push(paper_node);
            graph[paper_node].push(author);
        }
    }

    // BFS from 研究者1 (頂点0)
    let mut dist = vec![-1i32; total];
    let mut queue = VecDeque::new();

    dist[0] = 0;
    queue.push_back(0);

    while let Some(v) = queue.pop_front() {
        for &u in &graph[v] {
            if dist[u] == -1 {
                dist[u] = dist[v] + 1;
                queue.push_back(u);
            }
        }
    }

    // 各研究者の高橋数を出力
    for i in 0..n {
        if dist[i] == -1 {
            println!("-1");
        } else {
            // 距離は論文を経由するので2倍になっている
            println!("{}", dist[i] / 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve_to_vec(n: usize, m: usize, papers: &[Vec<usize>]) -> Vec<i32> {
        let total = n + m;
        let mut graph = vec![vec![]; total];

        for (paper_idx, authors) in papers.iter().enumerate() {
            let paper_node = n + paper_idx;
            for &author in authors {
                graph[author].push(paper_node);
                graph[paper_node].push(author);
            }
        }

        let mut dist = vec![-1i32; total];
        let mut queue = VecDeque::new();

        dist[0] = 0;
        queue.push_back(0);

        while let Some(v) = queue.pop_front() {
            for &u in &graph[v] {
                if dist[u] == -1 {
                    dist[u] = dist[v] + 1;
                    queue.push_back(u);
                }
            }
        }

        (0..n)
            .map(|i| if dist[i] == -1 { -1 } else { dist[i] / 2 })
            .collect()
    }

    #[test]
    fn test_example1() {
        // 6人、3論文
        // 論文1: 研究者1,2,3 → 0-indexed: 0,1,2
        // 論文2: 研究者3,4 → 0-indexed: 2,3
        // 論文3: 研究者5,6 → 0-indexed: 4,5 (孤立)
        let papers = vec![vec![0, 1, 2], vec![2, 3], vec![4, 5]];
        let result = solve_to_vec(6, 3, &papers);
        // 研究者1: 0, 研究者2,3: 1, 研究者4: 2, 研究者5,6: -1
        assert_eq!(result, vec![0, 1, 1, 2, -1, -1]);
    }

    #[test]
    fn test_example2() {
        // 4人、3論文（連鎖的に繋がるケース）
        // 論文1: 研究者1,2
        // 論文2: 研究者2,3
        // 論文3: 研究者3,4
        let papers = vec![vec![0, 1], vec![1, 2], vec![2, 3]];
        let result = solve_to_vec(4, 3, &papers);
        assert_eq!(result, vec![0, 1, 2, 3]);
    }
}
