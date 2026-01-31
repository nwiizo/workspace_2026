//! 文字列アルゴリズム

/// Z-Algorithm
/// z[i] = s[0..] と s[i..] の最長共通接頭辞の長さ
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

/// Rolling Hash
#[allow(dead_code)]
pub struct RollingHash {
    hash: Vec<u64>,
    pow: Vec<u64>,
    base: u64,
    modulo: u64,
}

impl RollingHash {
    pub fn new(s: &[u8], base: u64, modulo: u64) -> Self {
        let n = s.len();
        let mut hash = vec![0u64; n + 1];
        let mut pow = vec![1u64; n + 1];

        for i in 0..n {
            hash[i + 1] = (hash[i] * base + s[i] as u64) % modulo;
            pow[i + 1] = pow[i] * base % modulo;
        }

        Self {
            hash,
            pow,
            base,
            modulo,
        }
    }

    /// [l, r) のハッシュ値
    pub fn get(&self, l: usize, r: usize) -> u64 {
        let h = self.hash[r] + self.modulo - self.hash[l] * self.pow[r - l] % self.modulo;
        h % self.modulo
    }
}

/// Suffix Array 構築 (ダブリング O(n log n))
///
/// # Example
/// ```
/// use typical90::string_algo::suffix_array;
///
/// let s = b"banana";
/// let sa = suffix_array(s);
/// // suffix array: [5,3,1,0,4,2] (a, ana, anana, banana, na, nana)
/// assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
/// ```
pub fn suffix_array(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }

    // 簡易版: O(n log n) のダブリングベース
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

/// LCP Array (Longest Common Prefix Array)
///
/// suffix_array と同時に使用し、隣接するsuffixの最長共通接頭辞の長さを求める
///
/// # Arguments
/// * `s` - 元の文字列
/// * `sa` - suffix array
///
/// # Returns
/// lcp[i] = s[sa[i]..] と s[sa[i+1]..] の最長共通接頭辞の長さ
///
/// # Example
/// ```
/// use typical90::string_algo::{suffix_array, lcp_array};
///
/// let s = b"banana";
/// let sa = suffix_array(s);
/// let lcp = lcp_array(s, &sa);
/// // sa = [5,3,1,0,4,2] -> suffixes: a, ana, anana, banana, na, nana
/// // lcp = [1, 3, 0, 0, 2] (a-ana:1, ana-anana:3, anana-banana:0, banana-na:0, na-nana:2)
/// assert_eq!(lcp, vec![1, 3, 0, 0, 2]);
/// ```
pub fn lcp_array(s: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![];
    }

    // rank[i] = sa の中での位置 (sa[rank[i]] = i)
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

        h = h.saturating_sub(1);
    }

    lcp
}

/// 文字列の出現回数を数える
///
/// suffix array と lcp array を使用
///
/// # Returns
/// パターンの出現回数
///
/// # Example
/// ```
/// use typical90::string_algo::{suffix_array, count_occurrences};
///
/// let s = b"abracadabra";
/// let sa = suffix_array(s);
/// assert_eq!(count_occurrences(s, &sa, b"abra"), 2);
/// assert_eq!(count_occurrences(s, &sa, b"a"), 5);
/// assert_eq!(count_occurrences(s, &sa, b"xyz"), 0);
/// ```
pub fn count_occurrences(s: &[u8], sa: &[usize], pattern: &[u8]) -> usize {
    let n = s.len();
    let m = pattern.len();

    if m == 0 || n == 0 {
        return 0;
    }

    // 下界を二分探索
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

    // 上界を二分探索
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
    fn test_rolling_hash() {
        let s = b"abcabc";
        let rh = RollingHash::new(s, 31, 1_000_000_007);

        // "abc" (0-3) と "abc" (3-6) は同じハッシュ
        assert_eq!(rh.get(0, 3), rh.get(3, 6));

        // "ab" (0-2) と "bc" (1-3) は異なるハッシュ
        assert_ne!(rh.get(0, 2), rh.get(1, 3));
    }

    #[test]
    fn test_suffix_array() {
        let s = b"banana";
        let sa = suffix_array(s);
        // banana の suffix array: [5,3,1,0,4,2]
        // a, ana, anana, banana, na, nana
        assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn test_lcp_array() {
        let s = b"banana";
        let sa = suffix_array(s);
        let lcp = lcp_array(s, &sa);
        // a-ana:1, ana-anana:3, anana-banana:0, banana-na:0, na-nana:2
        assert_eq!(lcp, vec![1, 3, 0, 0, 2]);
    }

    #[test]
    fn test_lcp_array_empty() {
        let s = b"";
        let sa = suffix_array(s);
        let lcp = lcp_array(s, &sa);
        assert!(lcp.is_empty());
    }

    #[test]
    fn test_count_occurrences() {
        let s = b"abracadabra";
        let sa = suffix_array(s);
        assert_eq!(count_occurrences(s, &sa, b"abra"), 2);
        assert_eq!(count_occurrences(s, &sa, b"a"), 5);
        assert_eq!(count_occurrences(s, &sa, b"xyz"), 0);
        assert_eq!(count_occurrences(s, &sa, b"bra"), 2);
    }
}
