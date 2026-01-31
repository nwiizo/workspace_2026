//! Data Structures
//!
//! - Binary Indexed Tree (Fenwick Tree)
//! - Combination
//! - Sparse Table

/// Binary Indexed Tree (Fenwick Tree)
///
/// Point update and prefix sum in O(log N).
///
/// # Example
/// ```
/// use procon_lib::data_structures::Bit;
///
/// let mut bit = Bit::new(10);
/// bit.add(3, 5);
/// bit.add(7, 3);
///
/// assert_eq!(bit.prefix_sum(5), 5);   // [0,5) sum
/// assert_eq!(bit.prefix_sum(10), 8);  // [0,10) sum
/// assert_eq!(bit.sum(3, 7), 5);       // [3,7] sum
/// ```
#[derive(Clone)]
pub struct Bit {
    data: Vec<i64>,
}

impl Bit {
    /// Create a new BIT of size n
    pub fn new(n: usize) -> Self {
        Self {
            data: vec![0; n + 1],
        }
    }

    /// Create a BIT from a slice
    ///
    /// # Complexity
    /// O(N)
    pub fn from_slice(a: &[i64]) -> Self {
        let n = a.len();
        let mut bit = Self::new(n);
        for (i, &x) in a.iter().enumerate() {
            bit.add(i, x);
        }
        bit
    }

    /// Add x to index i
    ///
    /// # Complexity
    /// O(log N)
    pub fn add(&mut self, mut i: usize, x: i64) {
        i += 1;
        while i < self.data.len() {
            self.data[i] += x;
            i += i & i.wrapping_neg();
        }
    }

    /// Get prefix sum [0, i)
    ///
    /// # Complexity
    /// O(log N)
    pub fn prefix_sum(&self, mut i: usize) -> i64 {
        let mut s = 0;
        while i > 0 {
            s += self.data[i];
            i -= i & i.wrapping_neg();
        }
        s
    }

    /// Get sum [l, r]
    ///
    /// # Complexity
    /// O(log N)
    pub fn sum(&self, l: usize, r: usize) -> i64 {
        if l > r {
            return 0;
        }
        self.prefix_sum(r + 1) - self.prefix_sum(l)
    }

    /// Find the smallest index i such that prefix_sum(i) >= x
    ///
    /// # Complexity
    /// O(log N)
    pub fn lower_bound(&self, mut x: i64) -> usize {
        if x <= 0 {
            return 0;
        }
        let n = self.data.len() - 1;
        let mut k = 1;
        while k * 2 <= n {
            k *= 2;
        }
        let mut i = 0;
        while k > 0 {
            if i + k <= n && self.data[i + k] < x {
                x -= self.data[i + k];
                i += k;
            }
            k /= 2;
        }
        i + 1
    }
}

/// 2D Binary Indexed Tree
///
/// Point update and 2D prefix sum in O(log N * log M).
#[derive(Clone)]
pub struct Bit2D {
    data: Vec<Vec<i64>>,
    h: usize,
    w: usize,
}

impl Bit2D {
    /// Create a new 2D BIT of size h x w
    pub fn new(h: usize, w: usize) -> Self {
        Self {
            data: vec![vec![0; w + 1]; h + 1],
            h,
            w,
        }
    }

    /// Add x to position (r, c)
    pub fn add(&mut self, mut r: usize, c: usize, x: i64) {
        r += 1;
        while r <= self.h {
            let mut c = c + 1;
            while c <= self.w {
                self.data[r][c] += x;
                c += c & c.wrapping_neg();
            }
            r += r & r.wrapping_neg();
        }
    }

    /// Get prefix sum [0, r) x [0, c)
    pub fn prefix_sum(&self, mut r: usize, c: usize) -> i64 {
        let mut s = 0;
        while r > 0 {
            let mut c = c;
            while c > 0 {
                s += self.data[r][c];
                c -= c & c.wrapping_neg();
            }
            r -= r & r.wrapping_neg();
        }
        s
    }

    /// Get sum in rectangle [r1, r2] x [c1, c2]
    pub fn sum(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> i64 {
        self.prefix_sum(r2 + 1, c2 + 1) - self.prefix_sum(r2 + 1, c1)
            - self.prefix_sum(r1, c2 + 1)
            + self.prefix_sum(r1, c1)
    }
}

/// Combination calculator with precomputed factorials
///
/// # Example
/// ```
/// use procon_lib::data_structures::Combination;
///
/// let comb = Combination::new(100, 1_000_000_007);
/// assert_eq!(comb.comb(5, 2), 10);
/// assert_eq!(comb.comb(10, 3), 120);
/// assert_eq!(comb.perm(5, 2), 20);
/// ```
#[derive(Clone)]
pub struct Combination {
    fact: Vec<i64>,
    inv_fact: Vec<i64>,
    modulo: i64,
}

impl Combination {
    /// Create a new Combination calculator
    ///
    /// # Arguments
    /// - `n`: Maximum n for nCr
    /// - `modulo`: Must be a prime number
    ///
    /// # Complexity
    /// O(N)
    pub fn new(n: usize, modulo: i64) -> Self {
        let mut fact = vec![1i64; n + 1];
        for i in 1..=n {
            fact[i] = fact[i - 1] * i as i64 % modulo;
        }

        let mut inv_fact = vec![1i64; n + 1];
        inv_fact[n] = Self::mod_pow(fact[n], modulo - 2, modulo);
        for i in (0..n).rev() {
            inv_fact[i] = inv_fact[i + 1] * (i + 1) as i64 % modulo;
        }

        Self {
            fact,
            inv_fact,
            modulo,
        }
    }

    fn mod_pow(mut base: i64, mut exp: i64, m: i64) -> i64 {
        let mut result = 1i64;
        base %= m;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result * base % m;
            }
            exp >>= 1;
            base = base * base % m;
        }
        result
    }

    /// Calculate nCr (binomial coefficient)
    ///
    /// # Complexity
    /// O(1)
    pub fn comb(&self, n: usize, r: usize) -> i64 {
        if n < r {
            return 0;
        }
        self.fact[n] * self.inv_fact[r] % self.modulo * self.inv_fact[n - r] % self.modulo
    }

    /// Calculate nPr (permutation)
    ///
    /// # Complexity
    /// O(1)
    pub fn perm(&self, n: usize, r: usize) -> i64 {
        if n < r {
            return 0;
        }
        self.fact[n] * self.inv_fact[n - r] % self.modulo
    }

    /// Get n!
    ///
    /// # Complexity
    /// O(1)
    pub fn factorial(&self, n: usize) -> i64 {
        self.fact[n]
    }

    /// Get (n!)^(-1) mod p
    ///
    /// # Complexity
    /// O(1)
    pub fn inv_factorial(&self, n: usize) -> i64 {
        self.inv_fact[n]
    }

    /// Calculate nHr (homogeneous product)
    /// = n+r-1 C r
    ///
    /// # Complexity
    /// O(1)
    pub fn homo(&self, n: usize, r: usize) -> i64 {
        if n == 0 && r == 0 {
            return 1;
        }
        if n == 0 {
            return 0;
        }
        self.comb(n + r - 1, r)
    }
}

/// Sparse Table for Range Minimum/Maximum Query
///
/// Preprocessing: O(N log N), Query: O(1)
///
/// # Example
/// ```
/// use procon_lib::data_structures::SparseTable;
///
/// let a = vec![5, 3, 7, 2, 8, 1, 4];
/// let st = SparseTable::new(&a, |&x, &y| x.min(y));
///
/// assert_eq!(st.query(0, 7), 1);  // min of whole array
/// assert_eq!(st.query(0, 4), 2);  // min of [0, 4)
/// assert_eq!(st.query(1, 3), 3);  // min of [1, 3)
/// ```
pub struct SparseTable<T, F> {
    table: Vec<Vec<T>>,
    log: Vec<usize>,
    op: F,
}

impl<T: Clone, F: Fn(&T, &T) -> T> SparseTable<T, F> {
    /// Create a new Sparse Table
    ///
    /// # Arguments
    /// - `a`: Input array
    /// - `op`: Idempotent operation (e.g., min, max, gcd, lcm, and, or)
    ///
    /// # Complexity
    /// O(N log N) time and space
    pub fn new(a: &[T], op: F) -> Self {
        let n = a.len();
        if n == 0 {
            return Self {
                table: vec![],
                log: vec![0],
                op,
            };
        }

        let log_n = (usize::BITS - n.leading_zeros()) as usize;

        // Precompute log values
        let mut log = vec![0; n + 1];
        for i in 2..=n {
            log[i] = log[i / 2] + 1;
        }

        // Build sparse table
        let mut table = vec![vec![]; log_n];
        table[0] = a.to_vec();

        for k in 1..log_n {
            let prev_len = 1 << (k - 1);
            table[k] = Vec::with_capacity(n - (1 << k) + 1);
            for i in 0..=n - (1 << k) {
                let val = op(&table[k - 1][i], &table[k - 1][i + prev_len]);
                table[k].push(val);
            }
        }

        Self { table, log, op }
    }

    /// Query range [l, r)
    ///
    /// # Complexity
    /// O(1)
    ///
    /// # Panics
    /// Panics if l >= r
    pub fn query(&self, l: usize, r: usize) -> T {
        assert!(l < r);
        let k = self.log[r - l];
        (self.op)(&self.table[k][l], &self.table[k][r - (1 << k)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit() {
        let mut bit = Bit::new(10);
        bit.add(2, 1);
        bit.add(5, 1);
        bit.add(7, 1);

        assert_eq!(bit.sum(0, 9), 3);
        assert_eq!(bit.sum(3, 6), 1);
        assert_eq!(bit.sum(2, 5), 2);
        assert_eq!(bit.prefix_sum(3), 1);
        assert_eq!(bit.prefix_sum(6), 2);
    }

    #[test]
    fn test_bit_lower_bound() {
        let mut bit = Bit::new(10);
        bit.add(0, 1);
        bit.add(2, 2);
        bit.add(4, 3);
        // prefix sums: [1, 1, 3, 3, 6, 6, 6, 6, 6, 6]

        assert_eq!(bit.lower_bound(1), 1);
        assert_eq!(bit.lower_bound(2), 3);
        assert_eq!(bit.lower_bound(3), 3);
        assert_eq!(bit.lower_bound(4), 5);
    }

    #[test]
    fn test_bit_2d() {
        let mut bit = Bit2D::new(3, 3);
        bit.add(0, 0, 1);
        bit.add(1, 1, 2);
        bit.add(2, 2, 3);

        assert_eq!(bit.prefix_sum(3, 3), 6);
        assert_eq!(bit.sum(0, 0, 1, 1), 3);
        assert_eq!(bit.sum(1, 1, 2, 2), 5);
    }

    #[test]
    fn test_combination() {
        let comb = Combination::new(100, 1_000_000_007);

        assert_eq!(comb.comb(5, 2), 10);
        assert_eq!(comb.comb(10, 3), 120);
        assert_eq!(comb.comb(5, 0), 1);
        assert_eq!(comb.comb(5, 5), 1);
        assert_eq!(comb.comb(3, 5), 0);

        assert_eq!(comb.perm(5, 2), 20);
        assert_eq!(comb.perm(5, 5), 120);

        assert_eq!(comb.homo(3, 2), 6); // 4C2
    }

    #[test]
    fn test_sparse_table() {
        let a = vec![5, 3, 7, 2, 8, 1, 4];
        let st = SparseTable::new(&a, |&x, &y| x.min(y));

        assert_eq!(st.query(0, 7), 1);
        assert_eq!(st.query(0, 4), 2);
        assert_eq!(st.query(1, 3), 3);
        assert_eq!(st.query(4, 6), 1);
    }

    #[test]
    fn test_sparse_table_max() {
        let a = vec![5, 3, 7, 2, 8, 1, 4];
        let st = SparseTable::new(&a, |&x, &y| x.max(y));

        assert_eq!(st.query(0, 7), 8);
        assert_eq!(st.query(0, 4), 7);
        assert_eq!(st.query(5, 7), 4);
    }
}
