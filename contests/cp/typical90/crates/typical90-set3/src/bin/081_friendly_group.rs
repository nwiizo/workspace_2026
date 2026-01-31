// 081 - Friendly Group (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_cc
//
// 身長差 ≤ K かつ 体重差 ≤ K のグループ
// → 身長の最大-最小 ≤ K かつ 体重の最大-最小 ≤ K
// → (身長, 体重) が K×K の矩形内に収まる
//
// 2次元累積和で各矩形内の人数を O(1) で求め、
// 全ての矩形位置を試す

use proconio::input;

fn main() {
    input! {
        n: usize,
        k: usize,
        students: [(usize, usize); n],
    }
    println!("{}", solve(n, k, &students));
}

fn solve(_n: usize, k: usize, students: &[(usize, usize)]) -> usize {
    // 座標の範囲: 1 ~ 5000
    const MAX: usize = 5001;

    // 各座標に何人いるか
    let mut count = vec![vec![0usize; MAX + 1]; MAX + 1];
    for &(a, b) in students {
        count[a][b] += 1;
    }

    // 2次元累積和
    let mut prefix = vec![vec![0usize; MAX + 2]; MAX + 2];
    for i in 1..=MAX {
        for j in 1..=MAX {
            prefix[i][j] = count[i][j] + prefix[i - 1][j] + prefix[i][j - 1] - prefix[i - 1][j - 1];
        }
    }

    // 矩形クエリ: [x1, x2] × [y1, y2] 内の人数
    let range_sum = |x1: usize, y1: usize, x2: usize, y2: usize| -> usize {
        prefix[x2][y2] + prefix[x1 - 1][y1 - 1] - prefix[x2][y1 - 1] - prefix[x1 - 1][y2]
    };

    let mut ans = 0;

    // 全ての (K+1) × (K+1) 矩形を試す
    for x1 in 1..=MAX {
        let x2 = (x1 + k).min(MAX);
        for y1 in 1..=MAX {
            let y2 = (y1 + k).min(MAX);
            ans = ans.max(range_sum(x1, y1, x2, y2));
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let students = vec![(1, 1), (2, 5), (7, 4)];
        assert_eq!(solve(3, 4, &students), 2);
    }

    #[test]
    fn test_example2() {
        let students = vec![(4, 5), (678, 901)];
        assert_eq!(solve(2, 123, &students), 1);
    }

    #[test]
    fn test_example3() {
        let students = vec![
            (20, 20),
            (20, 20),
            (20, 30),
            (20, 40),
            (30, 20),
            (30, 30),
            (40, 20),
        ];
        assert_eq!(solve(7, 10, &students), 5);
    }
}
