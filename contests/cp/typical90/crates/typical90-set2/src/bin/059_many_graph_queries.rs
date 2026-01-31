// 059 - Many Graph Queries (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_bg
//
// 64ビット並列化でクエリを処理
// 64個のクエリをまとめてBFS/DFSで伝播

use proconio::input;
use proconio::marker::Usize1;

fn main() {
    input! {
        n: usize,
        m: usize,
        q: usize,
        edges: [(Usize1, Usize1); m],
        queries: [(Usize1, Usize1); q],
    }

    // グラフ構築
    let mut graph = vec![vec![]; n];
    for &(x, y) in &edges {
        graph[x].push(y);
    }

    // 結果
    let mut results = vec![false; q];

    // 64個ずつクエリを処理
    for chunk_start in (0..q).step_by(64) {
        let chunk_end = (chunk_start + 64).min(q);
        let _chunk_size = chunk_end - chunk_start;

        // 各頂点のビットマスク
        let mut reachable = vec![0u64; n];

        // 開始点にビットを立てる
        for (i, &(a, _b)) in queries[chunk_start..chunk_end].iter().enumerate() {
            reachable[a] |= 1u64 << i;
        }

        // トポロジカル順序（DAGなので頂点番号順でOK）
        for v in 0..n {
            let mask = reachable[v];
            if mask == 0 {
                continue;
            }
            for &u in &graph[v] {
                reachable[u] |= mask;
            }
        }

        // 結果を収集
        for (i, &(_a, b)) in queries[chunk_start..chunk_end].iter().enumerate() {
            if (reachable[b] >> i) & 1 == 1 {
                results[chunk_start + i] = true;
            }
        }
    }

    // 出力
    for &r in &results {
        println!("{}", if r { "Yes" } else { "No" });
    }
}

#[cfg(test)]
mod tests {
    // テストは手動で実行
}
