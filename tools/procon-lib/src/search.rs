//! Search Algorithms
//!
//! - Binary Search variants
//! - Ternary Search

/// Binary search for maximum (finds largest x where f(x) is true)
///
/// # Example
/// ```
/// use procon_lib::search::binary_search_max;
///
/// let result = binary_search_max(0, 100, |x| x <= 42);
/// assert_eq!(result, 42);
/// ```
pub fn binary_search_max<F: Fn(i64) -> bool>(lo: i64, hi: i64, check: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if check(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Binary search for minimum (finds smallest x where f(x) is true)
///
/// # Example
/// ```
/// use procon_lib::search::binary_search_min;
///
/// let result = binary_search_min(0, 100, |x| x >= 42);
/// assert_eq!(result, 42);
/// ```
pub fn binary_search_min<F: Fn(i64) -> bool>(lo: i64, hi: i64, check: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if check(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// Binary search on floating point
///
/// # Example
/// ```
/// use procon_lib::search::binary_search_float;
///
/// // Find sqrt(2)
/// let result = binary_search_float(0.0, 2.0, 1e-9, |x| x * x <= 2.0);
/// assert!((result - std::f64::consts::SQRT_2).abs() < 1e-8);
/// ```
pub fn binary_search_float<F: Fn(f64) -> bool>(lo: f64, hi: f64, eps: f64, check: F) -> f64 {
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..100 {
        if hi - lo < eps {
            break;
        }
        let mid = (lo + hi) / 2.0;
        if check(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Ternary search for unimodal function maximum
///
/// For a function that first increases then decreases.
///
/// # Example
/// ```
/// use procon_lib::search::ternary_search_max;
///
/// // Find maximum of -(x-5)^2 + 10 (peak at x=5)
/// let result = ternary_search_max(0.0, 10.0, 1e-9, |x| -(x - 5.0).powi(2) + 10.0);
/// assert!((result - 5.0).abs() < 1e-6);
/// ```
pub fn ternary_search_max<F: Fn(f64) -> f64>(lo: f64, hi: f64, eps: f64, f: F) -> f64 {
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..100 {
        if hi - lo < eps {
            break;
        }
        let m1 = lo + (hi - lo) / 3.0;
        let m2 = hi - (hi - lo) / 3.0;
        if f(m1) < f(m2) {
            lo = m1;
        } else {
            hi = m2;
        }
    }
    (lo + hi) / 2.0
}

/// Ternary search for unimodal function minimum
///
/// For a function that first decreases then increases (convex).
pub fn ternary_search_min<F: Fn(f64) -> f64>(lo: f64, hi: f64, eps: f64, f: F) -> f64 {
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..100 {
        if hi - lo < eps {
            break;
        }
        let m1 = lo + (hi - lo) / 3.0;
        let m2 = hi - (hi - lo) / 3.0;
        if f(m1) > f(m2) {
            lo = m1;
        } else {
            hi = m2;
        }
    }
    (lo + hi) / 2.0
}

/// Integer ternary search for maximum
pub fn ternary_search_max_int<F: Fn(i64) -> i64>(lo: i64, hi: i64, f: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 2 {
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;
        if f(m1) < f(m2) {
            lo = m1;
        } else {
            hi = m2;
        }
    }

    let mut best_x = lo;
    let mut best_y = f(lo);
    for x in lo + 1..=hi {
        let y = f(x);
        if y > best_y {
            best_y = y;
            best_x = x;
        }
    }
    best_x
}

/// Integer ternary search for minimum
pub fn ternary_search_min_int<F: Fn(i64) -> i64>(lo: i64, hi: i64, f: F) -> i64 {
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 2 {
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;
        if f(m1) > f(m2) {
            lo = m1;
        } else {
            hi = m2;
        }
    }

    let mut best_x = lo;
    let mut best_y = f(lo);
    for x in lo + 1..=hi {
        let y = f(x);
        if y < best_y {
            best_y = y;
            best_x = x;
        }
    }
    best_x
}

/// Check if valid parentheses (for bit representation)
///
/// mask bit i: 0 = '(', 1 = ')'
pub fn is_valid_parentheses(mask: u64, n: usize) -> bool {
    let mut open = 0i32;
    let mut close = 0i32;

    for i in 0..n {
        if (mask >> i) & 1 == 0 {
            open += 1;
        } else {
            close += 1;
        }
        if close > open {
            return false;
        }
    }
    open == close
}

/// Exponential search (for unbounded binary search)
///
/// Useful when the upper bound is unknown.
pub fn exponential_search<F: Fn(i64) -> bool>(check: F) -> i64 {
    let mut hi = 1;
    while check(hi) {
        hi *= 2;
    }
    binary_search_max(hi / 2, hi, check)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_max() {
        assert_eq!(binary_search_max(0, 100, |x| x <= 42), 42);
        assert_eq!(binary_search_max(0, 100, |x| x <= 0), 0);
        assert_eq!(binary_search_max(0, 100, |x| x <= 99), 99);
    }

    #[test]
    fn test_binary_search_min() {
        assert_eq!(binary_search_min(0, 100, |x| x >= 42), 42);
        assert_eq!(binary_search_min(0, 100, |x| x >= 1), 1);
        assert_eq!(binary_search_min(0, 100, |x| x >= 100), 100);
    }

    #[test]
    fn test_binary_search_float() {
        let sqrt2 = binary_search_float(0.0, 2.0, 1e-12, |x| x * x <= 2.0);
        assert!((sqrt2 - std::f64::consts::SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn test_ternary_search_max() {
        let result = ternary_search_max(0.0, 10.0, 1e-9, |x| -(x - 5.0).powi(2) + 10.0);
        assert!((result - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_ternary_search_min() {
        let result = ternary_search_min(0.0, 10.0, 1e-9, |x| (x - 5.0).powi(2));
        assert!((result - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_ternary_search_int() {
        // Maximum of -|x - 50|
        let result = ternary_search_max_int(0, 100, |x| -(x - 50).abs());
        assert_eq!(result, 50);
    }

    #[test]
    fn test_parentheses() {
        // "()" = 0b10
        assert!(is_valid_parentheses(0b10, 2));
        // "(())" = 0b1100
        assert!(is_valid_parentheses(0b1100, 4));
        // ")(" = 0b01
        assert!(!is_valid_parentheses(0b01, 2));
    }

    #[test]
    fn test_exponential_search() {
        let result = exponential_search(|x| x <= 12345);
        assert_eq!(result, 12345);
    }
}
