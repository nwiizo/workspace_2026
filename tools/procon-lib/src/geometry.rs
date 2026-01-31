//! 2D Geometry
//!
//! - Point and Vector operations
//! - Line and Segment
//! - Convex Hull
//! - Polygon operations

use std::cmp::Ordering;

/// 2D Point/Vector
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Magnitude (length) of vector
    pub fn norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Squared magnitude
    pub fn norm_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Dot product
    pub fn dot(&self, other: &Point) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Cross product (z-component of 3D cross product)
    pub fn cross(&self, other: &Point) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Rotate by angle (radians)
    pub fn rotate(&self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
        }
    }

    /// Unit vector
    pub fn unit(&self) -> Self {
        let n = self.norm();
        Self {
            x: self.x / n,
            y: self.y / n,
        }
    }

    /// Distance to another point
    pub fn dist(&self, other: &Point) -> f64 {
        (*self - *other).norm()
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<f64> for Point {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
        }
    }
}

const EPS: f64 = 1e-9;

/// Sign with epsilon tolerance
fn sign(x: f64) -> i32 {
    if x > EPS {
        1
    } else if x < -EPS {
        -1
    } else {
        0
    }
}

/// Counter-clockwise check
///
/// Returns:
/// - 1 if p0 -> p1 -> p2 is counter-clockwise
/// - -1 if clockwise
/// - 0 if collinear
pub fn ccw(p0: Point, p1: Point, p2: Point) -> i32 {
    let a = p1 - p0;
    let b = p2 - p0;
    sign(a.cross(&b))
}

/// Line segment
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub p1: Point,
    pub p2: Point,
}

impl Segment {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self { p1, p2 }
    }

    /// Check if point is on segment
    pub fn contains(&self, p: Point) -> bool {
        let d1 = (self.p2 - self.p1).cross(&(p - self.p1));
        if sign(d1) != 0 {
            return false;
        }
        let d2 = (p - self.p1).dot(&(self.p2 - self.p1));
        let d3 = (p - self.p2).dot(&(self.p1 - self.p2));
        sign(d2) >= 0 && sign(d3) >= 0
    }

    /// Check if two segments intersect
    pub fn intersects(&self, other: &Segment) -> bool {
        let d1 = ccw(self.p1, self.p2, other.p1);
        let d2 = ccw(self.p1, self.p2, other.p2);
        let d3 = ccw(other.p1, other.p2, self.p1);
        let d4 = ccw(other.p1, other.p2, self.p2);

        if d1 * d2 < 0 && d3 * d4 < 0 {
            return true;
        }

        if d1 == 0 && self.contains(other.p1) {
            return true;
        }
        if d2 == 0 && self.contains(other.p2) {
            return true;
        }
        if d3 == 0 && other.contains(self.p1) {
            return true;
        }
        if d4 == 0 && other.contains(self.p2) {
            return true;
        }

        false
    }

    /// Get intersection point (returns None if parallel or no intersection)
    pub fn intersection(&self, other: &Segment) -> Option<Point> {
        if !self.intersects(other) {
            return None;
        }

        let a = self.p2 - self.p1;
        let b = other.p2 - other.p1;
        let c = other.p1 - self.p1;

        let denom = a.cross(&b);
        if sign(denom) == 0 {
            return None;
        }

        let t = c.cross(&b) / denom;
        Some(self.p1 + a * t)
    }

    /// Distance from point to segment
    pub fn distance_to_point(&self, p: Point) -> f64 {
        let a = self.p2 - self.p1;
        let b = p - self.p1;
        let c = p - self.p2;

        if a.dot(&b) < 0.0 {
            return b.norm();
        }
        if a.dot(&c) > 0.0 {
            return c.norm();
        }

        a.cross(&b).abs() / a.norm()
    }
}

/// Convex Hull (Andrew's monotone chain)
///
/// Returns vertices in counter-clockwise order.
///
/// # Example
/// ```
/// use procon_lib::geometry::{Point, convex_hull};
///
/// let points = vec![
///     Point::new(0.0, 0.0),
///     Point::new(1.0, 1.0),
///     Point::new(2.0, 0.0),
///     Point::new(1.0, -1.0),
///     Point::new(1.0, 0.5),
/// ];
/// let hull = convex_hull(&points);
/// assert_eq!(hull.len(), 4);  // square
/// ```
pub fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut points = points.to_vec();
    let n = points.len();
    if n <= 2 {
        return points;
    }

    // Sort by x, then by y
    points.sort_by(|a, b| {
        let cmp_x = a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal);
        if cmp_x != Ordering::Equal {
            cmp_x
        } else {
            a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal)
        }
    });

    let mut hull = Vec::with_capacity(2 * n);

    // Lower hull
    for &p in &points {
        while hull.len() >= 2 && ccw(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }

    // Upper hull
    let lower_len = hull.len();
    for &p in points.iter().rev().skip(1) {
        while hull.len() > lower_len && ccw(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }

    hull.pop(); // Remove last point (same as first)
    hull
}

/// Polygon area (signed)
///
/// Positive if vertices are counter-clockwise, negative if clockwise.
pub fn polygon_area(vertices: &[Point]) -> f64 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].cross(&vertices[j]);
    }
    area / 2.0
}

/// Check if point is inside polygon
///
/// Returns:
/// - 1 if inside
/// - 0 if on boundary
/// - -1 if outside
pub fn point_in_polygon(p: Point, polygon: &[Point]) -> i32 {
    let n = polygon.len();
    let mut winding = 0;

    for i in 0..n {
        let j = (i + 1) % n;
        let seg = Segment::new(polygon[i], polygon[j]);

        if seg.contains(p) {
            return 0; // On boundary
        }

        let y1 = polygon[i].y;
        let y2 = polygon[j].y;

        if y1 <= p.y {
            if y2 > p.y {
                if ccw(polygon[i], polygon[j], p) > 0 {
                    winding += 1;
                }
            }
        } else if y2 <= p.y {
            if ccw(polygon[i], polygon[j], p) < 0 {
                winding -= 1;
            }
        }
    }

    if winding != 0 {
        1
    } else {
        -1
    }
}

/// Closest pair of points
///
/// # Complexity
/// O(N log N)
///
/// # Example
/// ```
/// use procon_lib::geometry::{Point, closest_pair};
///
/// let points = vec![
///     Point::new(0.0, 0.0),
///     Point::new(1.0, 0.0),
///     Point::new(10.0, 10.0),
/// ];
/// let dist = closest_pair(&points);
/// assert!((dist - 1.0).abs() < 1e-9);
/// ```
pub fn closest_pair(points: &[Point]) -> f64 {
    let mut points = points.to_vec();
    points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));

    fn closest_pair_rec(points: &mut [Point]) -> f64 {
        let n = points.len();
        if n <= 3 {
            let mut min_dist = f64::MAX;
            for i in 0..n {
                for j in i + 1..n {
                    min_dist = min_dist.min(points[i].dist(&points[j]));
                }
            }
            // Sort by y for merge step
            points.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal));
            return min_dist;
        }

        let mid = n / 2;
        let mid_x = points[mid].x;

        let d1 = closest_pair_rec(&mut points[..mid]);
        let d2 = closest_pair_rec(&mut points[mid..]);
        let mut d = d1.min(d2);

        // Merge sorted by y
        let mut temp = Vec::with_capacity(n);
        let (mut i, mut j) = (0, mid);
        while i < mid && j < n {
            if points[i].y < points[j].y {
                temp.push(points[i]);
                i += 1;
            } else {
                temp.push(points[j]);
                j += 1;
            }
        }
        temp.extend_from_slice(&points[i..mid]);
        temp.extend_from_slice(&points[j..n]);
        points.copy_from_slice(&temp);

        // Check strip
        let strip: Vec<Point> = points
            .iter()
            .filter(|p| (p.x - mid_x).abs() < d)
            .copied()
            .collect();

        for i in 0..strip.len() {
            for j in i + 1..strip.len() {
                if strip[j].y - strip[i].y >= d {
                    break;
                }
                d = d.min(strip[i].dist(&strip[j]));
            }
        }

        d
    }

    closest_pair_rec(&mut points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_operations() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);

        let sum = p1 + p2;
        assert!((sum.x - 4.0).abs() < EPS);
        assert!((sum.y - 6.0).abs() < EPS);

        assert!((p1.dot(&p2) - 11.0).abs() < EPS);
        assert!((p1.cross(&p2) - (-2.0)).abs() < EPS);
    }

    #[test]
    fn test_ccw() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(1.0, 0.0);
        let p2 = Point::new(1.0, 1.0);

        assert_eq!(ccw(p0, p1, p2), 1); // CCW
        assert_eq!(ccw(p0, p2, p1), -1); // CW
    }

    #[test]
    fn test_segment_intersection() {
        let s1 = Segment::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0));
        let s2 = Segment::new(Point::new(0.0, 2.0), Point::new(2.0, 0.0));

        assert!(s1.intersects(&s2));

        let inter = s1.intersection(&s2).unwrap();
        assert!((inter.x - 1.0).abs() < EPS);
        assert!((inter.y - 1.0).abs() < EPS);
    }

    #[test]
    fn test_convex_hull() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 0.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 0.0),
        ];
        let hull = convex_hull(&points);
        assert_eq!(hull.len(), 4);
    }

    #[test]
    fn test_polygon_area() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ];
        assert!((polygon_area(&square) - 1.0).abs() < EPS);
    }

    #[test]
    fn test_point_in_polygon() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];

        assert_eq!(point_in_polygon(Point::new(1.0, 1.0), &square), 1);
        assert_eq!(point_in_polygon(Point::new(0.0, 0.0), &square), 0);
        assert_eq!(point_in_polygon(Point::new(3.0, 3.0), &square), -1);
    }

    #[test]
    fn test_closest_pair() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        let dist = closest_pair(&points);
        assert!((dist - 1.0).abs() < EPS);
    }
}
