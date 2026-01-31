// 041 - Piles in AtCoder Farm (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_ao
//
// 凸包 + Pickの定理
//
// 1. 凸包を構築（Andrew's Monotone Chain）
// 2. Pickの定理: A = i + b/2 - 1
//    - A: 面積（外積で計算）
//    - b: 境界上の格子点数（辺ごとに gcd(|dx|, |dy|)）
//    - i: 内部の格子点数 = A - b/2 + 1
// 3. 答え = (i + b) - N

use proconio::input;

fn main() {
    input! {
        n: usize,
        points: [(i64, i64); n],
    }
    println!("{}", solve(&points));
}

fn solve(points: &[(i64, i64)]) -> i64 {
    let hull = convex_hull(points);

    if hull.len() < 3 {
        // 凸包が退化（直線上）
        return 0;
    }

    // 面積の2倍を計算（整数で保持）
    let area2 = polygon_area2(&hull);

    // 境界上の格子点数
    let boundary_points = count_boundary_points(&hull);

    // Pickの定理: A = i + b/2 - 1
    // 2A = 2i + b - 2
    // 2i = 2A - b + 2
    // i = (2A - b + 2) / 2
    let interior_points = (area2 - boundary_points + 2) / 2;

    // 全格子点数 - 既存のN本
    (interior_points + boundary_points) - points.len() as i64
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// 凸包構築（Andrew's Monotone Chain）
fn convex_hull(points: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut pts: Vec<(i64, i64)> = points.to_vec();
    pts.sort();
    pts.dedup();

    if pts.len() <= 2 {
        return pts;
    }

    let n = pts.len();
    let mut hull = Vec::with_capacity(2 * n);

    // 下側凸包
    for &p in &pts {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }

    // 上側凸包
    let lower_len = hull.len();
    for &p in pts.iter().rev().skip(1) {
        while hull.len() > lower_len && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }

    hull.pop(); // 最後の点は重複
    hull
}

// 外積 (b - a) × (c - a)
fn cross(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> i64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

// 多角形の面積の2倍
fn polygon_area2(polygon: &[(i64, i64)]) -> i64 {
    let n = polygon.len();
    let mut area2 = 0i64;
    for i in 0..n {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % n];
        area2 += x1 * y2 - x2 * y1;
    }
    area2.abs()
}

// 境界上の格子点数
fn count_boundary_points(polygon: &[(i64, i64)]) -> i64 {
    let n = polygon.len();
    let mut count = 0i64;
    for i in 0..n {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % n];
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        count += gcd(dx, dy);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let points = vec![(1, 4), (6, 1), (5, 8)];
        assert_eq!(solve(&points), 17);
    }

    #[test]
    fn test_example2() {
        let points = vec![(2, 2), (2, 3), (3, 2)];
        assert_eq!(solve(&points), 0);
    }

    #[test]
    fn test_example3() {
        let points = vec![(2, 39), (39, 35), (17, 5)];
        assert_eq!(solve(&points), 599);
    }
}
