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

/// 最長増加部分列 (LIS: Longest Increasing Subsequence)
///
/// 二分探索を用いて O(N log N) で計算
///
/// # Arguments
/// * `a` - 入力列
/// * `strict` - true なら狭義単調増加、false なら広義単調増加
///
/// # Returns
/// LIS の長さ
///
/// # Example
/// ```
/// use typical90::dp::lis;
///
/// let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
/// assert_eq!(lis(&a, true), 4);  // [1, 4, 5, 9] or [1, 4, 5, 6] など
/// assert_eq!(lis(&a, false), 4); // 広義でも4 (この例では差が出ない)
///
/// // 広義単調増加で長くなるケース
/// let b = vec![1, 1, 1, 1, 1];
/// assert_eq!(lis(&b, true), 1);   // 狭義: 重複不可
/// assert_eq!(lis(&b, false), 5);  // 広義: 全部選べる
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

/// 最長増加部分列を復元
///
/// # Returns
/// LIS を構成するインデックスの列
///
/// # Example
/// ```
/// use typical90::dp::lis_restore;
///
/// let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
/// let indices = lis_restore(&a, true);
/// assert_eq!(indices.len(), 4);
/// // インデックス列が増加部分列を構成していることを確認
/// for i in 1..indices.len() {
///     assert!(a[indices[i-1]] < a[indices[i]]);
/// }
/// ```
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

/// 最長共通部分列 (LCS: Longest Common Subsequence)
///
/// O(NM) の動的計画法で計算
///
/// # Returns
/// LCS の長さ
///
/// # Example
/// ```
/// use typical90::dp::lcs_length;
///
/// let a = "abcde".chars().collect::<Vec<_>>();
/// let b = "ace".chars().collect::<Vec<_>>();
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

/// 最長共通部分列を復元
///
/// # Returns
/// LCS の文字列（または要素列）
///
/// # Example
/// ```
/// use typical90::dp::lcs;
///
/// let a = "abcde".chars().collect::<Vec<_>>();
/// let b = "ace".chars().collect::<Vec<_>>();
/// let result = lcs(&a, &b);
/// assert_eq!(result, vec!['a', 'c', 'e']);
/// ```
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

    // 復元
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

/// 編集距離 (Levenshtein Distance)
///
/// # Returns
/// a を b に変換するのに必要な最小操作数（挿入・削除・置換）
///
/// # Example
/// ```
/// use typical90::dp::edit_distance;
///
/// let a = "kitten".chars().collect::<Vec<_>>();
/// let b = "sitting".chars().collect::<Vec<_>>();
/// assert_eq!(edit_distance(&a, &b), 3);  // k->s, e->i, +g
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lis() {
        let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(lis(&a, true), 4); // [1, 4, 5, 9] など
        assert_eq!(lis(&a, false), 4); // [1, 1, 2, 6] など（広義単調増加でも4が最大）

        let a = vec![1, 2, 3, 4, 5];
        assert_eq!(lis(&a, true), 5);

        let a = vec![5, 4, 3, 2, 1];
        assert_eq!(lis(&a, true), 1);

        let empty: Vec<i32> = vec![];
        assert_eq!(lis(&empty, true), 0);

        // 広義単調増加で長くなるケース
        let b = vec![1, 1, 1, 1, 1];
        assert_eq!(lis(&b, true), 1); // 狭義では1つのみ
        assert_eq!(lis(&b, false), 5); // 広義では全部選べる

        // 狭義と広義で差が出るケース
        let c = vec![1, 2, 2, 3, 3, 4];
        assert_eq!(lis(&c, true), 4); // [1, 2, 3, 4]
        assert_eq!(lis(&c, false), 6); // [1, 2, 2, 3, 3, 4] 全部
    }

    #[test]
    fn test_lis_restore() {
        let a = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let indices = lis_restore(&a, true);
        assert_eq!(indices.len(), 4);
        // 実際に増加部分列になっていることを確認
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

        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "def".chars().collect();
        assert_eq!(lcs_length(&a, &b), 0);
        assert_eq!(lcs(&a, &b), Vec::<char>::new());
    }

    #[test]
    fn test_edit_distance() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(edit_distance(&a, &b), 3);

        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "abc".chars().collect();
        assert_eq!(edit_distance(&a, &b), 0);
    }

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
