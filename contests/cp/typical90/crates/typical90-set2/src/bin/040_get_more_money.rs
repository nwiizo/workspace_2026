//! 040 - Get More Money (★7)
//!
//! プロジェクト選択問題（燃やす埋める問題）
//!
//! N 人の家がある。各家を訪問すると報酬 A[i] が得られる。
//! ただし、訪問にはコスト C がかかる。
//! また、依存関係があり、家 i を訪問するには家 j も訪問する必要がある場合がある。
//!
//! 最大利益を求める。
//!
//! 最小カットで解く。
//! - 始点 S: 「訪問する」側
//! - 終点 T: 「訪問しない」側
//! - S → i: 容量 A[i]（訪問しないと得られない利益）
//! - i → T: 容量 C（訪問するとかかるコスト）
//! - j → i: 容量 ∞（j を訪問しないなら i も訪問できない）
//!
//! 答え = Σ A[i] - 最小カット

use proconio::input;
use proconio::marker::Usize1;

fn main() {
    input! {
        n: usize,
        w: i64,
        a: [i64; n],
        m: usize,
        dependencies: [(Usize1, Usize1); m],
    }
    println!("{}", solve(n, w, &a, &dependencies));
}

fn solve(n: usize, w: i64, a: &[i64], dependencies: &[(usize, usize)]) -> i64 {
    use std::collections::VecDeque;

    // MaxFlow implementation (Dinic)
    struct MaxFlow {
        graph: Vec<Vec<(usize, usize, i64)>>, // (to, rev_idx, cap)
        level: Vec<i32>,
        iter: Vec<usize>,
    }

    impl MaxFlow {
        fn new(n: usize) -> Self {
            Self {
                graph: vec![vec![]; n],
                level: vec![0; n],
                iter: vec![0; n],
            }
        }

        fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
            let from_len = self.graph[from].len();
            let to_len = self.graph[to].len();
            self.graph[from].push((to, to_len, cap));
            self.graph[to].push((from, from_len, 0));
        }

        fn bfs(&mut self, s: usize) {
            self.level.fill(-1);
            let mut queue = VecDeque::new();
            self.level[s] = 0;
            queue.push_back(s);
            while let Some(v) = queue.pop_front() {
                for &(to, _, cap) in &self.graph[v] {
                    if cap > 0 && self.level[to] < 0 {
                        self.level[to] = self.level[v] + 1;
                        queue.push_back(to);
                    }
                }
            }
        }

        fn dfs(&mut self, v: usize, t: usize, f: i64) -> i64 {
            if v == t {
                return f;
            }
            while self.iter[v] < self.graph[v].len() {
                let i = self.iter[v];
                let (to, rev, cap) = self.graph[v][i];
                if cap > 0 && self.level[v] < self.level[to] {
                    let d = self.dfs(to, t, f.min(cap));
                    if d > 0 {
                        self.graph[v][i].2 -= d;
                        self.graph[to][rev].2 += d;
                        return d;
                    }
                }
                self.iter[v] += 1;
            }
            0
        }

        fn max_flow(&mut self, s: usize, t: usize) -> i64 {
            let mut flow = 0;
            loop {
                self.bfs(s);
                if self.level[t] < 0 {
                    return flow;
                }
                self.iter.fill(0);
                loop {
                    let f = self.dfs(s, t, i64::MAX);
                    if f == 0 {
                        break;
                    }
                    flow += f;
                }
            }
        }
    }

    let s = n; // source
    let t = n + 1; // sink
    let mut mf = MaxFlow::new(n + 2);

    let total_profit: i64 = a.iter().sum();

    for (i, &profit) in a.iter().enumerate() {
        // S → i: 訪問の利益
        mf.add_edge(s, i, profit);
        // i → T: 訪問のコスト
        mf.add_edge(i, t, w);
    }

    // 依存関係: j を訪問するなら i も訪問
    // つまり「i を訪問しない」かつ「j を訪問する」は不可
    // → カットで禁止: i → j に ∞ の辺
    // 注: 依存関係の意味を確認
    // (a, b) = 「家 a を訪問するには家 b も訪問する必要がある」
    // = a を選ぶなら b も選ぶ
    // = 「a を選んで b を選ばない」を禁止
    // = a → b に ∞
    const INF: i64 = 1_000_000_000_000_000;
    for &(a_dep, b_dep) in dependencies {
        mf.add_edge(a_dep, b_dep, INF);
    }

    let min_cut = mf.max_flow(s, t);
    total_profit - min_cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // N=3, W=3 (コスト)
        // A = [4, 3, 2]
        // 依存: (1,0) = 家1を訪問するには家0も必要
        // 訪問なし: 利益0
        // 家0のみ: 4-3=1
        // 家1のみ: 不可（0が必要）
        // 家2のみ: 2-3=-1
        // 家0,1: 4+3-6=1
        // 家0,2: 4+2-6=0
        // 家0,1,2: 4+3+2-9=0
        // 最大は 1
        let a = vec![4, 3, 2];
        let deps = vec![(1, 0)];
        assert_eq!(solve(3, 3, &a, &deps), 1);
    }

    #[test]
    fn no_dependencies() {
        // 依存なし、各家を独立に判断
        // A[i] > W なら訪問
        let a = vec![10, 5, 3];
        let deps = vec![];
        // 家0: 10-3=7 ✓
        // 家1: 5-3=2 ✓
        // 家2: 3-3=0 (どちらでも)
        // 最大 7+2+0=9
        assert_eq!(solve(3, 3, &a, &deps), 9);
    }

    #[test]
    fn all_dependent_chain() {
        // 連鎖依存: 2→1→0
        let a = vec![5, 5, 5];
        let deps = vec![(1, 0), (2, 1)];
        // 訪問するなら全部訪問
        // 全訪問: 15-9=6
        // 訪問なし: 0
        assert_eq!(solve(3, 3, &a, &deps), 6);
    }
}
