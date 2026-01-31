//! Utility Functions
//!
//! - Coordinate compression
//! - Imos method (1D and 2D)
//! - Permutations
//! - Run-length encoding

/// Coordinate compression
///
/// # Returns
/// (compressed_values, mapping)
///
/// # Example
/// ```
/// use procon_lib::util::compress;
///
/// let a = vec![100, 1, 50, 1, 100];
/// let (compressed, mapping) = compress(&a);
/// assert_eq!(compressed, vec![2, 0, 1, 0, 2]);
/// assert_eq!(mapping, vec![1, 50, 100]);
/// ```
pub fn compress(a: &[i64]) -> (Vec<usize>, Vec<i64>) {
    let mut sorted: Vec<i64> = a.to_vec();
    sorted.sort();
    sorted.dedup();

    let compressed: Vec<usize> = a
        .iter()
        .map(|x| sorted.binary_search(x).unwrap())
        .collect();

    (compressed, sorted)
}

/// Coordinate compression with generic type
pub fn compress_generic<T: Ord + Clone>(a: &[T]) -> (Vec<usize>, Vec<T>) {
    let mut sorted: Vec<T> = a.to_vec();
    sorted.sort();
    sorted.dedup();

    let compressed: Vec<usize> = a
        .iter()
        .map(|x| sorted.binary_search(x).unwrap())
        .collect();

    (compressed, sorted)
}

/// 1D Imos method
///
/// Add value to range [l, r) then compute prefix sum.
///
/// # Example
/// ```
/// use procon_lib::util::Imos1D;
///
/// let mut imos = Imos1D::new(5);
/// imos.add(0, 3, 1);  // add 1 to [0, 3)
/// imos.add(2, 5, 2);  // add 2 to [2, 5)
/// let result = imos.build();
/// assert_eq!(result, vec![1, 1, 3, 2, 2]);
/// ```
pub struct Imos1D {
    diff: Vec<i64>,
}

impl Imos1D {
    pub fn new(n: usize) -> Self {
        Self {
            diff: vec![0; n + 1],
        }
    }

    /// Add value to range [l, r)
    pub fn add(&mut self, l: usize, r: usize, value: i64) {
        self.diff[l] += value;
        self.diff[r] -= value;
    }

    /// Build the result array
    pub fn build(self) -> Vec<i64> {
        let n = self.diff.len() - 1;
        let mut result = vec![0; n];
        let mut sum = 0;
        for i in 0..n {
            sum += self.diff[i];
            result[i] = sum;
        }
        result
    }
}

/// 2D Imos method
///
/// Add value to rectangle [r1, r2) x [c1, c2) then compute prefix sum.
///
/// # Example
/// ```
/// use procon_lib::util::Imos2D;
///
/// let mut imos = Imos2D::new(3, 3);
/// imos.add(0, 0, 2, 2, 1);  // add 1 to top-left 2x2
/// imos.add(1, 1, 3, 3, 2);  // add 2 to bottom-right 2x2
/// let result = imos.build();
/// assert_eq!(result[0][0], 1);
/// assert_eq!(result[1][1], 3);  // 1 + 2
/// assert_eq!(result[2][2], 2);
/// ```
pub struct Imos2D {
    diff: Vec<Vec<i64>>,
}

impl Imos2D {
    pub fn new(h: usize, w: usize) -> Self {
        Self {
            diff: vec![vec![0; w + 1]; h + 1],
        }
    }

    /// Add value to rectangle [r1, r2) x [c1, c2)
    pub fn add(&mut self, r1: usize, c1: usize, r2: usize, c2: usize, value: i64) {
        self.diff[r1][c1] += value;
        self.diff[r1][c2] -= value;
        self.diff[r2][c1] -= value;
        self.diff[r2][c2] += value;
    }

    /// Build the result array
    #[allow(clippy::needless_range_loop)]
    pub fn build(mut self) -> Vec<Vec<i64>> {
        let h = self.diff.len() - 1;
        let w = self.diff[0].len() - 1;

        // Horizontal prefix sum
        for i in 0..=h {
            for j in 1..=w {
                self.diff[i][j] += self.diff[i][j - 1];
            }
        }

        // Vertical prefix sum
        for i in 1..=h {
            for j in 0..=w {
                self.diff[i][j] += self.diff[i - 1][j];
            }
        }

        // Extract result
        self.diff.truncate(h);
        for row in &mut self.diff {
            row.truncate(w);
        }
        self.diff
    }
}

/// Next permutation (C++ style)
///
/// # Returns
/// true if next permutation exists, false otherwise
///
/// # Example
/// ```
/// use procon_lib::util::next_permutation;
///
/// let mut a = vec![1, 2, 3];
/// assert!(next_permutation(&mut a));
/// assert_eq!(a, vec![1, 3, 2]);
/// ```
pub fn next_permutation<T: Ord>(arr: &mut [T]) -> bool {
    let n = arr.len();
    if n <= 1 {
        return false;
    }

    let mut i = n - 1;
    while i > 0 && arr[i - 1] >= arr[i] {
        i -= 1;
    }

    if i == 0 {
        return false;
    }

    let mut j = n - 1;
    while arr[j] <= arr[i - 1] {
        j -= 1;
    }

    arr.swap(i - 1, j);
    arr[i..].reverse();
    true
}

/// Previous permutation
pub fn prev_permutation<T: Ord>(arr: &mut [T]) -> bool {
    let n = arr.len();
    if n <= 1 {
        return false;
    }

    let mut i = n - 1;
    while i > 0 && arr[i - 1] <= arr[i] {
        i -= 1;
    }

    if i == 0 {
        return false;
    }

    let mut j = n - 1;
    while arr[j] >= arr[i - 1] {
        j -= 1;
    }

    arr.swap(i - 1, j);
    arr[i..].reverse();
    true
}

/// Run-length encoding
///
/// # Example
/// ```
/// use procon_lib::util::run_length_encode;
///
/// let s = vec!['a', 'a', 'b', 'b', 'b', 'a'];
/// let rle = run_length_encode(&s);
/// assert_eq!(rle, vec![('a', 2), ('b', 3), ('a', 1)]);
/// ```
pub fn run_length_encode<T: Eq + Clone>(s: &[T]) -> Vec<(T, usize)> {
    if s.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current = s[0].clone();
    let mut count = 1;

    for item in s.iter().skip(1) {
        if *item == current {
            count += 1;
        } else {
            result.push((current, count));
            current = item.clone();
            count = 1;
        }
    }
    result.push((current, count));

    result
}

/// Decode run-length encoding
pub fn run_length_decode<T: Clone>(rle: &[(T, usize)]) -> Vec<T> {
    let mut result = Vec::new();
    for (item, count) in rle {
        for _ in 0..*count {
            result.push(item.clone());
        }
    }
    result
}

/// Iterate over all subsets of a set represented as bitmask
///
/// # Example
/// ```
/// use procon_lib::util::subsets;
///
/// let mask = 0b111u32; // {0, 1, 2}
/// let subs: Vec<u32> = subsets(mask).collect();
/// // All subsets including empty set: 8 subsets
/// assert_eq!(subs.len(), 8);
/// ```
pub fn subsets(mask: u32) -> impl Iterator<Item = u32> {
    std::iter::successors(Some(mask), move |&sub| {
        if sub == 0 {
            None
        } else {
            Some((sub - 1) & mask)
        }
    })
    .chain(std::iter::once(0))
}

/// Iterate over all supersets within a universe
pub fn supersets(mask: u32, universe: u32) -> impl Iterator<Item = u32> {
    std::iter::successors(Some(mask), move |&sup| {
        if sup == universe {
            None
        } else {
            Some((sup + 1) | mask)
        }
    })
}

/// Count set bits (popcount)
pub fn popcount(x: u64) -> u32 {
    x.count_ones()
}

/// Lowest set bit
pub fn lowest_bit(x: u64) -> u64 {
    x & x.wrapping_neg()
}

/// Remove lowest set bit
pub fn remove_lowest_bit(x: u64) -> u64 {
    x & (x - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress() {
        let a = vec![100, 1, 50, 1, 100];
        let (compressed, mapping) = compress(&a);
        assert_eq!(compressed, vec![2, 0, 1, 0, 2]);
        assert_eq!(mapping, vec![1, 50, 100]);
    }

    #[test]
    fn test_imos_1d() {
        let mut imos = Imos1D::new(5);
        imos.add(0, 3, 1);
        imos.add(2, 5, 2);
        let result = imos.build();
        assert_eq!(result, vec![1, 1, 3, 2, 2]);
    }

    #[test]
    fn test_imos_2d() {
        let mut imos = Imos2D::new(3, 3);
        imos.add(0, 0, 2, 2, 1);
        imos.add(1, 1, 3, 3, 2);
        let result = imos.build();
        assert_eq!(result[0][0], 1);
        assert_eq!(result[1][1], 3);
        assert_eq!(result[2][2], 2);
    }

    #[test]
    fn test_next_permutation() {
        let mut a = vec![1, 2, 3];
        assert!(next_permutation(&mut a));
        assert_eq!(a, vec![1, 3, 2]);
        assert!(next_permutation(&mut a));
        assert_eq!(a, vec![2, 1, 3]);
    }

    #[test]
    fn test_next_permutation_last() {
        let mut a = vec![3, 2, 1];
        assert!(!next_permutation(&mut a));
    }

    #[test]
    fn test_run_length_encode() {
        let s = vec!['a', 'a', 'b', 'b', 'b', 'a'];
        let rle = run_length_encode(&s);
        assert_eq!(rle, vec![('a', 2), ('b', 3), ('a', 1)]);

        let decoded = run_length_decode(&rle);
        assert_eq!(decoded, s);
    }

    #[test]
    fn test_subsets() {
        let mask = 0b111u32;
        let subs: Vec<u32> = subsets(mask).collect();
        assert_eq!(subs.len(), 8);
        assert!(subs.contains(&0b000));
        assert!(subs.contains(&0b111));
    }

    #[test]
    fn test_bit_operations() {
        assert_eq!(popcount(0b10110), 3);
        assert_eq!(lowest_bit(0b10100), 0b00100);
        assert_eq!(remove_lowest_bit(0b10100), 0b10000);
    }
}
