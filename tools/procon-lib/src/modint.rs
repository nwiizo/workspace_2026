//! ModInt - Automatic modular arithmetic
//!
//! # Example
//! ```
//! use procon_lib::modint::{ModInt, Mint998, Mint107};
//!
//! let a = Mint998::new(123456789);
//! let b = Mint998::new(987654321);
//!
//! // Automatic mod calculation
//! let c = a + b;
//! let d = a * b;
//! let e = a / b;  // Uses modular inverse
//! let f = a.pow(1000);
//!
//! // Different mod
//! let x = Mint107::new(123);
//! ```

use std::fmt;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

/// Common modulo constants
pub const MOD: i64 = 998_244_353;
pub const MOD2: i64 = 1_000_000_007;

/// ModInt with compile-time modulo
///
/// # Type Parameter
/// - `M`: The modulo value
///
/// # Example
/// ```
/// use procon_lib::modint::ModInt;
///
/// type Mint = ModInt<998244353>;
/// let a = Mint::new(5);
/// let b = Mint::new(3);
/// assert_eq!((a + b).val(), 8);
/// assert_eq!((a * b).val(), 15);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct ModInt<const M: i64> {
    val: i64,
}

impl<const M: i64> ModInt<M> {
    /// Create a new ModInt
    ///
    /// # Example
    /// ```
    /// use procon_lib::modint::ModInt;
    ///
    /// type Mint = ModInt<1000000007>;
    /// let a = Mint::new(123);
    /// let b = Mint::new(-5);  // Handles negative numbers
    /// assert_eq!(b.val(), 1000000002);
    /// ```
    pub fn new(val: i64) -> Self {
        let val = val % M;
        Self {
            val: if val < 0 { val + M } else { val },
        }
    }

    /// Create a ModInt from raw value (assumed to be already in [0, M))
    ///
    /// # Safety
    /// The caller must ensure that `val` is in the range `[0, M)`.
    pub const fn raw(val: i64) -> Self {
        Self { val }
    }

    /// Get the internal value
    pub const fn val(&self) -> i64 {
        self.val
    }

    /// Get the modulo
    pub const fn modulo() -> i64 {
        M
    }

    /// Calculate power using binary exponentiation
    ///
    /// # Complexity
    /// O(log exp)
    ///
    /// # Example
    /// ```
    /// use procon_lib::modint::ModInt;
    ///
    /// type Mint = ModInt<1000000007>;
    /// let a = Mint::new(2);
    /// assert_eq!(a.pow(10).val(), 1024);
    /// ```
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

    /// Calculate modular inverse
    ///
    /// Assumes M is prime (uses Fermat's little theorem).
    ///
    /// # Complexity
    /// O(log M)
    ///
    /// # Example
    /// ```
    /// use procon_lib::modint::ModInt;
    ///
    /// type Mint = ModInt<1000000007>;
    /// let a = Mint::new(2);
    /// let inv_a = a.inv();
    /// assert_eq!((a * inv_a).val(), 1);
    /// ```
    pub fn inv(&self) -> Self {
        self.pow(M - 2)
    }
}

// =============================================================================
// Conversions
// =============================================================================

impl<const M: i64> From<i64> for ModInt<M> {
    fn from(val: i64) -> Self {
        Self::new(val)
    }
}

impl<const M: i64> From<i32> for ModInt<M> {
    fn from(val: i32) -> Self {
        Self::new(val as i64)
    }
}

impl<const M: i64> From<usize> for ModInt<M> {
    fn from(val: usize) -> Self {
        Self::new(val as i64)
    }
}

impl<const M: i64> From<u64> for ModInt<M> {
    fn from(val: u64) -> Self {
        Self::new((val % M as u64) as i64)
    }
}

impl<const M: i64> FromStr for ModInt<M> {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let val: i64 = s.parse()?;
        Ok(Self::new(val))
    }
}

// =============================================================================
// Arithmetic Operations
// =============================================================================

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

// =============================================================================
// Iterator traits
// =============================================================================

impl<const M: i64> Sum for ModInt<M> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::new(0), |acc, x| acc + x)
    }
}

impl<'a, const M: i64> Sum<&'a Self> for ModInt<M> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::new(0), |acc, &x| acc + x)
    }
}

impl<const M: i64> Product for ModInt<M> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::new(1), |acc, x| acc * x)
    }
}

impl<'a, const M: i64> Product<&'a Self> for ModInt<M> {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::new(1), |acc, &x| acc * x)
    }
}

// =============================================================================
// Display
// =============================================================================

impl<const M: i64> fmt::Display for ModInt<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val)
    }
}

// =============================================================================
// Type aliases
// =============================================================================

/// ModInt with mod = 998244353 (NTT-friendly)
pub type Mint998 = ModInt<998_244_353>;

/// ModInt with mod = 10^9 + 7
pub type Mint107 = ModInt<1_000_000_007>;

// =============================================================================
// Operations with integers
// =============================================================================

macro_rules! impl_int_ops {
    ($($t:ty),*) => {
        $(
            impl<const M: i64> Add<$t> for ModInt<M> {
                type Output = Self;
                fn add(self, rhs: $t) -> Self {
                    self + Self::new(rhs as i64)
                }
            }

            impl<const M: i64> Sub<$t> for ModInt<M> {
                type Output = Self;
                fn sub(self, rhs: $t) -> Self {
                    self - Self::new(rhs as i64)
                }
            }

            impl<const M: i64> Mul<$t> for ModInt<M> {
                type Output = Self;
                fn mul(self, rhs: $t) -> Self {
                    self * Self::new(rhs as i64)
                }
            }

            impl<const M: i64> Div<$t> for ModInt<M> {
                type Output = Self;
                fn div(self, rhs: $t) -> Self {
                    self / Self::new(rhs as i64)
                }
            }
        )*
    };
}

impl_int_ops!(i32, i64, usize);

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
        let a: Mint998 = ModInt::new(MOD - 1);
        let b: Mint998 = ModInt::new(2);

        assert_eq!((a + b).val(), 1);
    }

    #[test]
    fn test_negative() {
        let a: Mint998 = ModInt::new(-1);
        assert_eq!(a.val(), MOD - 1);

        let b: Mint998 = ModInt::new(-MOD - 5);
        assert_eq!(b.val(), MOD - 5);
    }

    #[test]
    fn test_pow() {
        let a: Mint998 = ModInt::new(2);
        assert_eq!(a.pow(10).val(), 1024);
        assert_eq!(a.pow(0).val(), 1);
    }

    #[test]
    fn test_inv() {
        let a: Mint998 = ModInt::new(2);
        let inv_a = a.inv();
        assert_eq!((a * inv_a).val(), 1);

        let b: Mint998 = ModInt::new(12345);
        let inv_b = b.inv();
        assert_eq!((b * inv_b).val(), 1);
    }

    #[test]
    fn test_div() {
        let a: Mint998 = ModInt::new(10);
        let b: Mint998 = ModInt::new(2);
        assert_eq!((a / b).val(), 5);
    }

    #[test]
    fn test_from_str() {
        let a: Mint998 = "123".parse().unwrap();
        assert_eq!(a.val(), 123);
    }

    #[test]
    fn test_sum() {
        let v: Vec<Mint998> = vec![1.into(), 2.into(), 3.into(), 4.into(), 5.into()];
        let sum: Mint998 = v.iter().sum();
        assert_eq!(sum.val(), 15);

        let sum2: Mint998 = v.into_iter().sum();
        assert_eq!(sum2.val(), 15);
    }

    #[test]
    fn test_product() {
        let v: Vec<Mint998> = vec![1.into(), 2.into(), 3.into(), 4.into(), 5.into()];
        let prod: Mint998 = v.iter().product();
        assert_eq!(prod.val(), 120);
    }

    #[test]
    fn test_int_ops() {
        let a: Mint998 = ModInt::new(10);
        assert_eq!((a + 5i64).val(), 15);
        assert_eq!((a - 3i64).val(), 7);
        assert_eq!((a * 2i64).val(), 20);
        assert_eq!((a / 2i64).val(), 5);
    }

    #[test]
    fn test_mint107() {
        let a: Mint107 = ModInt::new(MOD2 - 1);
        let b: Mint107 = ModInt::new(2);
        assert_eq!((a + b).val(), 1);
    }
}
