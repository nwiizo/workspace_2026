// 073 - We Need Both a and b (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_bu
//
// 木DP: 各部分木について「aとb両方を含む」状態での辺の切り方を数える
// dp[v][0] = 部分木vが 'a' のみを含む切り方の数
// dp[v][1] = 部分木vが 'b' のみを含む切り方の数
// dp[v][2] = 部分木vが 'a' と 'b' 両方を含む切り方の数
//
// 辺を切る場合：子の部分木が両方含む必要がある
// 辺を切らない場合：親に含める

use proconio::input;
use proconio::marker::Chars;

const MOD: u64 = 1_000_000_007;

fn main() {
    input! {
        n: usize,
        c: Chars,
        edges: [(usize, usize); n - 1],
    }

    // 木を構築
    let mut graph = vec![vec![]; n];
    for (a, b) in edges {
        let a = a - 1;
        let b = b - 1;
        graph[a].push(b);
        graph[b].push(a);
    }

    // 各頂点の文字: 0='a', 1='b'
    let chars: Vec<usize> = c.iter().map(|&ch| if ch == 'a' { 0 } else { 1 }).collect();

    // dp[v][0] = 'a'のみ, dp[v][1] = 'b'のみ, dp[v][2] = 両方
    let mut dp = vec![[0u64; 3]; n];

    // DFSで木DP
    fn dfs(v: usize, parent: usize, graph: &[Vec<usize>], chars: &[usize], dp: &mut [[u64; 3]]) {
        // 初期状態: この頂点の文字のみ
        if chars[v] == 0 {
            dp[v][0] = 1;
            dp[v][1] = 0;
            dp[v][2] = 0;
        } else {
            dp[v][0] = 0;
            dp[v][1] = 1;
            dp[v][2] = 0;
        }

        for &u in &graph[v] {
            if u == parent {
                continue;
            }

            dfs(u, v, graph, chars, dp);

            // 子 u を統合
            // 辺を切る場合: dp[u][2] 通り（子は両方含む必要がある）
            // 辺を切らない場合: dp[u] の状態を親に加える

            let mut new_dp = [0u64; 3];

            // 現在の dp[v] と dp[u] を組み合わせる
            for i in 0..3 {
                for j in 0..3 {
                    // 辺を切らない場合
                    // i と j の和集合の状態になる
                    let merged = match (i, j) {
                        (0, 0) => 0,
                        (1, 1) => 1,
                        (0, 1) | (1, 0) | (0, 2) | (2, 0) | (1, 2) | (2, 1) | (2, 2) => 2,
                        _ => unreachable!(),
                    };
                    new_dp[merged] = (new_dp[merged] + dp[v][i] * dp[u][j]) % MOD;
                }
            }

            // 辺を切る場合: dp[v] はそのまま、dp[u][2] を掛ける
            for i in 0..3 {
                new_dp[i] = (new_dp[i] + dp[v][i] * dp[u][2]) % MOD;
            }

            dp[v] = new_dp;
        }
    }

    dfs(0, n, &graph, &chars, &mut dp);

    // 答えは dp[0][2]（根の部分木全体が両方含む）
    println!("{}", dp[0][2]);
}

#[cfg(test)]
mod tests {
    // テストは複雑なので省略
}
