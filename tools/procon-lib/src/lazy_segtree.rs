//! Lazy Segment Tree
//!
//! Range update and range query in O(log N).
//!
//! # Example
//! ```
//! use procon_lib::lazy_segtree::LazySegTree;
//!
//! // Range Add, Range Sum
//! let mut seg = LazySegTree::range_add_range_sum(5);
//! seg.apply(0, 3, 10);  // add 10 to [0, 3)
//! seg.apply(2, 5, 5);   // add 5 to [2, 5)
//!
//! assert_eq!(seg.query(0, 5), 45);  // 10+10+15+5+5
//! ```

/// Lazy Segment Tree with generic operations
///
/// # Type Parameters
/// - `T`: Value type
/// - `L`: Lazy type
///
/// # Complexity
/// - Construction: O(N)
/// - Query: O(log N)
/// - Apply: O(log N)
pub struct LazySegTree<T, L> {
    n: usize,
    size: usize,
    log: usize,
    data: Vec<T>,
    lazy: Vec<L>,
    op: fn(T, T) -> T,
    e: fn() -> T,
    mapping: fn(L, T, usize) -> T,
    composition: fn(L, L) -> L,
    id: fn() -> L,
}

impl<T: Clone + Copy, L: Clone + Copy + PartialEq> LazySegTree<T, L> {
    /// Create a new lazy segment tree
    ///
    /// # Arguments
    /// - `n`: Size of the array
    /// - `e`: Identity for query operation
    /// - `id`: Identity for lazy operation
    /// - `op`: Query operation (e.g., add, max)
    /// - `mapping`: Apply lazy to value: `mapping(lazy, value, length)`
    /// - `composition`: Compose lazy operations: `composition(new, old)`
    pub fn new(
        n: usize,
        e: fn() -> T,
        id: fn() -> L,
        op: fn(T, T) -> T,
        mapping: fn(L, T, usize) -> T,
        composition: fn(L, L) -> L,
    ) -> Self {
        let size = n.next_power_of_two();
        let log = size.trailing_zeros() as usize;
        Self {
            n,
            size,
            log,
            data: vec![e(); 2 * size],
            lazy: vec![id(); size],
            op,
            e,
            mapping,
            composition,
            id,
        }
    }

    /// Create from a vector
    pub fn from_vec(
        v: &[T],
        e: fn() -> T,
        id: fn() -> L,
        op: fn(T, T) -> T,
        mapping: fn(L, T, usize) -> T,
        composition: fn(L, L) -> L,
    ) -> Self {
        let n = v.len();
        let size = n.next_power_of_two();
        let log = size.trailing_zeros() as usize;
        let mut data = vec![e(); 2 * size];

        for (i, &x) in v.iter().enumerate() {
            data[size + i] = x;
        }
        for i in (1..size).rev() {
            data[i] = op(data[2 * i], data[2 * i + 1]);
        }

        Self {
            n,
            size,
            log,
            data,
            lazy: vec![id(); size],
            op,
            e,
            mapping,
            composition,
            id,
        }
    }

    fn push(&mut self, k: usize) {
        let id = (self.id)();
        if self.lazy[k] != id {
            let len = self.size >> (k.ilog2() as usize + 1);
            self.all_apply(2 * k, self.lazy[k], len);
            self.all_apply(2 * k + 1, self.lazy[k], len);
            self.lazy[k] = id;
        }
    }

    fn all_apply(&mut self, k: usize, f: L, len: usize) {
        self.data[k] = (self.mapping)(f, self.data[k], len);
        if k < self.size {
            self.lazy[k] = (self.composition)(f, self.lazy[k]);
        }
    }

    fn update(&mut self, k: usize) {
        self.data[k] = (self.op)(self.data[2 * k], self.data[2 * k + 1]);
    }

    /// Set value at index i
    pub fn set(&mut self, mut i: usize, val: T) {
        i += self.size;
        for j in (1..=self.log).rev() {
            self.push(i >> j);
        }
        self.data[i] = val;
        for j in 1..=self.log {
            self.update(i >> j);
        }
    }

    /// Get value at index i
    pub fn get(&mut self, mut i: usize) -> T {
        i += self.size;
        for j in (1..=self.log).rev() {
            self.push(i >> j);
        }
        self.data[i]
    }

    /// Query range [l, r)
    pub fn query(&mut self, mut l: usize, mut r: usize) -> T {
        if l >= r {
            return (self.e)();
        }

        l += self.size;
        r += self.size;

        for i in (1..=self.log).rev() {
            if ((l >> i) << i) != l {
                self.push(l >> i);
            }
            if ((r >> i) << i) != r {
                self.push((r - 1) >> i);
            }
        }

        let mut sml = (self.e)();
        let mut smr = (self.e)();

        while l < r {
            if l & 1 == 1 {
                sml = (self.op)(sml, self.data[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                smr = (self.op)(self.data[r], smr);
            }
            l /= 2;
            r /= 2;
        }

        (self.op)(sml, smr)
    }

    /// Query all elements
    pub fn all_query(&self) -> T {
        self.data[1]
    }

    /// Apply operation to range [l, r)
    pub fn apply(&mut self, mut l: usize, mut r: usize, f: L) {
        if l >= r {
            return;
        }

        l += self.size;
        r += self.size;

        for i in (1..=self.log).rev() {
            if ((l >> i) << i) != l {
                self.push(l >> i);
            }
            if ((r >> i) << i) != r {
                self.push((r - 1) >> i);
            }
        }

        let (l2, r2) = (l, r);
        let mut len = 1;
        while l < r {
            if l & 1 == 1 {
                self.all_apply(l, f, len);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                self.all_apply(r, f, len);
            }
            l /= 2;
            r /= 2;
            len *= 2;
        }

        let (l, r) = (l2, r2);
        for i in 1..=self.log {
            if ((l >> i) << i) != l {
                self.update(l >> i);
            }
            if ((r >> i) << i) != r {
                self.update((r - 1) >> i);
            }
        }
    }

    /// Apply operation to a single element
    pub fn apply_at(&mut self, i: usize, f: L) {
        self.apply(i, i + 1, f);
    }
}

// =============================================================================
// Preset constructors
// =============================================================================

impl LazySegTree<i64, i64> {
    /// Range Add, Range Sum
    ///
    /// # Example
    /// ```
    /// use procon_lib::lazy_segtree::LazySegTree;
    ///
    /// let mut seg = LazySegTree::range_add_range_sum(5);
    /// seg.apply(0, 3, 10);
    /// assert_eq!(seg.query(0, 3), 30);
    /// ```
    pub fn range_add_range_sum(n: usize) -> Self {
        Self::new(
            n,
            || 0,
            || 0,
            |a, b| a + b,
            |f, x, len| x + f * len as i64,
            |f, g| f + g,
        )
    }

    /// Range Add, Range Min
    ///
    /// # Example
    /// ```
    /// use procon_lib::lazy_segtree::LazySegTree;
    ///
    /// let mut seg = LazySegTree::range_add_range_min(5);
    /// seg.set(0, 10);
    /// seg.set(1, 20);
    /// seg.apply(0, 2, 5);
    /// assert_eq!(seg.query(0, 2), 15);  // min(10+5, 20+5) = 15
    /// ```
    pub fn range_add_range_min(n: usize) -> Self {
        Self::new(
            n,
            || i64::MAX,
            || 0,
            |a, b| a.min(b),
            |f, x, _len| {
                if x == i64::MAX {
                    x
                } else {
                    x + f
                }
            },
            |f, g| f + g,
        )
    }

    /// Range Add, Range Max
    pub fn range_add_range_max(n: usize) -> Self {
        Self::new(
            n,
            || i64::MIN,
            || 0,
            |a, b| a.max(b),
            |f, x, _len| {
                if x == i64::MIN {
                    x
                } else {
                    x + f
                }
            },
            |f, g| f + g,
        )
    }
}

impl LazySegTree<i64, Option<i64>> {
    /// Range Update, Range Sum
    ///
    /// # Example
    /// ```
    /// use procon_lib::lazy_segtree::LazySegTree;
    ///
    /// let mut seg = LazySegTree::range_update_range_sum(5);
    /// seg.apply(0, 3, Some(10));
    /// assert_eq!(seg.query(0, 3), 30);
    /// ```
    pub fn range_update_range_sum(n: usize) -> Self {
        Self::new(
            n,
            || 0,
            || None,
            |a, b| a + b,
            |f, x, len| f.map_or(x, |v| v * len as i64),
            |f, g| f.or(g),
        )
    }

    /// Range Update, Range Min
    pub fn range_update_range_min(n: usize) -> Self {
        Self::new(
            n,
            || i64::MAX,
            || None,
            |a, b| a.min(b),
            |f, x, _len| f.unwrap_or(x),
            |f, g| f.or(g),
        )
    }

    /// Range Update, Range Max
    pub fn range_update_range_max(n: usize) -> Self {
        Self::new(
            n,
            || i64::MIN,
            || None,
            |a, b| a.max(b),
            |f, x, _len| f.unwrap_or(x),
            |f, g| f.or(g),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_add_range_sum() {
        let mut seg = LazySegTree::range_add_range_sum(5);
        seg.apply(0, 3, 10);
        seg.apply(2, 5, 5);

        assert_eq!(seg.query(0, 5), 45); // 10+10+15+5+5
        assert_eq!(seg.query(0, 2), 20); // 10+10
        assert_eq!(seg.query(2, 5), 25); // 15+5+5
    }

    #[test]
    fn test_range_add_range_min() {
        let mut seg = LazySegTree::range_add_range_min(5);
        for i in 0..5 {
            seg.set(i, (i * 10) as i64);
        }
        // values: 0, 10, 20, 30, 40
        assert_eq!(seg.query(0, 5), 0);

        seg.apply(0, 3, 100);
        // values: 100, 110, 120, 30, 40
        assert_eq!(seg.query(0, 5), 30);
        assert_eq!(seg.query(0, 3), 100);
    }

    #[test]
    fn test_range_update_range_sum() {
        let mut seg = LazySegTree::range_update_range_sum(5);
        seg.apply(0, 3, Some(10));
        assert_eq!(seg.query(0, 3), 30);
        assert_eq!(seg.query(0, 5), 30);

        seg.apply(2, 5, Some(5));
        assert_eq!(seg.query(0, 5), 35); // 10+10+5+5+5
    }

    #[test]
    fn test_point_operations() {
        let mut seg = LazySegTree::range_add_range_sum(5);
        seg.set(0, 10);
        seg.set(1, 20);
        seg.set(2, 30);

        assert_eq!(seg.get(0), 10);
        assert_eq!(seg.get(1), 20);
        assert_eq!(seg.query(0, 3), 60);
    }

    #[test]
    fn test_from_vec() {
        let v = vec![1i64, 2, 3, 4, 5];
        let mut seg = LazySegTree::from_vec(
            &v,
            || 0,
            || 0,
            |a, b| a + b,
            |f, x, len| x + f * len as i64,
            |f, g| f + g,
        );

        assert_eq!(seg.query(0, 5), 15);
        seg.apply(0, 3, 10);
        assert_eq!(seg.query(0, 5), 45);
    }
}
