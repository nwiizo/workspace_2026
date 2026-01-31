// 045 - Simple Grouping (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_as
//
// bit DP
// - N ≤ 15 なので 2^15 = 32768 状態
// - cost[S] = 集合Sの最大距離の2乗
// - dp[S][k] = Sをk個のグループに分割したときの最大距離の2乗の最小値
//
// 計算量: O(3^N * K)

use proconio::input;

const INF: i64 = std::i64::MAX;

fn main() {
    input! {
        n: usize,
        k: usize,
        points: [(i64, i64); n],
    }
    println!("{}", solve(n, k, &points));
}

fn solve(n: usize, k: usize, points: &[(i64, i64)]) -> i64 {
    let size = 1 << n;

    // 2点間の距離の2乗
    let dist2 = |i: usize, j: usize| -> i64 {
        let dx = points[i].0 - points[j].0;
        let dy = points[i].1 - points[j].1;
        dx * dx + dy * dy
    };

    // cost[S] = 集合Sの最大距離の2乗
    let mut cost = vec![0i64; size];
    for s in 0..size {
        let mut max_dist = 0i64;
        for i in 0..n {
            if (s >> i) & 1 == 0 {
                continue;
            }
            for j in (i + 1)..n {
                if (s >> j) & 1 == 0 {
                    continue;
                }
                max_dist = max_dist.max(dist2(i, j));
            }
        }
        cost[s] = max_dist;
    }

    // dp[s] = 集合sを複数グループに分割したときの最大距離の最小値
    // グループ数ごとにDPを更新
    let mut dp = vec![INF; size];
    dp[0] = 0;

    // 1グループ目
    for s in 1..size {
        dp[s] = cost[s];
    }

    // 2グループ目以降
    for _ in 2..=k {
        let mut new_dp = vec![INF; size];
        for s in 0..size {
            // 部分集合を列挙
            let mut t = s;
            while t > 0 {
                let rest = s ^ t;
                if dp[rest] != INF {
                    new_dp[s] = new_dp[s].min(dp[rest].max(cost[t]));
                }
                t = (t - 1) & s;
            }
        }
        dp = new_dp;
    }

    dp[size - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let points = vec![(0, 1), (1, 2), (2, 0)];
        assert_eq!(solve(3, 2, &points), 2);
    }

    #[test]
    fn test_example2() {
        let points = vec![(0, 0), (1, 1), (0, 2), (2, 3), (3, 1)];
        assert_eq!(solve(5, 3, &points), 4);
    }

    #[test]
    fn test_example3() {
        let points = vec![
            (0, 3),
            (3, 5),
            (2, 7),
            (9, 0),
            (5, 6),
            (4, 3),
            (7, 8),
            (6, 5),
            (9, 9),
            (2, 1),
        ];
        assert_eq!(solve(10, 4, &points), 20);
    }

    #[test]
    fn test_example4() {
        let points = vec![(0, 0), (500000000, 500000000), (1000000000, 1000000000)];
        assert_eq!(solve(3, 2, &points), 500000000000000000);
    }
}
