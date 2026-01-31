//! Math algorithms
//!
//! - GCD, LCM
//! - Modular arithmetic
//! - Matrix operations
//! - Floor sum (ACL compatible)

/// Greatest Common Divisor
///
/// # Example
/// ```
/// use procon_lib::math::gcd;
///
/// assert_eq!(gcd(12, 18), 6);
/// assert_eq!(gcd(17, 13), 1);
/// assert_eq!(gcd(0, 5), 5);
/// ```
pub fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// Least Common Multiple
///
/// # Example
/// ```
/// use procon_lib::math::lcm;
///
/// assert_eq!(lcm(4, 6), 12);
/// assert_eq!(lcm(3, 7), 21);
/// ```
pub fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a, b) * b
}

/// Modular exponentiation
///
/// Calculate base^exp mod m.
///
/// # Complexity
/// O(log exp)
///
/// # Example
/// ```
/// use procon_lib::math::mod_pow;
///
/// assert_eq!(mod_pow(2, 10, 1_000_000_007), 1024);
/// assert_eq!(mod_pow(3, 100, 1_000_000_007), 983553326);
/// ```
pub fn mod_pow(mut base: i64, mut exp: i64, m: i64) -> i64 {
    let mut result = 1;
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

/// Modular inverse (m must be prime)
///
/// # Example
/// ```
/// use procon_lib::math::mod_inv;
///
/// let inv2 = mod_inv(2, 1_000_000_007);
/// assert_eq!((2 * inv2) % 1_000_000_007, 1);
/// ```
pub fn mod_inv(a: i64, m: i64) -> i64 {
    mod_pow(a, m - 2, m)
}

/// Matrix multiplication with modulo
///
/// # Complexity
/// O(N^3)
#[allow(clippy::needless_range_loop)]
pub fn matrix_mul(a: &[Vec<i64>], b: &[Vec<i64>], m: i64) -> Vec<Vec<i64>> {
    let n = a.len();
    let mut c = vec![vec![0i64; n]; n];
    for i in 0..n {
        for k in 0..n {
            if a[i][k] == 0 {
                continue;
            }
            for j in 0..n {
                c[i][j] = (c[i][j] + a[i][k] * b[k][j]) % m;
            }
        }
    }
    c
}

/// Matrix exponentiation with modulo
///
/// # Complexity
/// O(N^3 log exp)
///
/// # Example
/// ```
/// use procon_lib::math::matrix_pow;
///
/// // Fibonacci: [[1,1],[1,0]]^n の [0][0] = F(n+1)
/// let fib_mat = vec![vec![1i64, 1], vec![1, 0]];
/// let result = matrix_pow(&fib_mat, 10, 1_000_000_007);
/// assert_eq!(result[0][0], 89);  // F(11)
/// ```
#[allow(clippy::needless_range_loop)]
pub fn matrix_pow(mat: &[Vec<i64>], mut exp: u64, m: i64) -> Vec<Vec<i64>> {
    let n = mat.len();
    let mut result = vec![vec![0i64; n]; n];
    for i in 0..n {
        result[i][i] = 1;
    }

    let mut base = mat.to_vec();
    while exp > 0 {
        if exp & 1 == 1 {
            result = matrix_mul(&result, &base, m);
        }
        base = matrix_mul(&base, &base, m);
        exp >>= 1;
    }
    result
}

/// Floor sum (ACL compatible)
///
/// Calculates sum_{i=0}^{n-1} floor((a*i + b) / m)
///
/// # Complexity
/// O(log m)
///
/// # Example
/// ```
/// use procon_lib::math::floor_sum;
///
/// // sum_{i=0}^{4} floor((2*i + 1) / 3) = 0 + 1 + 1 + 2 + 3 = 7
/// assert_eq!(floor_sum(5, 3, 2, 1), 7);
/// ```
pub fn floor_sum(n: i64, m: i64, a: i64, b: i64) -> i64 {
    floor_sum_signed(n, m, a, b)
}

/// Floor sum - correct implementation
///
/// Calculates sum_{i=0}^{n-1} floor((a*i + b) / m)
///
/// Uses the formula from AtCoder Library.
pub fn floor_sum_u64(n: u64, m: u64, a: u64, b: u64) -> u64 {
    fn inner(n: u64, m: u64, a: u64, b: u64) -> u64 {
        if a == 0 {
            return n * (b / m);
        }
        if a >= m {
            return n * (n - 1) / 2 * (a / m) + inner(n, m, a % m, b);
        }
        if b >= m {
            return n * (b / m) + inner(n, m, a, b % m);
        }
        let m2 = a * n + b;
        if m2 < m {
            return 0;
        }
        inner(m2 / m, a, m, m2 % m)
    }
    inner(n, m, a, b)
}

/// Safe floor sum with signed integers
///
/// Calculates sum_{i=0}^{n-1} floor((a*i + b) / m)
pub fn floor_sum_signed(n: i64, m: i64, a: i64, b: i64) -> i64 {
    assert!(n >= 0);
    assert!(m > 0);

    let mut ans = 0i64;
    let n = n as u64;
    let m = m as u64;

    let (a, a_neg) = if a >= 0 {
        (a as u64, 0i64)
    } else {
        let a2 = a.rem_euclid(m as i64) as u64;
        (a2, -(n as i64 * (n as i64 - 1) / 2 * ((a2 as i64 - a) / m as i64)))
    };

    let (b, b_neg) = if b >= 0 {
        (b as u64, 0i64)
    } else {
        let b2 = b.rem_euclid(m as i64) as u64;
        (b2, -(n as i64 * ((b2 as i64 - b) / m as i64)))
    };

    ans += a_neg + b_neg;
    ans += floor_sum_u64(n, m, a, b) as i64;
    ans
}

/// Safe division with floor (works correctly for negative numbers)
///
/// # Example
/// ```
/// use procon_lib::math::div_floor;
///
/// assert_eq!(div_floor(7, 3), 2);
/// assert_eq!(div_floor(-7, 3), -3);  // Not -2!
/// assert_eq!(div_floor(7, -3), -3);
/// ```
pub fn div_floor(a: i64, b: i64) -> i64 {
    a.div_euclid(b)
}

/// Safe modulo (works correctly for negative numbers)
///
/// # Example
/// ```
/// use procon_lib::math::mod_floor;
///
/// assert_eq!(mod_floor(7, 3), 1);
/// assert_eq!(mod_floor(-7, 3), 2);  // Not -1!
/// assert_eq!(mod_floor(-1, 5), 4);
/// ```
pub fn mod_floor(a: i64, b: i64) -> i64 {
    a.rem_euclid(b)
}

/// Ceiling division
///
/// # Example
/// ```
/// use procon_lib::math::div_ceil;
///
/// assert_eq!(div_ceil(7, 3), 3);
/// assert_eq!(div_ceil(6, 3), 2);
/// assert_eq!(div_ceil(0, 3), 0);
/// ```
pub fn div_ceil(a: i64, b: i64) -> i64 {
    (a + b - 1) / b
}

/// Constants
pub const MOD: i64 = 998_244_353;
pub const MOD2: i64 = 1_000_000_007;
pub const INF: i64 = 1_000_000_000_000_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 18), 6);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(-12, 18), 6);
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(3, 7), 21);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(2, 10, MOD2), 1024);
        assert_eq!(mod_pow(3, 3, 10), 7);
    }

    #[test]
    fn test_mod_inv() {
        let inv2 = mod_inv(2, MOD2);
        assert_eq!((2 * inv2) % MOD2, 1);
    }

    #[test]
    fn test_matrix_pow() {
        let fib_mat = vec![vec![1i64, 1], vec![1, 0]];
        let result = matrix_pow(&fib_mat, 10, MOD2);
        assert_eq!(result[0][0], 89);
    }

    #[test]
    fn test_floor_sum_u64() {
        // sum_{i=0}^{4} floor((2*i + 1) / 3)
        // i=0: floor(1/3)=0
        // i=1: floor(3/3)=1
        // i=2: floor(5/3)=1
        // i=3: floor(7/3)=2
        // i=4: floor(9/3)=3
        // total = 7
        assert_eq!(floor_sum_u64(5, 3, 2, 1), 7);
    }

    #[test]
    fn test_div_ceil() {
        assert_eq!(div_ceil(7, 3), 3);
        assert_eq!(div_ceil(6, 3), 2);
        assert_eq!(div_ceil(0, 3), 0);
    }

    #[test]
    fn test_div_floor() {
        assert_eq!(div_floor(7, 3), 2);
        assert_eq!(div_floor(-7, 3), -3);
    }

    #[test]
    fn test_mod_floor() {
        assert_eq!(mod_floor(7, 3), 1);
        assert_eq!(mod_floor(-7, 3), 2);
        assert_eq!(mod_floor(-1, 5), 4);
    }
}
