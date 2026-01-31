// 001 - Yokan Party (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_a
//
// 問題: 長さLのようかんをK+1個に分ける。N箇所の切れ目候補がある。
//       最小ピースの長さを最大化せよ。
//
// 解法: 答えで二分探索
//       「最小ピースがx以上で分けられるか？」を貪欲に判定

use proconio::input;

fn main() {
    input! {
        n: usize,
        l: i64,
        k: usize,
        a: [i64; n],
    }
    println!("{}", solve(l, k, &a));
}

/// 最小ピースの長さ `min_len` で K+1 個以上に分けられるか判定
fn can_divide(a: &[i64], l: i64, k: usize, min_len: i64) -> bool {
    let pieces = a
        .iter()
        .chain(std::iter::once(&l)) // 右端を追加
        .fold((0usize, 0i64), |(count, prev), &pos| {
            if pos - prev >= min_len {
                (count + 1, pos)
            } else {
                (count, prev)
            }
        })
        .0;
    pieces > k
}

fn solve(l: i64, k: usize, a: &[i64]) -> i64 {
    // 二分探索: can_divide が true となる最大の min_len を求める
    let (mut lo, mut hi) = (0i64, l + 1);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if can_divide(a, l, k, mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        assert_eq!(solve(34, 1, &[8, 13, 26]), 13);
    }

    #[test]
    fn example2() {
        assert_eq!(solve(34, 2, &[8, 13, 26]), 8);
    }

    #[test]
    fn edge_case_single_cut() {
        // 1箇所だけで切る場合
        assert_eq!(solve(10, 1, &[5]), 5);
    }
}
