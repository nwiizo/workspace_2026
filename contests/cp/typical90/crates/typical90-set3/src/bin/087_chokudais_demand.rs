// 087 - Chokudai's Demand (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_ci
//
// X を大きくすると、-1 の道のコストが上がり、ペア数は減る（単調減少）
// X を小さくすると、-1 の道のコストが下がり、ペア数は増える（単調増加）
//
// X の値でペア数が K になる範囲を二分探索で求める
// Floyd-Warshall で全点対最短距離を計算

use proconio::input;

fn main() {
    input! {
        n: usize,
        p: i64,
        k: usize,
        a: [[i64; n]; n],
    }

    match solve(n, p, k, &a) {
        Answer::Zero => println!("0"),
        Answer::Infinity => println!("Infinity"),
        Answer::Count(c) => println!("{}", c),
    }
}

enum Answer {
    Zero,
    Infinity,
    Count(i64),
}

fn solve(n: usize, p: i64, k: usize, a: &[Vec<i64>]) -> Answer {
    // X の候補範囲: 1 から 10^9 まで

    // count_pairs(x) = X = x のときに距離 ≤ P のペア数
    let count_pairs = |x: i64| -> usize {
        // Floyd-Warshall
        let mut dist = vec![vec![i64::MAX / 2; n]; n];

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    dist[i][j] = 0;
                } else if a[i][j] == -1 {
                    dist[i][j] = x;
                } else {
                    dist[i][j] = a[i][j];
                }
            }
        }

        for mid in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if dist[i][mid] + dist[mid][j] < dist[i][j] {
                        dist[i][j] = dist[i][mid] + dist[mid][j];
                    }
                }
            }
        }

        // 距離 ≤ P のペア数 (i < j)
        let mut cnt = 0;
        for i in 0..n {
            for j in (i + 1)..n {
                if dist[i][j] <= p {
                    cnt += 1;
                }
            }
        }
        cnt
    };

    // X = 1 のとき（最小）
    let max_pairs = count_pairs(1);
    // X = 10^9 のとき（最大）
    let min_pairs = count_pairs(1_000_000_000);

    // K がこの範囲外なら 0
    if k > max_pairs || k < min_pairs {
        return Answer::Zero;
    }

    // X = 10^9 でも K ペア以上あるなら Infinity
    if k == min_pairs {
        // 任意の X ≥ 10^9 で成り立つ可能性
        // より厳密には、-1 がなければ Infinity
        // ここでは X = 10^9 + 1 でも確認
        let pairs_large = count_pairs(2_000_000_000);
        if pairs_large == k {
            return Answer::Infinity;
        }
    }

    // 二分探索で X の範囲を求める
    // count_pairs(x) >= k となる最大の x を求める
    let mut lo = 1i64;
    let mut hi = 1_000_000_001i64;

    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if count_pairs(mid) >= k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let upper = lo;

    // count_pairs(x) > k となる最大の x を求める
    // つまり count_pairs(x) >= k + 1 となる最大の x
    let mut lo = 0i64;
    let mut hi = 1_000_000_001i64;

    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if count_pairs(mid) > k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let lower = lo + 1; // count_pairs が exactly k になる最小の x

    if upper >= lower {
        Answer::Count(upper - lower + 1)
    } else {
        Answer::Zero
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![vec![0, 3, -1], vec![3, 0, 5], vec![-1, 5, 0]];
        match solve(3, 4, 2, &a) {
            Answer::Count(c) => assert_eq!(c, 3),
            _ => panic!("Expected Count"),
        }
    }

    #[test]
    fn test_example2() {
        let a = vec![vec![0, -1, 10], vec![-1, 0, 1], vec![10, 1, 0]];
        match solve(3, 10, 2, &a) {
            Answer::Infinity => (),
            _ => panic!("Expected Infinity"),
        }
    }
}
