//! Dynamic Programming Algorithms
//!
//! - LIS (Longest Increasing Subsequence)
//! - LCS (Longest Common Subsequence)
//! - Edit Distance
//! - Prefix Sum (1D and 2D)
//! - Knapsack

/// Longest Increasing Subsequence
///
/// # Complexity
/// O(N log N)
///
/// # Example
/// ```
/// use procon_lib::dp::lis;
///
/// let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
/// assert_eq!(lis(&a, true), 4);   // strict: [1, 4, 5, 9] or [1, 4, 5, 6]
/// assert_eq!(lis(&a, false), 4);  // non-strict
///
/// let b = vec![1, 1, 1, 1, 1];
/// assert_eq!(lis(&b, true), 1);   // strict: only 1 element
/// assert_eq!(lis(&b, false), 5);  // non-strict: all elements
/// ```
pub fn lis<T: Ord + Clone>(a: &[T], strict: bool) -> usize {
    let mut dp: Vec<T> = Vec::new();

    for x in a {
        let pos = if strict {
            dp.partition_point(|y| y < x)
        } else {
            dp.partition_point(|y| y <= x)
        };

        if pos == dp.len() {
            dp.push(x.clone());
        } else {
            dp[pos] = x.clone();
        }
    }

    dp.len()
}

/// LIS with reconstruction
///
/// # Returns
/// Indices of elements in the LIS
pub fn lis_restore<T: Ord + Clone>(a: &[T], strict: bool) -> Vec<usize> {
    let n = a.len();
    if n == 0 {
        return vec![];
    }

    let mut dp: Vec<T> = Vec::new();
    let mut pos_in_dp = vec![0; n];

    for (i, x) in a.iter().enumerate() {
        let pos = if strict {
            dp.partition_point(|y| y < x)
        } else {
            dp.partition_point(|y| y <= x)
        };

        if pos == dp.len() {
            dp.push(x.clone());
        } else {
            dp[pos] = x.clone();
        }
        pos_in_dp[i] = pos;
    }

    let lis_len = dp.len();
    let mut result = vec![0; lis_len];
    let mut current_pos = lis_len - 1;

    for i in (0..n).rev() {
        if pos_in_dp[i] == current_pos {
            result[current_pos] = i;
            if current_pos == 0 {
                break;
            }
            current_pos -= 1;
        }
    }

    result
}

/// Longest Common Subsequence length
///
/// # Complexity
/// O(NM)
///
/// # Example
/// ```
/// use procon_lib::dp::lcs_length;
///
/// let a: Vec<char> = "abcde".chars().collect();
/// let b: Vec<char> = "ace".chars().collect();
/// assert_eq!(lcs_length(&a, &b), 3);  // "ace"
/// ```
pub fn lcs_length<T: Eq>(a: &[T], b: &[T]) -> usize {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 0..n {
        for j in 0..m {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    dp[n][m]
}

/// LCS with reconstruction
pub fn lcs<T: Eq + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 0..n {
        for j in 0..m {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut result = Vec::new();
    let mut i = n;
    let mut j = m;

    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1].clone());
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

/// Edit Distance (Levenshtein Distance)
///
/// # Complexity
/// O(NM)
///
/// # Example
/// ```
/// use procon_lib::dp::edit_distance;
///
/// let a: Vec<char> = "kitten".chars().collect();
/// let b: Vec<char> = "sitting".chars().collect();
/// assert_eq!(edit_distance(&a, &b), 3);
/// ```
pub fn edit_distance<T: Eq>(a: &[T], b: &[T]) -> usize {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().take(m + 1) {
        *val = j;
    }

    for i in 0..n {
        for j in 0..m {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j];
            } else {
                dp[i + 1][j + 1] = dp[i][j].min(dp[i + 1][j]).min(dp[i][j + 1]) + 1;
            }
        }
    }

    dp[n][m]
}

/// 1D Prefix Sum
///
/// # Example
/// ```
/// use procon_lib::dp::{prefix_sum, range_sum};
///
/// let a = vec![1, 2, 3, 4, 5];
/// let prefix = prefix_sum(&a);
/// assert_eq!(range_sum(&prefix, 1, 4), 9);  // 2 + 3 + 4
/// ```
pub fn prefix_sum(a: &[i64]) -> Vec<i64> {
    let mut prefix = vec![0; a.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        prefix[i + 1] = prefix[i] + x;
    }
    prefix
}

/// Get range sum [l, r) using prefix sum
pub fn range_sum(prefix: &[i64], l: usize, r: usize) -> i64 {
    prefix[r] - prefix[l]
}

/// 2D Prefix Sum
///
/// # Example
/// ```
/// use procon_lib::dp::{prefix_sum_2d, range_sum_2d};
///
/// let a = vec![
///     vec![1, 2, 3],
///     vec![4, 5, 6],
///     vec![7, 8, 9],
/// ];
/// let sum = prefix_sum_2d(&a);
/// assert_eq!(range_sum_2d(&sum, 1, 1, 3, 3), 28);  // 5+6+8+9
/// ```
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

/// Get 2D range sum [r1, r2) x [c1, c2)
pub fn range_sum_2d(sum: &[Vec<i64>], r1: usize, c1: usize, r2: usize, c2: usize) -> i64 {
    sum[r2][c2] - sum[r2][c1] - sum[r1][c2] + sum[r1][c1]
}

/// 0-1 Knapsack
///
/// # Returns
/// Maximum value achievable
///
/// # Complexity
/// O(N * W)
///
/// # Example
/// ```
/// use procon_lib::dp::knapsack_01;
///
/// let items = vec![(2, 3), (3, 4), (4, 5)];  // (weight, value)
/// assert_eq!(knapsack_01(5, &items), 7);  // items 0 and 1: weight=5, value=7
/// ```
pub fn knapsack_01(capacity: usize, items: &[(usize, i64)]) -> i64 {
    let mut dp = vec![0i64; capacity + 1];

    for &(weight, value) in items {
        for w in (weight..=capacity).rev() {
            dp[w] = dp[w].max(dp[w - weight] + value);
        }
    }

    dp[capacity]
}

/// Unbounded Knapsack
///
/// # Returns
/// Maximum value achievable (can use each item unlimited times)
pub fn knapsack_unbounded(capacity: usize, items: &[(usize, i64)]) -> i64 {
    let mut dp = vec![0i64; capacity + 1];

    for w in 1..=capacity {
        for &(weight, value) in items {
            if weight <= w {
                dp[w] = dp[w].max(dp[w - weight] + value);
            }
        }
    }

    dp[capacity]
}

/// Count subsequence occurrences
///
/// Count how many ways to form `target` as subsequence of `s`.
///
/// # Example
/// ```
/// use procon_lib::dp::count_subsequence;
///
/// let s: Vec<char> = "aattccooddeerr".chars().collect();
/// let target: Vec<char> = "atcoder".chars().collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lis() {
        let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(lis(&a, true), 4);
        assert_eq!(lis(&a, false), 4);

        let b = vec![1, 1, 1, 1, 1];
        assert_eq!(lis(&b, true), 1);
        assert_eq!(lis(&b, false), 5);
    }

    #[test]
    fn test_lis_restore() {
        let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let indices = lis_restore(&a, true);
        assert_eq!(indices.len(), 4);

        for i in 1..indices.len() {
            assert!(a[indices[i - 1]] < a[indices[i]]);
        }
    }

    #[test]
    fn test_lcs() {
        let a: Vec<char> = "abcde".chars().collect();
        let b: Vec<char> = "ace".chars().collect();
        assert_eq!(lcs_length(&a, &b), 3);
        assert_eq!(lcs(&a, &b), vec!['a', 'c', 'e']);
    }

    #[test]
    fn test_edit_distance() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(edit_distance(&a, &b), 3);
    }

    #[test]
    fn test_prefix_sum() {
        let a = vec![1, 2, 3, 4, 5];
        let prefix = prefix_sum(&a);
        assert_eq!(prefix, vec![0, 1, 3, 6, 10, 15]);
        assert_eq!(range_sum(&prefix, 1, 4), 9);
    }

    #[test]
    fn test_prefix_sum_2d() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let sum = prefix_sum_2d(&a);
        assert_eq!(range_sum_2d(&sum, 0, 0, 3, 3), 45);
        assert_eq!(range_sum_2d(&sum, 1, 1, 3, 3), 28);
    }

    #[test]
    fn test_knapsack_01() {
        let items = vec![(2, 3), (3, 4), (4, 5)];
        assert_eq!(knapsack_01(5, &items), 7);
    }

    #[test]
    fn test_knapsack_unbounded() {
        let items = vec![(2, 3), (3, 4)];
        // With capacity 6: use (2,3) three times = 9
        assert_eq!(knapsack_unbounded(6, &items), 9);
    }

    #[test]
    fn test_count_subsequence() {
        let s: Vec<char> = "aattccooddeerr".chars().collect();
        let target: Vec<char> = "atcoder".chars().collect();
        assert_eq!(count_subsequence(&s, &target, 1_000_000_007), 128);
    }
}
