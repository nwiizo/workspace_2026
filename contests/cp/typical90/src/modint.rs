//! ModInt - 自動でmod計算を行う整数型

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub const MOD: i64 = 998_244_353;
pub const MOD2: i64 = 1_000_000_007;

/// ModInt (固定mod)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ModInt<const M: i64> {
    val: i64,
}

impl<const M: i64> ModInt<M> {
    pub fn new(val: i64) -> Self {
        let val = val % M;
        Self {
            val: if val < 0 { val + M } else { val },
        }
    }

    pub fn val(&self) -> i64 {
        self.val
    }

    pub fn pow(&self, mut exp: i64) -> Self {
        let mut base = *self;
        let mut result = Self::new(1);
        while exp > 0 {
            if exp & 1 == 1 {
                result *= base;
            }
            base *= base;
            exp >>= 1;
        }
        result
    }

    pub fn inv(&self) -> Self {
        self.pow(M - 2)
    }
}

impl<const M: i64> From<i64> for ModInt<M> {
    fn from(val: i64) -> Self {
        Self::new(val)
    }
}

impl<const M: i64> Add for ModInt<M> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let val = self.val + rhs.val;
        Self {
            val: if val >= M { val - M } else { val },
        }
    }
}

impl<const M: i64> AddAssign for ModInt<M> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<const M: i64> Sub for ModInt<M> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let val = self.val - rhs.val;
        Self {
            val: if val < 0 { val + M } else { val },
        }
    }
}

impl<const M: i64> SubAssign for ModInt<M> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<const M: i64> Mul for ModInt<M> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.val * rhs.val)
    }
}

impl<const M: i64> MulAssign for ModInt<M> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<const M: i64> Div for ModInt<M> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self * rhs.inv()
    }
}

impl<const M: i64> DivAssign for ModInt<M> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<const M: i64> Neg for ModInt<M> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.val)
    }
}

impl<const M: i64> std::fmt::Display for ModInt<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

/// よく使うmod
pub type Mint998 = ModInt<998_244_353>;
pub type Mint107 = ModInt<1_000_000_007>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ops() {
        let a: Mint998 = ModInt::new(5);
        let b: Mint998 = ModInt::new(3);

        assert_eq!((a + b).val(), 8);
        assert_eq!((a - b).val(), 2);
        assert_eq!((a * b).val(), 15);
    }

    #[test]
    fn test_mod_overflow() {
        let m = MOD;
        let a: Mint998 = ModInt::new(m - 1);
        let b: Mint998 = ModInt::new(2);

        assert_eq!((a + b).val(), 1);
    }

    #[test]
    fn test_negative() {
        let a: Mint998 = ModInt::new(-1);
        assert_eq!(a.val(), MOD - 1);
    }

    #[test]
    fn test_pow() {
        let a: Mint998 = ModInt::new(2);
        assert_eq!(a.pow(10).val(), 1024);
    }

    #[test]
    fn test_inv() {
        let a: Mint998 = ModInt::new(2);
        let inv_a = a.inv();
        assert_eq!((a * inv_a).val(), 1);
    }

    #[test]
    fn test_div() {
        let a: Mint998 = ModInt::new(10);
        let b: Mint998 = ModInt::new(2);
        assert_eq!((a / b).val(), 5);
    }
}
