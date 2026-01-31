// 056 - Lucky Bag (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_bd
//
// ============================================================
// 部分和DP + 経路復元
// ============================================================
//
// dp[i][s] = i日目までで合計sになるか
// 各状態で「どちらを選んだか」を記録して復元
//
// 計算量: O(N × S)
//
// ============================================================

use proconio::input;

fn main() {
    input! {
        n: usize,
        s: usize,
        bags: [(usize, usize); n], // (A_i, B_i)
    }

    match solve(n, s, &bags) {
        Some(path) => println!("{}", path),
        None => println!("Impossible"),
    }
}

fn solve(n: usize, s: usize, bags: &[(usize, usize)]) -> Option<String> {
    // dp[i][sum] = Some(選択) or None
    // 選択: true = A, false = B
    let mut dp: Vec<Vec<Option<bool>>> = vec![vec![None; s + 1]; n + 1];
    dp[0][0] = Some(true); // ダミー値

    for i in 0..n {
        let (a, b) = bags[i];
        for sum in 0..=s {
            if dp[i][sum].is_none() {
                continue;
            }

            // バッグAを選ぶ
            if sum + a <= s {
                dp[i + 1][sum + a] = Some(true);
            }

            // バッグBを選ぶ
            if sum + b <= s {
                dp[i + 1][sum + b] = Some(false);
            }
        }
    }

    // 合計Sに到達できるか確認
    if dp[n][s].is_none() {
        return None;
    }

    // 経路復元
    let mut result = Vec::with_capacity(n);
    let mut current_sum = s;

    for i in (0..n).rev() {
        let choice = dp[i + 1][current_sum].unwrap();
        let (a, b) = bags[i];

        if choice {
            result.push('A');
            current_sum -= a;
        } else {
            result.push('B');
            current_sum -= b;
        }
    }

    result.reverse();
    Some(result.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // BAB: 14 + 15 + 5 = 34
        let bags = vec![(3, 14), (15, 9), (26, 5)];
        let result = solve(3, 34, &bags);
        assert!(result.is_some());
        let path = result.unwrap();
        // 合計が34であることを確認
        let sum: usize = path
            .chars()
            .enumerate()
            .map(|(i, c)| if c == 'A' { bags[i].0 } else { bags[i].1 })
            .sum();
        assert_eq!(sum, 34);
    }

    #[test]
    fn test_example2() {
        let bags = vec![(1, 16), (3, 91), (43, 9), (4, 26), (23, 11)];
        let result = solve(5, 77, &bags);
        assert!(result.is_some());
        let path = result.unwrap();
        let sum: usize = path
            .chars()
            .enumerate()
            .map(|(i, c)| if c == 'A' { bags[i].0 } else { bags[i].1 })
            .sum();
        assert_eq!(sum, 77);
    }

    #[test]
    fn test_example3() {
        let bags = vec![(8, 13), (55, 5), (58, 8), (23, 14), (4, 61)];
        let result = solve(5, 59, &bags);
        assert!(result.is_none());
    }
}
