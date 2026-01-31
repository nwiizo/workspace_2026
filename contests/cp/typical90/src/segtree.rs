//! セグメント木
//!
//! 点更新・区間クエリを O(log N) で処理

/// モノイドの trait
pub trait Monoid {
    fn identity() -> Self;
    fn op(&self, other: &Self) -> Self;
}

/// セグメント木
pub struct SegTree<M: Monoid + Clone> {
    size: usize,
    data: Vec<M>,
}

impl<M: Monoid + Clone> SegTree<M> {
    /// 長さ n のセグ木を単位元で初期化
    pub fn new(n: usize) -> Self {
        let size = n.next_power_of_two();
        Self {
            size,
            data: vec![M::identity(); 2 * size],
        }
    }

    /// 配列から構築
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

    /// index i の値を更新
    pub fn set(&mut self, mut i: usize, val: M) {
        i += self.size;
        self.data[i] = val;
        while i > 1 {
            i /= 2;
            self.data[i] = self.data[2 * i].op(&self.data[2 * i + 1]);
        }
    }

    /// index i の値を取得
    pub fn get(&self, i: usize) -> M {
        self.data[self.size + i].clone()
    }

    /// 区間 [l, r) のクエリ
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
}

// よく使うモノイド実装
#[derive(Clone, Copy, Debug)]
pub struct Max(pub i64);

impl Monoid for Max {
    fn identity() -> Self {
        Max(i64::MIN)
    }
    fn op(&self, other: &Self) -> Self {
        Max(self.0.max(other.0))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Min(pub i64);

impl Monoid for Min {
    fn identity() -> Self {
        Min(i64::MAX)
    }
    fn op(&self, other: &Self) -> Self {
        Min(self.0.min(other.0))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sum(pub i64);

impl Monoid for Sum {
    fn identity() -> Self {
        Sum(0)
    }
    fn op(&self, other: &Self) -> Self {
        Sum(self.0 + other.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Gcd(pub i64);

impl Monoid for Gcd {
    fn identity() -> Self {
        Gcd(0)
    }
    fn op(&self, other: &Self) -> Self {
        fn gcd(a: i64, b: i64) -> i64 {
            if b == 0 { a } else { gcd(b, a % b) }
        }
        Gcd(gcd(self.0, other.0))
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
}
