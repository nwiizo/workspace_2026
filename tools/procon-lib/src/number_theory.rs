//! Number Theory
//!
//! - Extended GCD
//! - Chinese Remainder Theorem (CRT)
//! - Modular inverse (general)

/// Extended GCD
///
/// Returns (g, x, y) where g = gcd(a, b) and ax + by = g.
///
/// # Example
/// ```
/// use procon_lib::number_theory::extgcd;
///
/// let (g, x, y) = extgcd(12, 18);
/// assert_eq!(g, 6);
/// assert_eq!(12 * x + 18 * y, 6);
/// ```
pub fn extgcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        (a, 1, 0)
    } else {
        let (g, x, y) = extgcd(b, a % b);
        (g, y, x - (a / b) * y)
    }
}

/// Modular inverse using extended GCD
///
/// Returns x such that ax ≡ 1 (mod m).
/// Works for any m (not necessarily prime), as long as gcd(a, m) = 1.
///
/// Returns None if inverse doesn't exist.
///
/// # Example
/// ```
/// use procon_lib::number_theory::mod_inv_general;
///
/// let inv = mod_inv_general(3, 7).unwrap();
/// assert_eq!((3 * inv) % 7, 1);
///
/// // No inverse when gcd(a, m) > 1
/// assert!(mod_inv_general(4, 8).is_none());
/// ```
pub fn mod_inv_general(a: i64, m: i64) -> Option<i64> {
    let (g, x, _) = extgcd(a, m);
    if g != 1 {
        None
    } else {
        Some(x.rem_euclid(m))
    }
}

/// Chinese Remainder Theorem (pair version)
///
/// Find x such that:
/// - x ≡ r1 (mod m1)
/// - x ≡ r2 (mod m2)
///
/// Returns (remainder, modulo) or None if no solution exists.
///
/// # Example
/// ```
/// use procon_lib::number_theory::crt;
///
/// // x ≡ 2 (mod 3) and x ≡ 3 (mod 5)
/// let (r, m) = crt(2, 3, 3, 5).unwrap();
/// assert_eq!(r % 3, 2);
/// assert_eq!(r % 5, 3);
/// assert_eq!(m, 15);
/// ```
pub fn crt(r1: i64, m1: i64, r2: i64, m2: i64) -> Option<(i64, i64)> {
    let (g, p, _) = extgcd(m1, m2);

    if (r2 - r1) % g != 0 {
        return None;
    }

    let lcm = m1 / g * m2;
    let tmp = (r2 - r1) / g * p % (m2 / g);
    let r = (r1 + m1 * tmp).rem_euclid(lcm);

    Some((r, lcm))
}

/// Chinese Remainder Theorem (general version)
///
/// Find x such that x ≡ r[i] (mod m[i]) for all i.
///
/// # Example
/// ```
/// use procon_lib::number_theory::crt_general;
///
/// let remainders = vec![2, 3, 2];
/// let moduli = vec![3, 5, 7];
/// let (r, m) = crt_general(&remainders, &moduli).unwrap();
/// assert_eq!(r % 3, 2);
/// assert_eq!(r % 5, 3);
/// assert_eq!(r % 7, 2);
/// assert_eq!(m, 105);
/// ```
pub fn crt_general(remainders: &[i64], moduli: &[i64]) -> Option<(i64, i64)> {
    assert_eq!(remainders.len(), moduli.len());

    let mut r = 0i64;
    let mut m = 1i64;

    for (&ri, &mi) in remainders.iter().zip(moduli.iter()) {
        let (new_r, new_m) = crt(r, m, ri, mi)?;
        r = new_r;
        m = new_m;
    }

    Some((r, m))
}

/// inv_gcd (ACL compatible)
///
/// Returns (gcd(a, m), x) where x * a ≡ gcd(a, m) (mod m).
///
/// # Constraints
/// - 1 <= m
/// - 0 <= a < m
///
/// # Example
/// ```
/// use procon_lib::number_theory::inv_gcd;
///
/// let (g, x) = inv_gcd(3, 7);
/// assert_eq!(g, 1);
/// assert_eq!((3 * x) % 7, 1);
/// ```
pub fn inv_gcd(a: i64, m: i64) -> (i64, i64) {
    if a == 0 {
        return (m, 0);
    }

    let mut s = m;
    let mut t = a;
    let mut m0 = 0i64;
    let mut m1 = 1i64;

    while t != 0 {
        let u = s / t;
        s -= t * u;
        m0 -= m1 * u;

        std::mem::swap(&mut s, &mut t);
        std::mem::swap(&mut m0, &mut m1);
    }

    if m0 < 0 {
        m0 += m / s;
    }

    (s, m0)
}

/// Discrete logarithm (Baby-step Giant-step)
///
/// Find the smallest non-negative x such that g^x ≡ h (mod p).
/// Returns None if no solution exists.
///
/// # Constraints
/// - p must be prime
/// - g must not be 0
///
/// # Complexity
/// O(√p)
///
/// # Example
/// ```
/// use procon_lib::number_theory::discrete_log;
///
/// // 2^3 = 8 ≡ 1 (mod 7)... wait, 2^3 = 8 ≡ 1 (mod 7)
/// // Actually, let's find x where 3^x ≡ 2 (mod 5)
/// // 3^1 = 3, 3^2 = 9 ≡ 4, 3^3 = 27 ≡ 2
/// let x = discrete_log(3, 2, 5).unwrap();
/// assert_eq!(x, 3);
/// ```
pub fn discrete_log(g: i64, h: i64, p: i64) -> Option<i64> {
    use std::collections::HashMap;

    let m = (p as f64).sqrt().ceil() as i64 + 1;

    // Baby step: compute g^j for j = 0, 1, ..., m-1
    let mut baby = HashMap::new();
    let mut val = 1i64;
    for j in 0..m {
        if !baby.contains_key(&val) {
            baby.insert(val, j);
        }
        val = val * g % p;
    }

    // g^(-m) mod p
    let factor = mod_pow_internal(g, p - 1 - m, p);

    // Giant step: check if h * g^(-im) is in baby steps
    let mut gamma = h % p;
    for i in 0..m {
        if let Some(&j) = baby.get(&gamma) {
            return Some(i * m + j);
        }
        gamma = gamma * factor % p;
    }

    None
}

fn mod_pow_internal(mut base: i64, mut exp: i64, m: i64) -> i64 {
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

/// Tonelli-Shanks algorithm
///
/// Find x such that x^2 ≡ n (mod p).
/// Returns None if n is not a quadratic residue.
///
/// # Constraints
/// - p must be an odd prime
///
/// # Complexity
/// O(log^2 p)
///
/// # Example
/// ```
/// use procon_lib::number_theory::sqrt_mod;
///
/// // 4^2 = 16 ≡ 5 (mod 11)
/// let x = sqrt_mod(5, 11).unwrap();
/// assert_eq!(x * x % 11, 5);
/// ```
pub fn sqrt_mod(n: i64, p: i64) -> Option<i64> {
    let n = n.rem_euclid(p);
    if n == 0 {
        return Some(0);
    }

    // Check if n is a quadratic residue using Euler's criterion
    if mod_pow_internal(n, (p - 1) / 2, p) != 1 {
        return None;
    }

    if p % 4 == 3 {
        // Simple case
        return Some(mod_pow_internal(n, (p + 1) / 4, p));
    }

    // Find q and s such that p - 1 = q * 2^s
    let mut q = p - 1;
    let mut s = 0;
    while q % 2 == 0 {
        q /= 2;
        s += 1;
    }

    // Find a quadratic non-residue z
    let mut z = 2i64;
    while mod_pow_internal(z, (p - 1) / 2, p) != p - 1 {
        z += 1;
    }

    let mut m = s;
    let mut c = mod_pow_internal(z, q, p);
    let mut t = mod_pow_internal(n, q, p);
    let mut r = mod_pow_internal(n, (q + 1) / 2, p);

    loop {
        if t == 1 {
            return Some(r);
        }

        // Find the least i such that t^(2^i) = 1
        let mut i = 1;
        let mut temp = t * t % p;
        while temp != 1 {
            temp = temp * temp % p;
            i += 1;
        }

        // Update
        let b = mod_pow_internal(c, 1 << (m - i - 1), p);
        m = i;
        c = b * b % p;
        t = t * c % p;
        r = r * b % p;
    }
}

/// Primitive root modulo p
///
/// Find the smallest primitive root of p.
///
/// # Constraints
/// - p must be prime
///
/// # Example
/// ```
/// use procon_lib::number_theory::primitive_root;
///
/// assert_eq!(primitive_root(7), 3);  // 3^1=3, 3^2=2, 3^3=6, 3^4=4, 3^5=5, 3^6=1
/// ```
pub fn primitive_root(p: i64) -> i64 {
    if p == 2 {
        return 1;
    }

    // Factorize p - 1
    let mut factors = Vec::new();
    let mut n = p - 1;
    let mut d = 2i64;
    while d * d <= n {
        if n % d == 0 {
            factors.push(d);
            while n % d == 0 {
                n /= d;
            }
        }
        d += 1;
    }
    if n > 1 {
        factors.push(n);
    }

    // Find primitive root
    for g in 2..p {
        let mut is_primitive = true;
        for &f in &factors {
            if mod_pow_internal(g, (p - 1) / f, p) == 1 {
                is_primitive = false;
                break;
            }
        }
        if is_primitive {
            return g;
        }
    }

    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extgcd() {
        let (g, x, y) = extgcd(12, 18);
        assert_eq!(g, 6);
        assert_eq!(12 * x + 18 * y, 6);

        let (g, x, y) = extgcd(35, 15);
        assert_eq!(g, 5);
        assert_eq!(35 * x + 15 * y, 5);
    }

    #[test]
    fn test_mod_inv_general() {
        let inv = mod_inv_general(3, 7).unwrap();
        assert_eq!((3 * inv) % 7, 1);

        let inv = mod_inv_general(3, 11).unwrap();
        assert_eq!((3 * inv) % 11, 1);

        assert!(mod_inv_general(4, 8).is_none());
    }

    #[test]
    fn test_crt() {
        let (r, m) = crt(2, 3, 3, 5).unwrap();
        assert_eq!(r % 3, 2);
        assert_eq!(r % 5, 3);
        assert_eq!(m, 15);

        // No solution
        assert!(crt(0, 2, 1, 4).is_none());
    }

    #[test]
    fn test_crt_general() {
        let remainders = vec![2, 3, 2];
        let moduli = vec![3, 5, 7];
        let (r, m) = crt_general(&remainders, &moduli).unwrap();
        assert_eq!(r % 3, 2);
        assert_eq!(r % 5, 3);
        assert_eq!(r % 7, 2);
        assert_eq!(m, 105);
    }

    #[test]
    fn test_inv_gcd() {
        let (g, x) = inv_gcd(3, 7);
        assert_eq!(g, 1);
        assert_eq!((3 * x) % 7, 1);

        let (g, _) = inv_gcd(4, 8);
        assert_eq!(g, 4);
    }

    #[test]
    fn test_discrete_log() {
        // 3^3 = 27 ≡ 2 (mod 5)
        let x = discrete_log(3, 2, 5).unwrap();
        assert_eq!(mod_pow_internal(3, x, 5), 2);

        // 2^10 = 1024 ≡ 1 (mod 1023)... let's use smaller numbers
        // 2^4 = 16 ≡ 5 (mod 11)
        let x = discrete_log(2, 5, 11).unwrap();
        assert_eq!(mod_pow_internal(2, x, 11), 5);
    }

    #[test]
    fn test_sqrt_mod() {
        // x^2 ≡ 5 (mod 11)
        let x = sqrt_mod(5, 11).unwrap();
        assert_eq!(x * x % 11, 5);

        // x^2 ≡ 2 (mod 7)
        let x = sqrt_mod(2, 7).unwrap();
        assert_eq!(x * x % 7, 2);

        // No solution: 2 is not a quadratic residue mod 5
        assert!(sqrt_mod(2, 5).is_none());
    }

    #[test]
    fn test_primitive_root() {
        assert_eq!(primitive_root(7), 3);
        assert_eq!(primitive_root(11), 2);
        assert_eq!(primitive_root(13), 2);
    }
}
