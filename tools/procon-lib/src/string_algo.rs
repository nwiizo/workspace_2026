//! String Algorithms
//!
//! - Z-Algorithm
//! - KMP
//! - Rolling Hash
//! - Suffix Array
//! - LCP Array

/// Z-Algorithm
///
/// z[i] = length of longest common prefix of s[0..] and s[i..]
///
/// # Complexity
/// O(N)
///
/// # Example
/// ```
/// use procon_lib::string_algo::z_algorithm;
///
/// let s = b"abacaba";
/// let z = z_algorithm(s);
/// assert_eq!(z, vec![7, 0, 1, 0, 3, 0, 1]);
/// ```
pub fn z_algorithm(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    let mut z = vec![0; n];
    z[0] = n;
    let (mut l, mut r) = (0, 0);

    for i in 1..n {
        if i < r && z[i - l] < r - i {
            z[i] = z[i - l];
        } else {
            let mut j = r.saturating_sub(i);
            while i + j < n && s[j] == s[i + j] {
                j += 1;
            }
            z[i] = j;
            l = i;
            r = i + j;
        }
    }
    z
}

/// KMP failure function
///
/// failure[i] = length of longest proper prefix of s[0..=i] that is also a suffix
///
/// # Example
/// ```
/// use procon_lib::string_algo::kmp_failure;
///
/// let s = b"abacaba";
/// let f = kmp_failure(s);
/// assert_eq!(f, vec![0, 0, 1, 0, 1, 2, 3]);
/// ```
pub fn kmp_failure(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    let mut failure = vec![0; n];
    let mut j = 0;

    for i in 1..n {
        while j > 0 && s[i] != s[j] {
            j = failure[j - 1];
        }
        if s[i] == s[j] {
            j += 1;
        }
        failure[i] = j;
    }

    failure
}

/// Find all occurrences of pattern in text using KMP
///
/// # Returns
/// List of starting positions
///
/// # Example
/// ```
/// use procon_lib::string_algo::kmp_search;
///
/// let text = b"abcabcabc";
/// let pattern = b"abc";
/// assert_eq!(kmp_search(text, pattern), vec![0, 3, 6]);
/// ```
pub fn kmp_search(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..=text.len()).collect();
    }

    let failure = kmp_failure(pattern);
    let mut result = Vec::new();
    let mut j = 0;

    for (i, &c) in text.iter().enumerate() {
        while j > 0 && c != pattern[j] {
            j = failure[j - 1];
        }
        if c == pattern[j] {
            j += 1;
        }
        if j == pattern.len() {
            result.push(i + 1 - pattern.len());
            j = failure[j - 1];
        }
    }

    result
}

/// Rolling Hash
///
/// # Example
/// ```
/// use procon_lib::string_algo::RollingHash;
///
/// let s = b"abcabc";
/// let rh = RollingHash::new(s);
///
/// // "abc" (0-3) and "abc" (3-6) have same hash
/// assert_eq!(rh.get(0, 3), rh.get(3, 6));
/// ```
pub struct RollingHash {
    hash: Vec<u64>,
    pow: Vec<u64>,
    base: u64,
    modulo: u64,
}

impl RollingHash {
    /// Create with default base and modulo
    pub fn new(s: &[u8]) -> Self {
        Self::with_params(s, 31, (1 << 61) - 1)
    }

    /// Create with custom parameters
    pub fn with_params(s: &[u8], base: u64, modulo: u64) -> Self {
        let n = s.len();
        let mut hash = vec![0u64; n + 1];
        let mut pow = vec![1u64; n + 1];

        for i in 0..n {
            hash[i + 1] = Self::mod_add(Self::mod_mul(hash[i], base, modulo), s[i] as u64, modulo);
            pow[i + 1] = Self::mod_mul(pow[i], base, modulo);
        }

        Self {
            hash,
            pow,
            base,
            modulo,
        }
    }

    fn mod_mul(a: u64, b: u64, m: u64) -> u64 {
        ((a as u128 * b as u128) % m as u128) as u64
    }

    fn mod_add(a: u64, b: u64, m: u64) -> u64 {
        let sum = a + b;
        if sum >= m {
            sum - m
        } else {
            sum
        }
    }

    /// Get hash of s[l..r)
    pub fn get(&self, l: usize, r: usize) -> u64 {
        let h = self.hash[r] + self.modulo
            - Self::mod_mul(self.hash[l], self.pow[r - l], self.modulo);
        h % self.modulo
    }

    /// Combine two hashes
    pub fn combine(&self, h1: u64, h2: u64, len2: usize) -> u64 {
        Self::mod_add(Self::mod_mul(h1, self.pow[len2], self.modulo), h2, self.modulo)
    }
}

/// Suffix Array (SA-IS algorithm style, O(N log N))
///
/// # Example
/// ```
/// use procon_lib::string_algo::suffix_array;
///
/// let s = b"banana";
/// let sa = suffix_array(s);
/// assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
/// ```
pub fn suffix_array(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    let mut sa: Vec<usize> = (0..n).collect();
    let mut rank: Vec<i32> = s.iter().map(|&c| c as i32).collect();
    let mut tmp = vec![0i32; n];

    let mut k = 1;
    while k < n {
        sa.sort_by(|&i, &j| {
            let ri = rank[i];
            let rj = rank[j];
            if ri != rj {
                return ri.cmp(&rj);
            }
            let ri2 = if i + k < n { rank[i + k] } else { -1 };
            let rj2 = if j + k < n { rank[j + k] } else { -1 };
            ri2.cmp(&rj2)
        });

        tmp[sa[0]] = 0;
        for i in 1..n {
            let prev = sa[i - 1];
            let curr = sa[i];
            let same = rank[prev] == rank[curr]
                && (prev + k >= n) == (curr + k >= n)
                && (prev + k >= n || rank[prev + k] == rank[curr + k]);
            tmp[curr] = tmp[prev] + if same { 0 } else { 1 };
        }
        std::mem::swap(&mut rank, &mut tmp);

        if rank[sa[n - 1]] == (n - 1) as i32 {
            break;
        }
        k *= 2;
    }

    sa
}

/// LCP Array (Kasai's algorithm)
///
/// lcp[i] = longest common prefix of s[sa[i]..] and s[sa[i+1]..]
///
/// # Complexity
/// O(N)
///
/// # Example
/// ```
/// use procon_lib::string_algo::{suffix_array, lcp_array};
///
/// let s = b"banana";
/// let sa = suffix_array(s);
/// let lcp = lcp_array(s, &sa);
/// assert_eq!(lcp, vec![1, 3, 0, 0, 2]);
/// ```
pub fn lcp_array(s: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = s.len();
    if n <= 1 {
        return vec![];
    }

    let mut rank = vec![0; n];
    for (i, &sa_i) in sa.iter().enumerate() {
        rank[sa_i] = i;
    }

    let mut lcp = vec![0; n - 1];
    let mut h = 0;

    for i in 0..n {
        if rank[i] == 0 {
            h = 0;
            continue;
        }

        let j = sa[rank[i] - 1];
        while i + h < n && j + h < n && s[i + h] == s[j + h] {
            h += 1;
        }

        lcp[rank[i] - 1] = h;

        if h > 0 {
            h -= 1;
        }
    }

    lcp
}

/// Count occurrences of pattern in text using suffix array
///
/// # Complexity
/// O(M log N) where M = pattern length, N = text length
pub fn count_occurrences(s: &[u8], sa: &[usize], pattern: &[u8]) -> usize {
    let n = s.len();
    let m = pattern.len();

    if m == 0 || n == 0 {
        return 0;
    }

    let lower = {
        let mut lo = 0;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let suffix = &s[sa[mid]..];
            if suffix < pattern {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    };

    let upper = {
        let mut lo = 0;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let suffix = &s[sa[mid]..];
            let cmp_len = suffix.len().min(m);
            if &suffix[..cmp_len] <= pattern {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    };

    upper.saturating_sub(lower)
}

/// Number of distinct substrings
///
/// # Example
/// ```
/// use procon_lib::string_algo::{suffix_array, lcp_array, distinct_substrings};
///
/// let s = b"aab";
/// let sa = suffix_array(s);
/// let lcp = lcp_array(s, &sa);
/// assert_eq!(distinct_substrings(s.len(), &lcp), 5); // a, aa, aab, ab, b
/// ```
pub fn distinct_substrings(n: usize, lcp: &[usize]) -> usize {
    let total = n * (n + 1) / 2;
    let duplicates: usize = lcp.iter().sum();
    total - duplicates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_algorithm() {
        let s = b"abacaba";
        let z = z_algorithm(s);
        assert_eq!(z, vec![7, 0, 1, 0, 3, 0, 1]);
    }

    #[test]
    fn test_kmp_failure() {
        let s = b"abacaba";
        let f = kmp_failure(s);
        assert_eq!(f, vec![0, 0, 1, 0, 1, 2, 3]);
    }

    #[test]
    fn test_kmp_search() {
        let text = b"abcabcabc";
        let pattern = b"abc";
        assert_eq!(kmp_search(text, pattern), vec![0, 3, 6]);
    }

    #[test]
    fn test_rolling_hash() {
        let s = b"abcabc";
        let rh = RollingHash::new(s);

        assert_eq!(rh.get(0, 3), rh.get(3, 6));
        assert_ne!(rh.get(0, 2), rh.get(1, 3));
    }

    #[test]
    fn test_suffix_array() {
        let s = b"banana";
        let sa = suffix_array(s);
        assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn test_lcp_array() {
        let s = b"banana";
        let sa = suffix_array(s);
        let lcp = lcp_array(s, &sa);
        assert_eq!(lcp, vec![1, 3, 0, 0, 2]);
    }

    #[test]
    fn test_count_occurrences() {
        let s = b"abracadabra";
        let sa = suffix_array(s);
        assert_eq!(count_occurrences(s, &sa, b"abra"), 2);
        assert_eq!(count_occurrences(s, &sa, b"a"), 5);
        assert_eq!(count_occurrences(s, &sa, b"xyz"), 0);
    }

    #[test]
    fn test_distinct_substrings() {
        let s = b"aab";
        let sa = suffix_array(s);
        let lcp = lcp_array(s, &sa);
        assert_eq!(distinct_substrings(s.len(), &lcp), 5);
    }
}
