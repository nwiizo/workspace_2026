//! データ構造

/// Binary Indexed Tree (Fenwick Tree)
///
/// 点更新・区間和クエリを O(log N) で処理
///
/// # Example
/// ```
/// use typical90::data_structures::Bit;
///
/// let mut bit = Bit::new(10);
/// bit.add(3, 5);  // index 3 に 5 を加算
/// bit.add(7, 3);  // index 7 に 3 を加算
///
/// assert_eq!(bit.prefix_sum(5), 5);   // [0,5) の和
/// assert_eq!(bit.prefix_sum(10), 8);  // [0,10) の和
/// assert_eq!(bit.sum(3, 7), 8);       // [3,7] の和 = 5 + 3
/// assert_eq!(bit.sum(4, 6), 0);       // [4,6] の和 = 0
/// ```
#[derive(Clone)]
pub struct Bit {
    data: Vec<i64>,
}

impl Bit {
    pub fn new(n: usize) -> Self {
        Self {
            data: vec![0; n + 1],
        }
    }

    /// index i に x を加算
    pub fn add(&mut self, mut i: usize, x: i64) {
        i += 1;
        while i < self.data.len() {
            self.data[i] += x;
            i += i & i.wrapping_neg();
        }
    }

    /// [0, i) の和を取得
    pub fn prefix_sum(&self, mut i: usize) -> i64 {
        let mut s = 0;
        while i > 0 {
            s += self.data[i];
            i -= i & i.wrapping_neg();
        }
        s
    }

    /// [l, r] の和を取得
    pub fn sum(&self, l: usize, r: usize) -> i64 {
        if l > r {
            return 0;
        }
        self.prefix_sum(r + 1) - self.prefix_sum(l)
    }
}

/// セグメント木
///
/// 区間クエリと点更新を O(log N) で処理
///
/// # Example
/// ```
/// use typical90::data_structures::SegTree;
///
/// // Range Minimum Query
/// let mut seg = SegTree::new(5, i64::MAX, |a, b| a.min(b));
/// seg.set(1, 3);
/// seg.set(3, 1);
/// seg.set(4, 4);
///
/// assert_eq!(seg.query(0, 5), 1);  // 全区間の最小
/// assert_eq!(seg.query(0, 3), 3);  // [0,3) の最小
/// assert_eq!(seg.query(1, 2), 3);  // [1,2) の最小
/// ```
#[derive(Clone)]
pub struct SegTree<T, F> {
    n: usize,
    data: Vec<T>,
    identity: T,
    op: F,
}

impl<T: Clone + Copy, F: Fn(T, T) -> T> SegTree<T, F> {
    pub fn new(size: usize, identity: T, op: F) -> Self {
        let mut n = 1;
        while n < size {
            n *= 2;
        }
        Self {
            n,
            data: vec![identity; 2 * n],
            identity,
            op,
        }
    }

    /// 配列から構築
    pub fn from_vec(v: &[T], identity: T, op: F) -> Self {
        let mut seg = Self::new(v.len(), identity, op);
        for (i, &x) in v.iter().enumerate() {
            seg.data[seg.n + i] = x;
        }
        for i in (1..seg.n).rev() {
            seg.data[i] = (seg.op)(seg.data[2 * i], seg.data[2 * i + 1]);
        }
        seg
    }

    /// index i に値 x を設定
    pub fn set(&mut self, i: usize, x: T) {
        let mut i = i + self.n;
        self.data[i] = x;
        while i > 1 {
            i /= 2;
            self.data[i] = (self.op)(self.data[2 * i], self.data[2 * i + 1]);
        }
    }

    /// index i の値を取得
    pub fn get(&self, i: usize) -> T {
        self.data[self.n + i]
    }

    /// [l, r) の区間クエリ
    pub fn query(&self, mut l: usize, mut r: usize) -> T {
        let mut left_val = self.identity;
        let mut right_val = self.identity;
        l += self.n;
        r += self.n;

        while l < r {
            if l & 1 == 1 {
                left_val = (self.op)(left_val, self.data[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                right_val = (self.op)(self.data[r], right_val);
            }
            l /= 2;
            r /= 2;
        }
        (self.op)(left_val, right_val)
    }
}

/// 遅延評価セグメント木
///
/// 区間更新・区間クエリを O(log N) で処理
///
/// # Example
/// ```
/// use typical90::data_structures::LazySegTree;
///
/// // Range Add, Range Sum
/// let mut seg = LazySegTree::new(
///     5,
///     0i64,                  // identity for query
///     0i64,                  // identity for lazy
///     |a, b| a + b,          // query operation
///     |a, b| a + b,          // lazy composition
///     |a, b, len| a + b * len as i64,  // apply lazy
/// );
///
/// seg.apply(0, 3, 10);  // [0,3) に 10 を加算
/// seg.apply(2, 5, 5);   // [2,5) に 5 を加算
///
/// assert_eq!(seg.query(0, 5), 45);  // 10+10+15+5+5 = 45
/// assert_eq!(seg.query(0, 2), 20);  // 10+10 = 20
/// ```
#[derive(Clone)]
pub struct LazySegTree<T, L, Op, Comp, Apply> {
    n: usize,
    data: Vec<T>,
    lazy: Vec<L>,
    identity: T,
    lazy_identity: L,
    op: Op,
    compose: Comp,
    apply_fn: Apply,
}

impl<T, L, Op, Comp, Apply> LazySegTree<T, L, Op, Comp, Apply>
where
    T: Clone + Copy,
    L: Clone + Copy + PartialEq,
    Op: Fn(T, T) -> T,
    Comp: Fn(L, L) -> L,
    Apply: Fn(T, L, usize) -> T,
{
    pub fn new(
        size: usize,
        identity: T,
        lazy_identity: L,
        op: Op,
        compose: Comp,
        apply_fn: Apply,
    ) -> Self {
        let mut n = 1;
        while n < size {
            n *= 2;
        }
        Self {
            n,
            data: vec![identity; 2 * n],
            lazy: vec![lazy_identity; 2 * n],
            identity,
            lazy_identity,
            op,
            compose,
            apply_fn,
        }
    }

    fn push(&mut self, k: usize, len: usize) {
        if self.lazy[k] != self.lazy_identity {
            self.data[k] = (self.apply_fn)(self.data[k], self.lazy[k], len);
            if k < self.n {
                self.lazy[2 * k] = (self.compose)(self.lazy[2 * k], self.lazy[k]);
                self.lazy[2 * k + 1] = (self.compose)(self.lazy[2 * k + 1], self.lazy[k]);
            }
            self.lazy[k] = self.lazy_identity;
        }
    }

    fn update(&mut self, k: usize) {
        self.data[k] = (self.op)(self.data[2 * k], self.data[2 * k + 1]);
    }

    /// [l, r) に x を適用
    pub fn apply(&mut self, l: usize, r: usize, x: L) {
        self.apply_inner(1, 0, self.n, l, r, x);
    }

    fn apply_inner(&mut self, k: usize, node_l: usize, node_r: usize, l: usize, r: usize, x: L) {
        self.push(k, node_r - node_l);
        if r <= node_l || node_r <= l {
            return;
        }
        if l <= node_l && node_r <= r {
            self.lazy[k] = (self.compose)(self.lazy[k], x);
            self.push(k, node_r - node_l);
            return;
        }
        let mid = (node_l + node_r) / 2;
        self.apply_inner(2 * k, node_l, mid, l, r, x);
        self.apply_inner(2 * k + 1, mid, node_r, l, r, x);
        self.update(k);
    }

    /// [l, r) の区間クエリ
    pub fn query(&mut self, l: usize, r: usize) -> T {
        self.query_inner(1, 0, self.n, l, r)
    }

    fn query_inner(&mut self, k: usize, node_l: usize, node_r: usize, l: usize, r: usize) -> T {
        self.push(k, node_r - node_l);
        if r <= node_l || node_r <= l {
            return self.identity;
        }
        if l <= node_l && node_r <= r {
            return self.data[k];
        }
        let mid = (node_l + node_r) / 2;
        let left_val = self.query_inner(2 * k, node_l, mid, l, r);
        let right_val = self.query_inner(2 * k + 1, mid, node_r, l, r);
        (self.op)(left_val, right_val)
    }
}

/// 組み合わせ計算用の前計算テーブル
#[derive(Clone)]
pub struct Combination {
    fact: Vec<i64>,
    inv_fact: Vec<i64>,
    modulo: i64,
}

impl Combination {
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

    /// nCr を計算
    pub fn comb(&self, n: usize, r: usize) -> i64 {
        if n < r {
            return 0;
        }
        self.fact[n] * self.inv_fact[r] % self.modulo * self.inv_fact[n - r] % self.modulo
    }

    /// nPr を計算
    pub fn perm(&self, n: usize, r: usize) -> i64 {
        if n < r {
            return 0;
        }
        self.fact[n] * self.inv_fact[n - r] % self.modulo
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
    fn test_combination() {
        let comb = Combination::new(10, 1_000_000_007);

        assert_eq!(comb.comb(5, 2), 10);
        assert_eq!(comb.comb(10, 3), 120);
        assert_eq!(comb.comb(5, 0), 1);
        assert_eq!(comb.comb(5, 5), 1);
        assert_eq!(comb.comb(3, 5), 0); // n < r

        assert_eq!(comb.perm(5, 2), 20);
        assert_eq!(comb.perm(5, 5), 120);
    }
}
