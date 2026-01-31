// 011 - Gravy Jobs (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_k
//
// 問題: N個の仕事があり、各仕事は締切D_i、所要時間C_i、報酬S_iを持つ。
//       1つずつ順に仕事をこなし、締切内に終わった仕事の報酬の最大値を求めよ。
//
// 解法: 締切でソートしてDP
//       dp[t] = 時刻tまでに得られる報酬の最大値
//       締切が早い順に処理し、各仕事を「やる/やらない」で更新

use proconio::input;

fn main() {
    input! {
        n: usize,
        jobs: [(usize, usize, i64); n], // (D, C, S)
    }
    println!("{}", solve(&jobs));
}

fn solve(jobs: &[(usize, usize, i64)]) -> i64 {
    // 締切でソート
    let mut jobs = jobs.to_vec();
    jobs.sort_by_key(|&(d, _, _)| d);

    // 最大締切を求める
    let max_d = jobs.iter().map(|&(d, _, _)| d).max().unwrap_or(0);

    // dp[t] = 時刻tまでに得られる報酬の最大値
    let mut dp = vec![0i64; max_d + 1];

    for (deadline, cost, score) in jobs {
        // 逆順に更新（同じ仕事を複数回使わないため）
        for t in (cost..=deadline).rev() {
            dp[t] = dp[t].max(dp[t - cost] + score);
        }
    }

    *dp.iter().max().unwrap_or(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 仕事1: 締切4, 所要2, 報酬10
        // 仕事2: 締切4, 所要3, 報酬15
        // 仕事3: 締切2, 所要1, 報酬5
        // 最適: 仕事3(0-1) + 仕事2(1-4) = 20
        let jobs = vec![(4, 2, 10), (4, 3, 15), (2, 1, 5)];
        assert_eq!(solve(&jobs), 20);
    }

    #[test]
    fn single_job() {
        let jobs = vec![(5, 3, 100)];
        assert_eq!(solve(&jobs), 100);
    }

    #[test]
    fn impossible_job() {
        // 締切より所要時間が長い
        let jobs = vec![(2, 5, 100)];
        assert_eq!(solve(&jobs), 0);
    }
}
