// 007 - CP Classes (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_g
//
// 問題: N個のクラス（レーティングA_i）と Q人の生徒（希望レーティングB_j）。
// 各生徒について、|A_i - B_j| の最小値を求めよ。
//
// 解法: 二分探索
// - Aをソートして各クエリで二分探索
// - lower_bound で B_j 以上の最小値を見つけ、その1つ前も確認

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; n],
        q: usize,
        b: [i64; q],
    }
    solve(n, &a, q, &b);
}

fn solve(_n: usize, a: &[i64], _q: usize, b: &[i64]) {
    // ソート
    let mut sorted_a = a.to_vec();
    sorted_a.sort_unstable();

    for &target in b {
        // lower_bound: target 以上の最小インデックス
        let pos = sorted_a.partition_point(|&x| x < target);

        let mut min_diff = i64::MAX;

        // pos の位置（target以上の最小）
        if pos < sorted_a.len() {
            min_diff = min_diff.min((sorted_a[pos] - target).abs());
        }

        // pos-1 の位置（target未満の最大）
        if pos > 0 {
            min_diff = min_diff.min((sorted_a[pos - 1] - target).abs());
        }

        println!("{}", min_diff);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_partition_point() {
        let a: Vec<i64> = vec![1, 3, 5, 7, 9];

        // 4以上の最小 → インデックス2（値5）
        assert_eq!(a.partition_point(|&x| x < 4), 2);

        // 5以上の最小 → インデックス2（値5）
        assert_eq!(a.partition_point(|&x| x < 5), 2);

        // 6以上の最小 → インデックス3（値7）
        assert_eq!(a.partition_point(|&x| x < 6), 3);
    }

    #[test]
    fn test_min_diff() {
        let sorted_a: Vec<i64> = vec![1, 3, 5, 7, 9];

        // target = 4: 候補は 3 と 5 → 差は 1
        let target: i64 = 4;
        let pos = sorted_a.partition_point(|&x| x < target);
        let diff1 = (sorted_a[pos] - target).abs();
        let diff2 = (sorted_a[pos - 1] - target).abs();
        assert_eq!(diff1.min(diff2), 1);
    }
}
