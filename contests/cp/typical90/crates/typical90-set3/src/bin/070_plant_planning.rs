// 070 - Plant Planning (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_br
//
// マンハッタン距離の和を最小化
// x座標とy座標は独立に考えられる
// |x - x_i| の総和を最小化 → 中央値を選ぶ
//
// 中央値: ソートして真ん中の値

use proconio::input;

fn main() {
    input! {
        n: usize,
        points: [(i64, i64); n],
    }
    println!("{}", solve(n, &points));
}

fn solve(n: usize, points: &[(i64, i64)]) -> i64 {
    let mut xs: Vec<i64> = points.iter().map(|&(x, _)| x).collect();
    let mut ys: Vec<i64> = points.iter().map(|&(_, y)| y).collect();

    xs.sort();
    ys.sort();

    // 中央値
    let median_x = xs[n / 2];
    let median_y = ys[n / 2];

    // 総距離
    let sum_x: i64 = xs.iter().map(|&x| (x - median_x).abs()).sum();
    let sum_y: i64 = ys.iter().map(|&y| (y - median_y).abs()).sum();

    sum_x + sum_y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let points = vec![(-1, 2), (1, 1)];
        assert_eq!(solve(2, &points), 3);
    }

    #[test]
    fn test_example2() {
        let points = vec![(1, 0), (0, 1)];
        assert_eq!(solve(2, &points), 2);
    }

    #[test]
    fn test_example3() {
        let points = vec![(2, 5), (2, 5), (-3, 4), (-4, -8), (6, -2)];
        assert_eq!(solve(5, &points), 35);
    }

    #[test]
    fn test_example4() {
        let points = vec![
            (1_000_000_000, 1_000_000_000),
            (-1_000_000_000, 1_000_000_000),
            (-1_000_000_000, -1_000_000_000),
            (1_000_000_000, -1_000_000_000),
        ];
        assert_eq!(solve(4, &points), 8_000_000_000);
    }
}
