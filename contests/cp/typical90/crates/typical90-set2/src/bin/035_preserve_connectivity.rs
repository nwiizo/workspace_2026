// 035 - Preserve Connectivity (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_ai
//
// Steiner Tree on Tree
// 木上で指定頂点群を連結に保つ最小辺数を求める
//
// アルゴリズム:
// 1. LCA前計算（ダブリング）
// 2. 頂点群をDFS順（オイラーツアー順）でソート
// 3. 隣接頂点間の距離を全て足し、最初と最後も足す
// 4. 合計を2で割る
//
// 答え = Σ dist(v_i, v_{i+1}) / 2  (v_{k+1} = v_1)
// dist(u,v) = depth[u] + depth[v] - 2*depth[LCA(u,v)]

use proconio::input;
use proconio::marker::Usize1;

const LOG: usize = 17; // 2^17 > 10^5

fn main() {
    input! {
        n: usize,
        edges: [(Usize1, Usize1); n - 1],
        q: usize,
    }

    // グラフ構築
    let mut graph = vec![vec![]; n];
    for &(a, b) in &edges {
        graph[a].push(b);
        graph[b].push(a);
    }

    // LCA前計算
    let mut depth = vec![0usize; n];
    let mut euler_tour = vec![0usize; n]; // DFS順
    let mut parent = vec![vec![0usize; n]; LOG];

    // DFS
    let mut order = 0;
    let mut stack = vec![(0, 0, false)]; // (node, parent, visited)
    while let Some((v, p, visited)) = stack.pop() {
        if visited {
            continue;
        }
        euler_tour[v] = order;
        order += 1;
        depth[v] = if v == 0 { 0 } else { depth[p] + 1 };
        parent[0][v] = p;

        stack.push((v, p, true));
        for &u in &graph[v] {
            if u != p {
                stack.push((u, v, false));
            }
        }
    }

    // ダブリング
    for k in 1..LOG {
        for v in 0..n {
            parent[k][v] = parent[k - 1][parent[k - 1][v]];
        }
    }

    // LCA関数
    let lca = |mut u: usize, mut v: usize| -> usize {
        if depth[u] > depth[v] {
            std::mem::swap(&mut u, &mut v);
        }
        let diff = depth[v] - depth[u];
        for k in 0..LOG {
            if (diff >> k) & 1 == 1 {
                v = parent[k][v];
            }
        }
        if u == v {
            return u;
        }
        for k in (0..LOG).rev() {
            if parent[k][u] != parent[k][v] {
                u = parent[k][u];
                v = parent[k][v];
            }
        }
        parent[0][u]
    };

    // 距離関数
    let dist = |u: usize, v: usize| -> usize {
        let l = lca(u, v);
        depth[u] + depth[v] - 2 * depth[l]
    };

    // クエリ処理
    for _ in 0..q {
        input! {
            k: usize,
            vertices: [Usize1; k],
        }

        // オイラーツアー順でソート
        let mut sorted: Vec<usize> = vertices;
        sorted.sort_by_key(|&v| euler_tour[v]);

        // 隣接頂点間の距離の総和
        let mut total = 0usize;
        for i in 0..sorted.len() {
            let u = sorted[i];
            let v = sorted[(i + 1) % sorted.len()];
            total += dist(u, v);
        }

        // 答えは総和の半分
        println!("{}", total / 2);
    }
}

#[cfg(test)]
mod tests {
    // テストは手動実行
}
