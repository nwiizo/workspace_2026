//! 036 - Max Manhattan Distance (★5)
//!
//! マンハッタン距離 → チェビシェフ距離への変換
//!
//! |x1-x2| + |y1-y2| = max(|(x1+y1)-(x2+y2)|, |(x1-y1)-(x2-y2)|)
//!
//! 座標変換: (x, y) → (x+y, x-y) = (u, v)
//! するとマンハッタン距離がチェビシェフ距離 max(|u1-u2|, |v1-v2|) になる。
//!
//! 各クエリで最大マンハッタン距離を求めるには、
//! u, v それぞれの最大・最小を前計算しておけば O(1) で答えられる。

use proconio::input;

fn main() {
    input! {
        n: usize,
        q: usize,
        points: [(i64, i64); n],
        queries: [usize; q],
    }

    let result = solve(n, &points, &queries);
    for ans in result {
        println!("{}", ans);
    }
}

fn solve(n: usize, points: &[(i64, i64)], queries: &[usize]) -> Vec<i64> {
    // 座標変換: (x, y) → (x+y, x-y)
    let transformed: Vec<(i64, i64)> = points.iter().map(|&(x, y)| (x + y, x - y)).collect();

    // u = x+y, v = x-y の累積 min/max を前計算
    let mut u_min = vec![i64::MAX; n + 1];
    let mut u_max = vec![i64::MIN; n + 1];
    let mut v_min = vec![i64::MAX; n + 1];
    let mut v_max = vec![i64::MIN; n + 1];

    for i in 0..n {
        let (u, v) = transformed[i];
        u_min[i + 1] = u_min[i].min(u);
        u_max[i + 1] = u_max[i].max(u);
        v_min[i + 1] = v_min[i].min(v);
        v_max[i + 1] = v_max[i].max(v);
    }

    queries
        .iter()
        .map(|&qi| {
            let (u, v) = transformed[qi - 1]; // 1-indexed
            // 他の点との最大チェビシェフ距離
            let max_u_diff = (u - u_min[n]).abs().max((u - u_max[n]).abs());
            let max_v_diff = (v - v_min[n]).abs().max((v - v_max[n]).abs());
            max_u_diff.max(max_v_diff)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        let points = vec![(0, 0), (1, 1), (2, 0)];
        let queries = vec![1, 2, 3];
        let result = solve(3, &points, &queries);
        // 点1(0,0)から最遠: 点2(1,1)距離2, 点3(2,0)距離2 → 2
        // 点2(1,1)から最遠: 点1(0,0)距離2, 点3(2,0)距離2 → 2
        // 点3(2,0)から最遠: 点1(0,0)距離2, 点2(1,1)距離2 → 2
        assert_eq!(result, vec![2, 2, 2]);
    }

    #[test]
    fn example2() {
        let points = vec![(0, 0), (3, 0), (0, 4)];
        let queries = vec![1];
        let result = solve(3, &points, &queries);
        // 点1(0,0)から: 点2距離3, 点3距離4 → 4
        assert_eq!(result, vec![4]);
    }

    #[test]
    fn negative_coords() {
        let points = vec![(-5, -5), (5, 5), (0, 0)];
        let queries = vec![1, 2, 3];
        let result = solve(3, &points, &queries);
        // (-5,-5) から (5,5): |10| + |10| = 20
        assert_eq!(result[0], 20);
        assert_eq!(result[1], 20);
        // (0,0) から (-5,-5) or (5,5): 距離10
        assert_eq!(result[2], 10);
    }
}
