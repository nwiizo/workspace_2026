// 032 - AtCoder Ekiden (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_af
//
// 問題: N人のランナー、各人の各区間のタイムが与えられる。
//       相性の悪いペアは隣接できない。最小合計タイムを求めよ。
//
// 解法: 順列全探索
//       N≤10なので O(N!) で間に合う

use proconio::input;

fn main() {
    input! {
        n: usize,
        times: [[i64; n]; n], // times[i][j] = 選手iの区間jのタイム
        m: usize,
        bad_pairs: [(usize, usize); m], // 相性の悪いペア (1-indexed)
    }
    println!("{}", solve(n, &times, &bad_pairs));
}

fn solve(n: usize, times: &[Vec<i64>], bad_pairs: &[(usize, usize)]) -> i64 {
    // 相性の悪いペアをセットに
    let mut is_bad = vec![vec![false; n]; n];
    for &(a, b) in bad_pairs {
        is_bad[a - 1][b - 1] = true;
        is_bad[b - 1][a - 1] = true;
    }

    // 順列全探索
    let mut perm: Vec<usize> = (0..n).collect();
    let mut min_time = i64::MAX;

    loop {
        // 相性チェック
        let mut valid = true;
        for i in 0..n - 1 {
            if is_bad[perm[i]][perm[i + 1]] {
                valid = false;
                break;
            }
        }

        if valid {
            // タイム計算
            let total: i64 = (0..n).map(|i| times[perm[i]][i]).sum();
            min_time = min_time.min(total);
        }

        // 次の順列
        if !next_permutation(&mut perm) {
            break;
        }
    }

    if min_time == i64::MAX { -1 } else { min_time }
}

fn next_permutation<T: Ord>(arr: &mut [T]) -> bool {
    let n = arr.len();
    if n <= 1 {
        return false;
    }

    // 後ろから降順でなくなる位置を探す
    let mut i = n - 1;
    while i > 0 && arr[i - 1] >= arr[i] {
        i -= 1;
    }

    if i == 0 {
        return false;
    }

    // arr[i-1] より大きい最小の要素を後ろから探す
    let mut j = n - 1;
    while arr[j] <= arr[i - 1] {
        j -= 1;
    }

    arr.swap(i - 1, j);
    arr[i..].reverse();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 3人、3区間
        let times = vec![vec![1, 10, 100], vec![10, 1, 100], vec![100, 10, 1]];
        let bad_pairs = vec![];
        // 最適: [0,1,2] → 1+1+1=3
        assert_eq!(solve(3, &times, &bad_pairs), 3);
    }

    #[test]
    fn with_bad_pair() {
        let times = vec![vec![1, 10], vec![10, 1]];
        let bad_pairs = vec![(1, 2)]; // 0と1は隣接不可
        // [0,1]も[1,0]も不可 → -1
        assert_eq!(solve(2, &times, &bad_pairs), -1);
    }

    #[test]
    fn test_next_permutation() {
        let mut arr = vec![1, 2, 3];
        assert!(next_permutation(&mut arr));
        assert_eq!(arr, vec![1, 3, 2]);
        assert!(next_permutation(&mut arr));
        assert_eq!(arr, vec![2, 1, 3]);
    }
}
