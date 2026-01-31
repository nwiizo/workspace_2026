//! 探索アルゴリズム

/// 答えで二分探索（最大化版）
///
/// check(x) が true となる最大の x を返す
///
/// # Example
/// ```
/// use typical90::search::binary_search_max;
///
/// // 10以下で最大のものを探す
/// let result = binary_search_max(0, 100, |x| x <= 10);
/// assert_eq!(result, 10);
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

/// 答えで二分探索（最小化版）
///
/// check(x) が true となる最小の x を返す
///
/// # Example
/// ```
/// use typical90::search::binary_search_min;
///
/// // 10以上で最小のものを探す
/// let result = binary_search_min(0, 100, |x| x >= 10);
/// assert_eq!(result, 10);
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

/// カッコ列の妥当性を検証
///
/// mask の各ビットが 0='(' 1=')' を表す
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
    fn test_parentheses() {
        // "()" = 0b10
        assert!(is_valid_parentheses(0b10, 2));
        // "(())" = 0b1100
        assert!(is_valid_parentheses(0b1100, 4));
        // "()()" = 0b1010
        assert!(is_valid_parentheses(0b1010, 4));
        // ")(" = 0b01
        assert!(!is_valid_parentheses(0b01, 2));
    }
}
