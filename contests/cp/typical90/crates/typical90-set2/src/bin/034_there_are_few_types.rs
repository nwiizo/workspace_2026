//! 034 - There Are Few Types of Elements (★4)
//!
//! 尺取り法（Two Pointers / Sliding Window）
//!
//! 長さNの数列から、含まれる値の種類数がK以下となる
//! 連続部分列の最大長を求める。

use proconio::input;
use std::collections::HashMap;

fn main() {
    input! {
        n: usize,
        k: usize,
        a: [i64; n],
    }
    println!("{}", solve(n, k, &a));
}

fn solve(n: usize, k: usize, a: &[i64]) -> usize {
    if k == 0 {
        return 0;
    }

    let mut count: HashMap<i64, usize> = HashMap::new();
    let mut left = 0;
    let mut max_len = 0;

    for right in 0..n {
        // 右端を追加
        *count.entry(a[right]).or_insert(0) += 1;

        // 種類数がKを超えたら左端を縮める
        while count.len() > k {
            let c = count.get_mut(&a[left]).unwrap();
            *c -= 1;
            if *c == 0 {
                count.remove(&a[left]);
            }
            left += 1;
        }

        max_len = max_len.max(right - left + 1);
    }

    max_len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // K=2, 最長は [1,2,1,2,1] で長さ5
        assert_eq!(solve(7, 2, &[1, 2, 1, 2, 3, 3, 1]), 5);
    }

    #[test]
    fn example2() {
        // K=1, 同じ値のみ
        assert_eq!(solve(5, 1, &[1, 2, 3, 4, 5]), 1);
    }

    #[test]
    fn example3() {
        // K=3, 全体でOK
        assert_eq!(solve(5, 3, &[1, 2, 1, 2, 1]), 5);
    }

    #[test]
    fn all_same() {
        assert_eq!(solve(5, 1, &[1, 1, 1, 1, 1]), 5);
    }

    #[test]
    fn k_equals_n() {
        assert_eq!(solve(5, 5, &[1, 2, 3, 4, 5]), 5);
    }
}
