//! Palindrome Algorithms
//!
//! - Manacher's algorithm

/// Manacher's algorithm for finding palindromes
///
/// Returns array where:
/// - radius[2*i] = radius of palindrome centered between s[i-1] and s[i]
/// - radius[2*i+1] = radius of palindrome centered at s[i]
///
/// Radius means the number of characters on each side (excluding center for odd).
///
/// # Complexity
/// O(N)
///
/// # Example
/// ```
/// use procon_lib::palindrome::manacher;
///
/// let s = b"abacaba";
/// let radius = manacher(s);
///
/// // Longest palindrome centered at index 3 ('c')
/// // "abacaba" has radius 3 (extends 3 chars each side)
/// assert_eq!(radius[7], 3);  // 2*3+1 = 7 is center of 'c'
/// ```
pub fn manacher(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    // Transform: "abc" -> "^#a#b#c#$"
    let mut t = Vec::with_capacity(2 * n + 3);
    t.push(b'^');
    for &c in s {
        t.push(b'#');
        t.push(c);
    }
    t.push(b'#');
    t.push(b'$');

    let m = t.len();
    let mut p = vec![0usize; m];
    let mut c = 0;
    let mut r = 0;

    for i in 1..m - 1 {
        if i < r {
            p[i] = (r - i).min(p[2 * c - i]);
        }

        while t[i + p[i] + 1] == t[i - p[i] - 1] {
            p[i] += 1;
        }

        if i + p[i] > r {
            c = i;
            r = i + p[i];
        }
    }

    // Extract results (skip sentinel characters)
    p[2..m - 2].to_vec()
}

/// Get longest palindromic substring
///
/// # Returns
/// (start_index, length)
///
/// # Example
/// ```
/// use procon_lib::palindrome::longest_palindrome;
///
/// let s = b"babad";
/// let (start, len) = longest_palindrome(s);
/// // Either "bab" or "aba"
/// assert_eq!(len, 3);
/// ```
pub fn longest_palindrome(s: &[u8]) -> (usize, usize) {
    if s.is_empty() {
        return (0, 0);
    }

    let radius = manacher(s);
    let mut max_len = 0;
    let mut max_center = 0;

    for (i, &r) in radius.iter().enumerate() {
        if r > max_len {
            max_len = r;
            max_center = i;
        }
    }

    // Convert back to original string indices
    // max_center in radius array corresponds to position in "^#a#b#c#$"
    // Center position in original = (max_center - 1) / 2
    // For odd palindrome: center is at s[(max_center - 1) / 2]
    // For even palindrome: center is between s[start] and s[start+1]
    let start = (max_center - max_len) / 2;
    (start, max_len)
}

/// Check if entire string is palindrome
///
/// # Example
/// ```
/// use procon_lib::palindrome::is_palindrome;
///
/// assert!(is_palindrome(b"racecar"));
/// assert!(is_palindrome(b"abba"));
/// assert!(!is_palindrome(b"hello"));
/// ```
pub fn is_palindrome(s: &[u8]) -> bool {
    let n = s.len();
    for i in 0..n / 2 {
        if s[i] != s[n - 1 - i] {
            return false;
        }
    }
    true
}

/// Count palindromic substrings
///
/// # Example
/// ```
/// use procon_lib::palindrome::count_palindromes;
///
/// assert_eq!(count_palindromes(b"aaa"), 6);  // a, a, a, aa, aa, aaa
/// assert_eq!(count_palindromes(b"abc"), 3);  // a, b, c
/// ```
pub fn count_palindromes(s: &[u8]) -> usize {
    let radius = manacher(s);
    radius.iter().map(|&r| (r + 1) / 2).sum()
}

/// Get all palindrome centers with their radii
///
/// # Returns
/// Vec of (center_index, is_odd, radius)
/// - center_index: index in original string
/// - is_odd: true if palindrome has odd length
/// - radius: number of characters on each side
pub fn palindrome_centers(s: &[u8]) -> Vec<(usize, bool, usize)> {
    let radius = manacher(s);
    let mut result = Vec::new();

    for (i, &r) in radius.iter().enumerate() {
        if r > 0 || i % 2 == 1 {
            let is_odd = i % 2 == 1;
            let center = i / 2;
            result.push((center, is_odd, r));
        }
    }

    result
}

/// Palindrome factorization
///
/// Returns minimum number of palindromes that s can be split into.
///
/// # Complexity
/// O(N^2) with Manacher preprocessing
///
/// # Example
/// ```
/// use procon_lib::palindrome::min_palindrome_cuts;
///
/// assert_eq!(min_palindrome_cuts(b"aab"), 2);  // "aa" + "b"
/// assert_eq!(min_palindrome_cuts(b"aba"), 1);  // "aba"
/// ```
pub fn min_palindrome_cuts(s: &[u8]) -> usize {
    let n = s.len();
    if n == 0 {
        return 0;
    }

    let radius = manacher(s);

    // is_palindrome[i][j] = true if s[i..=j] is palindrome
    let is_pal = |i: usize, j: usize| -> bool {
        let center = i + j;
        let len = j - i + 1;
        radius[center] >= len / 2
    };

    let mut dp = vec![n; n + 1];
    dp[0] = 0;

    for i in 0..n {
        for j in 0..=i {
            if is_pal(j, i) {
                dp[i + 1] = dp[i + 1].min(dp[j] + 1);
            }
        }
    }

    dp[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manacher() {
        let s = b"abacaba";
        let radius = manacher(s);

        // Center at 'c' (index 3)
        // radius[7] should be 3 (full string is palindrome)
        assert_eq!(radius[7], 3);
    }

    #[test]
    fn test_manacher_even() {
        let s = b"abba";
        let radius = manacher(s);

        // Center between 'b' and 'b'
        // radius[4] should be 2
        assert_eq!(radius[4], 2);
    }

    #[test]
    fn test_longest_palindrome() {
        let s = b"babad";
        let (start, len) = longest_palindrome(s);
        assert_eq!(len, 3);

        // Verify it's actually a palindrome
        let substr = &s[start..start + len];
        assert!(is_palindrome(substr));
    }

    #[test]
    fn test_is_palindrome() {
        assert!(is_palindrome(b"racecar"));
        assert!(is_palindrome(b"abba"));
        assert!(is_palindrome(b"a"));
        assert!(is_palindrome(b""));
        assert!(!is_palindrome(b"hello"));
    }

    #[test]
    fn test_count_palindromes() {
        assert_eq!(count_palindromes(b"aaa"), 6);
        assert_eq!(count_palindromes(b"abc"), 3);
        assert_eq!(count_palindromes(b"abba"), 6); // a, b, b, a, bb, abba
    }

    #[test]
    fn test_min_palindrome_cuts() {
        assert_eq!(min_palindrome_cuts(b"aab"), 2);
        assert_eq!(min_palindrome_cuts(b"aba"), 1);
        assert_eq!(min_palindrome_cuts(b"abc"), 3);
    }
}
