// 066 - Various Arrays (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_bn
//
// 転倒数の期待値 = Σ P(a_i > a_j) for all i < j
//
// 各ペア (i, j) について P(a_i > a_j) を計算

use proconio::input;

fn main() {
    input! {
        n: usize,
        ranges: [(usize, usize); n], // (L_i, R_i)
    }
    println!("{:.12}", solve(n, &ranges));
}

fn solve(n: usize, ranges: &[(usize, usize)]) -> f64 {
    let mut expected = 0.0;

    for i in 0..n {
        for j in (i + 1)..n {
            expected += prob_greater(ranges[i], ranges[j]);
        }
    }

    expected
}

// P(a_i > a_j) を計算
fn prob_greater(range_i: (usize, usize), range_j: (usize, usize)) -> f64 {
    let (l_i, r_i) = range_i;
    let (l_j, r_j) = range_j;

    let size_i = (r_i - l_i + 1) as f64;
    let size_j = (r_j - l_j + 1) as f64;

    let mut count = 0;

    // x > y となるペア数を数える
    for x in l_i..=r_i {
        // y < x となる y の数
        let y_max = x.saturating_sub(1).min(r_j);
        if y_max >= l_j {
            count += y_max - l_j + 1;
        }
    }

    count as f64 / (size_i * size_j)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let ranges = vec![(1, 2), (1, 2)];
        let result = solve(2, &ranges);
        assert!((result - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_example2() {
        let ranges = vec![(3, 3), (1, 1), (4, 4)];
        let result = solve(3, &ranges);
        assert!((result - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_example3() {
        let ranges = vec![
            (1, 10),
            (38, 40),
            (8, 87),
            (2, 9),
            (75, 100),
            (45, 50),
            (89, 92),
            (27, 77),
            (23, 88),
            (62, 81),
        ];
        let result = solve(10, &ranges);
        assert!((result - 13.696758921226).abs() < 1e-6);
    }
}
