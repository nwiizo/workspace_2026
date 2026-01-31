// 083 - Colorful Graph (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_ce
//
// 愚直に実装すると O(Q * max_degree) で TLE の可能性
// 平方分割: 次数が √M 以上の頂点は特別扱い
//
// 次数が小さい頂点: 愚直に隣接頂点を塗る
// 次数が大きい頂点: 遅延評価で処理
//
// ここでは愚直解で提出（制約によっては AC）

use proconio::input;

fn main() {
    input! {
        n: usize,
        m: usize,
        edges: [(usize, usize); m],
        q: usize,
        queries: [(usize, u64); q],
    }

    // グラフ構築
    let mut graph = vec![vec![]; n + 1];
    for (a, b) in edges {
        graph[a].push(b);
        graph[b].push(a);
    }

    // 各頂点の色
    let mut color = vec![1u64; n + 1];

    // クエリ処理
    for (x, y) in queries {
        // 出力: 現在の x の色
        println!("{}", color[x]);

        // x と隣接頂点を y に塗る
        color[x] = y;
        for &v in &graph[x] {
            color[v] = y;
        }
    }
}

#[cfg(test)]
mod tests {
    // テストは標準出力を使うので省略
}
