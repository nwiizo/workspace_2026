//! 動的計画法アルゴリズム

/// 部分列カウントDP
///
/// 文字列 s から target を部分列として作る方法の数を数える
///
/// # Example
/// ```
/// use typical90::dp::count_subsequence;
///
/// let s: Vec<char> = "aattccooddeerr".chars().collect();
/// let target: Vec<char> = "atcoder".chars().collect();
/// // 各文字2つずつなので 2^7 = 128 通り
/// assert_eq!(count_subsequence(&s, &target, 1_000_000_007), 128);
/// ```
pub fn count_subsequence(s: &[char], target: &[char], modulo: i64) -> i64 {
    let n = target.len();
    let mut dp = vec![0i64; n + 1];
    dp[0] = 1;

    for &c in s {
        for i in (0..n).rev() {
            if c == target[i] {
                dp[i + 1] = (dp[i + 1] + dp[i]) % modulo;
            }
        }
    }

    dp[n]
}

/// 累積和（1次元）
pub fn prefix_sum(a: &[i64]) -> Vec<i64> {
    let mut prefix = vec![0; a.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        prefix[i + 1] = prefix[i] + x;
    }
    prefix
}

/// 累積和から区間和を取得
///
/// # Arguments
/// * `prefix` - prefix_sum で作成した累積和配列
/// * `l` - 区間の左端（含む）
/// * `r` - 区間の右端（含まない）
///
/// # Returns
/// a[l..r] の和
pub fn range_sum(prefix: &[i64], l: usize, r: usize) -> i64 {
    prefix[r] - prefix[l]
}

/// 累積和（2次元）
#[allow(clippy::needless_range_loop)]
pub fn prefix_sum_2d(a: &[Vec<i64>]) -> Vec<Vec<i64>> {
    if a.is_empty() {
        return vec![];
    }
    let n = a.len();
    let m = a[0].len();
    let mut sum = vec![vec![0i64; m + 1]; n + 1];

    for i in 0..n {
        for j in 0..m {
            sum[i + 1][j + 1] = sum[i + 1][j] + sum[i][j + 1] - sum[i][j] + a[i][j];
        }
    }
    sum
}

/// 2次元累積和から矩形領域の和を取得
///
/// # Arguments
/// * `sum` - prefix_sum_2d で作成した累積和配列
/// * `r1`, `c1` - 左上の座標（含む）
/// * `r2`, `c2` - 右下の座標（含まない）
pub fn range_sum_2d(sum: &[Vec<i64>], r1: usize, c1: usize, r2: usize, c2: usize) -> i64 {
    sum[r2][c2] - sum[r2][c1] - sum[r1][c2] + sum[r1][c1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_subsequence() {
        let s: Vec<char> = "atcoder".chars().collect();
        let target: Vec<char> = "atcoder".chars().collect();
        assert_eq!(count_subsequence(&s, &target, 1_000_000_007), 1);

        let s: Vec<char> = "aattccooddeerr".chars().collect();
        assert_eq!(count_subsequence(&s, &target, 1_000_000_007), 128);
    }

    #[test]
    fn test_prefix_sum() {
        let a = vec![1, 2, 3, 4, 5];
        let prefix = prefix_sum(&a);
        assert_eq!(prefix, vec![0, 1, 3, 6, 10, 15]);
        assert_eq!(range_sum(&prefix, 1, 4), 9); // 2 + 3 + 4
    }

    #[test]
    fn test_prefix_sum_2d() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let sum = prefix_sum_2d(&a);

        // 全体の和
        assert_eq!(range_sum_2d(&sum, 0, 0, 3, 3), 45);

        // 中央の 5 のみ
        assert_eq!(range_sum_2d(&sum, 1, 1, 2, 2), 5);

        // 右下 2x2
        assert_eq!(range_sum_2d(&sum, 1, 1, 3, 3), 28); // 5+6+8+9
    }
}
