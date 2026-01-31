// 071 - Fuzzy Priority (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_bs
//
// トポロジカルソートで辞書順最小のK個の順列を出力
// 優先度付きキューを使った Kahn's algorithm の変形
// ただし K 個の候補を同時に管理

use proconio::input;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

fn main() {
    input! {
        n: usize,
        m: usize,
        k: usize,
        edges: [(usize, usize); m],
    }

    let result = solve(n, m, k, &edges);
    match result {
        Some(permutations) => {
            for perm in permutations {
                let s: Vec<String> = perm.iter().map(|x| x.to_string()).collect();
                println!("{}", s.join(" "));
            }
        }
        None => println!("-1"),
    }
}

fn solve(n: usize, _m: usize, k: usize, edges: &[(usize, usize)]) -> Option<Vec<Vec<usize>>> {
    // グラフ構築
    let mut graph = vec![vec![]; n + 1];
    let mut in_degree = vec![0; n + 1];

    for &(a, b) in edges {
        graph[a].push(b);
        in_degree[b] += 1;
    }

    // 状態: (現在の順列, 入次数の配列, 利用可能な頂点のヒープ)
    // BFSで辞書順に探索

    // 初期状態
    let mut initial_heap = BinaryHeap::new();
    for v in 1..=n {
        if in_degree[v] == 0 {
            initial_heap.push(Reverse(v));
        }
    }

    // 状態を管理: (順列, 入次数, 利用可能頂点)
    let mut states: Vec<(Vec<usize>, Vec<usize>, BinaryHeap<Reverse<usize>>)> = vec![];
    states.push((vec![], in_degree.clone(), initial_heap));

    let mut result = vec![];

    while !states.is_empty() && result.len() < k {
        // 辞書順最小の状態を探す
        // 全ての状態の次の選択肢のうち最小を選ぶ

        // 完了した順列を結果に追加
        let mut new_states = vec![];
        for (perm, deg, heap) in states {
            if perm.len() == n {
                result.push(perm);
                if result.len() >= k {
                    break;
                }
            } else {
                new_states.push((perm, deg, heap));
            }
        }
        states = new_states;

        if result.len() >= k || states.is_empty() {
            break;
        }

        // 次の頂点を選ぶ: 各状態から最大K個の次状態を生成
        let mut next_states: Vec<(Vec<usize>, Vec<usize>, BinaryHeap<Reverse<usize>>)> = vec![];

        for (perm, deg, mut heap) in states {
            if heap.is_empty() {
                // サイクルがあって完成できない
                continue;
            }

            // 次の候補を最大K個取り出す
            let mut candidates = vec![];
            while let Some(Reverse(v)) = heap.pop() {
                candidates.push(v);
                if candidates.len() >= k {
                    break;
                }
            }

            // 取り出さなかった分を戻す
            for &c in &candidates {
                heap.push(Reverse(c));
            }

            // 各候補について次状態を生成
            for &v in &candidates {
                let mut new_perm = perm.clone();
                new_perm.push(v);

                let mut new_deg = deg.clone();
                let mut new_heap = BinaryHeap::new();

                // 元のヒープから v 以外を追加
                let mut temp = vec![];
                while let Some(Reverse(u)) = heap.pop() {
                    temp.push(u);
                }
                for u in &temp {
                    heap.push(Reverse(*u));
                }

                for u in temp {
                    if u != v {
                        new_heap.push(Reverse(u));
                    }
                }

                // v の出辺を処理
                for &next in &graph[v] {
                    new_deg[next] -= 1;
                    if new_deg[next] == 0 {
                        new_heap.push(Reverse(next));
                    }
                }

                next_states.push((new_perm, new_deg, new_heap));
            }
        }

        // 辞書順でソートしてK個まで保持
        next_states.sort_by(|a, b| a.0.cmp(&b.0));
        next_states.truncate(k);
        states = next_states;
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let edges = vec![(1, 2), (3, 4)];
        let result = solve(5, 2, 3, &edges).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![1, 2, 3, 4, 5]);
        assert_eq!(result[1], vec![1, 3, 2, 4, 5]);
        assert_eq!(result[2], vec![1, 3, 5, 2, 4]);
    }

    #[test]
    fn test_example2() {
        let edges = vec![(1, 3), (3, 1)];
        let result = solve(5, 2, 1, &edges);
        assert!(result.is_none());
    }
}
