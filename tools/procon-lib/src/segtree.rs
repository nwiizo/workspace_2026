//! Segment Tree
//!
//! Point update and range query in O(log N).
//!
//! # Example
//! ```
//! use procon_lib::segtree::{SegTree, Sum, Max, Min};
//!
//! // Range Sum Query
//! let mut seg: SegTree<Sum> = SegTree::new(5);
//! seg.set(0, Sum(3));
//! seg.set(1, Sum(1));
//! seg.set(2, Sum(4));
//! assert_eq!(seg.query(0, 3).0, 8);  // 3 + 1 + 4
//!
//! // Range Maximum Query
//! let v: Vec<Max> = vec![Max(3), Max(1), Max(4), Max(1), Max(5)];
//! let seg = SegTree::from_vec(&v);
//! assert_eq!(seg.query(0, 5).0, 5);
//! ```

/// Monoid trait for Segment Tree
///
/// A monoid satisfies:
/// - Identity: `op(identity, x) = op(x, identity) = x`
/// - Associativity: `op(op(a, b), c) = op(a, op(b, c))`
pub trait Monoid {
    /// Returns the identity element
    fn identity() -> Self;
    /// Binary operation
    fn op(&self, other: &Self) -> Self;
}

/// Segment Tree with Monoid trait
///
/// # Complexity
/// - Construction: O(N)
/// - Query: O(log N)
/// - Update: O(log N)
pub struct SegTree<M: Monoid + Clone> {
    size: usize,
    data: Vec<M>,
}

impl<M: Monoid + Clone> SegTree<M> {
    /// Create a new segment tree of size n, initialized with identity elements
    ///
    /// # Example
    /// ```
    /// use procon_lib::segtree::{SegTree, Sum};
    ///
    /// let seg: SegTree<Sum> = SegTree::new(10);
    /// assert_eq!(seg.query(0, 10).0, 0);  // all zeros
    /// ```
    pub fn new(n: usize) -> Self {
        let size = n.next_power_of_two();
        Self {
            size,
            data: vec![M::identity(); 2 * size],
        }
    }

    /// Create a segment tree from a vector
    ///
    /// # Complexity
    /// O(N)
    ///
    /// # Example
    /// ```
    /// use procon_lib::segtree::{SegTree, Sum};
    ///
    /// let v: Vec<Sum> = vec![Sum(1), Sum(2), Sum(3)];
    /// let seg = SegTree::from_vec(&v);
    /// assert_eq!(seg.query(0, 3).0, 6);
    /// ```
    pub fn from_vec(v: &[M]) -> Self {
        let n = v.len();
        let size = n.next_power_of_two();
        let mut data = vec![M::identity(); 2 * size];

        for (i, x) in v.iter().enumerate() {
            data[size + i] = x.clone();
        }
        for i in (1..size).rev() {
            data[i] = data[2 * i].op(&data[2 * i + 1]);
        }

        Self { size, data }
    }

    /// Set value at index i
    ///
    /// # Complexity
    /// O(log N)
    ///
    /// # Panics
    /// Panics if `i >= size`
    pub fn set(&mut self, mut i: usize, val: M) {
        i += self.size;
        self.data[i] = val;
        while i > 1 {
            i /= 2;
            self.data[i] = self.data[2 * i].op(&self.data[2 * i + 1]);
        }
    }

    /// Get value at index i
    ///
    /// # Complexity
    /// O(1)
    pub fn get(&self, i: usize) -> M {
        self.data[self.size + i].clone()
    }

    /// Query range [l, r)
    ///
    /// # Complexity
    /// O(log N)
    ///
    /// # Example
    /// ```
    /// use procon_lib::segtree::{SegTree, Sum};
    ///
    /// let v: Vec<Sum> = vec![Sum(1), Sum(2), Sum(3), Sum(4), Sum(5)];
    /// let seg = SegTree::from_vec(&v);
    /// assert_eq!(seg.query(1, 4).0, 9);  // 2 + 3 + 4
    /// ```
    pub fn query(&self, mut l: usize, mut r: usize) -> M {
        let mut left = M::identity();
        let mut right = M::identity();
        l += self.size;
        r += self.size;

        while l < r {
            if l & 1 == 1 {
                left = left.op(&self.data[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                right = self.data[r].op(&right);
            }
            l /= 2;
            r /= 2;
        }

        left.op(&right)
    }

    /// Query all elements
    ///
    /// # Complexity
    /// O(1)
    pub fn all_query(&self) -> M {
        self.data[1].clone()
    }

    /// Find the rightmost position where f(query(l, r)) is true
    ///
    /// Binary search on segment tree.
    ///
    /// # Complexity
    /// O(log N)
    pub fn max_right<F>(&self, l: usize, f: F) -> usize
    where
        F: Fn(&M) -> bool,
    {
        if l >= self.size {
            return self.size;
        }

        let mut l = l + self.size;
        let mut sm = M::identity();

        loop {
            while l % 2 == 0 {
                l /= 2;
            }
            if !f(&sm.op(&self.data[l])) {
                while l < self.size {
                    l *= 2;
                    if f(&sm.op(&self.data[l])) {
                        sm = sm.op(&self.data[l]);
                        l += 1;
                    }
                }
                return l - self.size;
            }
            sm = sm.op(&self.data[l]);
            l += 1;
            if l & (l.wrapping_neg()) == l {
                break;
            }
        }
        self.size
    }

    /// Find the leftmost position where f(query(l, r)) is true
    ///
    /// # Complexity
    /// O(log N)
    pub fn min_left<F>(&self, r: usize, f: F) -> usize
    where
        F: Fn(&M) -> bool,
    {
        if r == 0 {
            return 0;
        }

        let mut r = r + self.size;
        let mut sm = M::identity();

        loop {
            r -= 1;
            while r > 1 && r % 2 == 1 {
                r /= 2;
            }
            if !f(&self.data[r].op(&sm)) {
                while r < self.size {
                    r = 2 * r + 1;
                    if f(&self.data[r].op(&sm)) {
                        sm = self.data[r].op(&sm);
                        r -= 1;
                    }
                }
                return r + 1 - self.size;
            }
            sm = self.data[r].op(&sm);
            if r & (r.wrapping_neg()) == r {
                break;
            }
        }
        0
    }
}

impl<M: Monoid + Clone> FromIterator<M> for SegTree<M> {
    fn from_iter<I: IntoIterator<Item = M>>(iter: I) -> Self {
        let v: Vec<M> = iter.into_iter().collect();
        Self::from_vec(&v)
    }
}

// =============================================================================
// Common Monoid implementations
// =============================================================================

/// Maximum monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Max(pub i64);

impl Monoid for Max {
    fn identity() -> Self {
        Max(i64::MIN)
    }
    fn op(&self, other: &Self) -> Self {
        Max(self.0.max(other.0))
    }
}

/// Minimum monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Min(pub i64);

impl Monoid for Min {
    fn identity() -> Self {
        Min(i64::MAX)
    }
    fn op(&self, other: &Self) -> Self {
        Min(self.0.min(other.0))
    }
}

/// Sum monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum(pub i64);

impl Monoid for Sum {
    fn identity() -> Self {
        Sum(0)
    }
    fn op(&self, other: &Self) -> Self {
        Sum(self.0 + other.0)
    }
}

/// Product monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prod(pub i64);

impl Monoid for Prod {
    fn identity() -> Self {
        Prod(1)
    }
    fn op(&self, other: &Self) -> Self {
        Prod(self.0 * other.0)
    }
}

/// GCD monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gcd(pub i64);

impl Monoid for Gcd {
    fn identity() -> Self {
        Gcd(0)
    }
    fn op(&self, other: &Self) -> Self {
        fn gcd(a: i64, b: i64) -> i64 {
            if b == 0 {
                a
            } else {
                gcd(b, a % b)
            }
        }
        Gcd(gcd(self.0, other.0))
    }
}

/// XOR monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xor(pub i64);

impl Monoid for Xor {
    fn identity() -> Self {
        Xor(0)
    }
    fn op(&self, other: &Self) -> Self {
        Xor(self.0 ^ other.0)
    }
}

/// AND monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct And(pub i64);

impl Monoid for And {
    fn identity() -> Self {
        And(!0) // all bits set
    }
    fn op(&self, other: &Self) -> Self {
        And(self.0 & other.0)
    }
}

/// OR monoid
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Or(pub i64);

impl Monoid for Or {
    fn identity() -> Self {
        Or(0)
    }
    fn op(&self, other: &Self) -> Self {
        Or(self.0 | other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_segtree() {
        let mut seg: SegTree<Sum> = SegTree::new(8);
        seg.set(0, Sum(1));
        seg.set(1, Sum(2));
        seg.set(2, Sum(3));
        seg.set(3, Sum(4));

        assert_eq!(seg.query(0, 4).0, 10);
        assert_eq!(seg.query(1, 3).0, 5);
        assert_eq!(seg.query(0, 1).0, 1);
        assert_eq!(seg.all_query().0, 10);
    }

    #[test]
    fn test_max_segtree() {
        let v: Vec<Max> = vec![Max(3), Max(1), Max(4), Max(1), Max(5)];
        let seg = SegTree::from_vec(&v);

        assert_eq!(seg.query(0, 5).0, 5);
        assert_eq!(seg.query(0, 3).0, 4);
        assert_eq!(seg.query(2, 4).0, 4);
    }

    #[test]
    fn test_min_segtree() {
        let v: Vec<Min> = vec![Min(3), Min(1), Min(4), Min(1), Min(5)];
        let seg = SegTree::from_vec(&v);

        assert_eq!(seg.query(0, 5).0, 1);
        assert_eq!(seg.query(2, 5).0, 1);
        assert_eq!(seg.query(0, 1).0, 3);
    }

    #[test]
    fn test_gcd_segtree() {
        let v: Vec<Gcd> = vec![Gcd(12), Gcd(18), Gcd(24)];
        let seg = SegTree::from_vec(&v);

        assert_eq!(seg.query(0, 3).0, 6);
        assert_eq!(seg.query(0, 2).0, 6);
        assert_eq!(seg.query(1, 3).0, 6);
    }

    #[test]
    fn test_xor_segtree() {
        let v: Vec<Xor> = vec![Xor(1), Xor(2), Xor(3)];
        let seg = SegTree::from_vec(&v);

        assert_eq!(seg.query(0, 3).0, 0); // 1 ^ 2 ^ 3 = 0
        assert_eq!(seg.query(0, 2).0, 3); // 1 ^ 2 = 3
    }

    #[test]
    fn test_from_iterator() {
        let seg: SegTree<Sum> = (0..5).map(|i| Sum(i)).collect();
        assert_eq!(seg.query(0, 5).0, 10); // 0 + 1 + 2 + 3 + 4
    }

    #[test]
    fn test_max_right() {
        let v: Vec<Sum> = vec![Sum(1), Sum(2), Sum(3), Sum(4), Sum(5)];
        let seg = SegTree::from_vec(&v);

        // Find rightmost position where sum < 10
        let r = seg.max_right(0, |x| x.0 < 10);
        assert_eq!(r, 4); // sum(0..4) = 10, so we stop at 4
    }

    #[test]
    fn test_min_left() {
        let v: Vec<Sum> = vec![Sum(1), Sum(2), Sum(3), Sum(4), Sum(5)];
        let seg = SegTree::from_vec(&v);

        // Find leftmost position where sum < 10
        let l = seg.min_left(5, |x| x.0 < 10);
        assert_eq!(l, 2); // sum(2..5) = 12 >= 10, sum(3..5) = 9 < 10
    }
}
