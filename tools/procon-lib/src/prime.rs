//! Prime Number Algorithms
//!
//! - Sieve of Eratosthenes
//! - Miller-Rabin primality test
//! - Pollard's rho factorization
//! - Prime factorization

/// Sieve of Eratosthenes
///
/// # Complexity
/// O(N log log N)
///
/// # Example
/// ```
/// use procon_lib::prime::sieve;
///
/// let is_prime = sieve(20);
/// let primes: Vec<usize> = (0..=20).filter(|&i| is_prime[i]).collect();
/// assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19]);
/// ```
pub fn sieve(n: usize) -> Vec<bool> {
    let mut is_prime = vec![true; n + 1];
    if n >= 1 {
        is_prime[0] = false;
        is_prime[1] = false;
    }

    let mut i = 2;
    while i * i <= n {
        if is_prime[i] {
            for j in (i * i..=n).step_by(i) {
                is_prime[j] = false;
            }
        }
        i += 1;
    }

    is_prime
}

/// Get list of primes up to n
///
/// # Example
/// ```
/// use procon_lib::prime::primes_up_to;
///
/// assert_eq!(primes_up_to(20), vec![2, 3, 5, 7, 11, 13, 17, 19]);
/// ```
pub fn primes_up_to(n: usize) -> Vec<usize> {
    let is_prime = sieve(n);
    (2..=n).filter(|&i| is_prime[i]).collect()
}

/// Linear sieve with smallest prime factor
///
/// Returns (is_prime, smallest_prime_factor)
///
/// # Example
/// ```
/// use procon_lib::prime::linear_sieve;
///
/// let (is_prime, spf) = linear_sieve(20);
/// assert!(is_prime[7]);
/// assert_eq!(spf[12], 2);  // 12 = 2 * 6
/// ```
pub fn linear_sieve(n: usize) -> (Vec<bool>, Vec<usize>) {
    let mut is_prime = vec![true; n + 1];
    let mut spf = vec![0; n + 1];
    let mut primes = Vec::new();

    if n >= 1 {
        is_prime[0] = false;
        is_prime[1] = false;
    }

    for i in 2..=n {
        if is_prime[i] {
            primes.push(i);
            spf[i] = i;
        }
        for &p in &primes {
            if i * p > n {
                break;
            }
            is_prime[i * p] = false;
            spf[i * p] = p;
            if i % p == 0 {
                break;
            }
        }
    }

    (is_prime, spf)
}

/// Miller-Rabin primality test
///
/// Deterministic for n < 2^64 using specific witnesses.
///
/// # Complexity
/// O(k log^3 n) where k is number of witnesses
///
/// # Example
/// ```
/// use procon_lib::prime::is_prime;
///
/// assert!(is_prime(1_000_000_007));
/// assert!(is_prime(998244353));
/// assert!(!is_prime(1_000_000_006));
/// ```
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    // Write n-1 as 2^r * d
    let mut d = n - 1;
    let mut r = 0;
    while d % 2 == 0 {
        d /= 2;
        r += 1;
    }

    // Witnesses for deterministic test up to 2^64
    let witnesses: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

    'witness: for &a in witnesses {
        if a >= n {
            continue;
        }

        let mut x = mod_pow_u64(a, d, n);
        if x == 1 || x == n - 1 {
            continue 'witness;
        }

        for _ in 0..r - 1 {
            x = mod_mul_u64(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }

        return false;
    }

    true
}

/// Modular exponentiation for u64
fn mod_pow_u64(mut base: u64, mut exp: u64, m: u64) -> u64 {
    let mut result = 1u64;
    base %= m;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mod_mul_u64(result, base, m);
        }
        exp >>= 1;
        base = mod_mul_u64(base, base, m);
    }
    result
}

/// Modular multiplication avoiding overflow
fn mod_mul_u64(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// Pollard's rho algorithm for factorization
///
/// Finds a non-trivial factor of n.
///
/// # Example
/// ```
/// use procon_lib::prime::pollard_rho;
///
/// let n = 91u64; // 7 * 13
/// let f = pollard_rho(n);
/// assert!(f == 7 || f == 13);
/// ```
pub fn pollard_rho(n: u64) -> u64 {
    if n % 2 == 0 {
        return 2;
    }
    if is_prime(n) {
        return n;
    }

    let f = |x: u64, c: u64| -> u64 { (mod_mul_u64(x, x, n) + c) % n };

    let mut c = 1u64;
    loop {
        let mut x = 2u64;
        let mut y = 2u64;
        let mut d = 1u64;

        while d == 1 {
            x = f(x, c);
            y = f(f(y, c), c);
            d = gcd_u64(x.abs_diff(y), n);
        }

        if d != n {
            return d;
        }
        c += 1;
    }
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd_u64(b, a % b)
    }
}

/// Prime factorization using Pollard's rho
///
/// # Example
/// ```
/// use procon_lib::prime::factorize;
///
/// let factors = factorize(12);
/// assert_eq!(factors, vec![(2, 2), (3, 1)]);
///
/// let factors = factorize(1_000_000_007);
/// assert_eq!(factors, vec![(1_000_000_007, 1)]);
/// ```
pub fn factorize(mut n: u64) -> Vec<(u64, usize)> {
    if n <= 1 {
        return vec![];
    }

    let mut factors = Vec::new();

    // Trial division for small factors
    for p in [2u64, 3, 5] {
        if n % p == 0 {
            let mut count = 0;
            while n % p == 0 {
                n /= p;
                count += 1;
            }
            factors.push((p, count));
        }
    }

    // Pollard's rho for remaining factors
    fn factor_recursive(n: u64, factors: &mut Vec<(u64, usize)>) {
        if n == 1 {
            return;
        }
        if is_prime(n) {
            // Add to factors
            if let Some(last) = factors.last_mut() {
                if last.0 == n {
                    last.1 += 1;
                    return;
                }
            }
            factors.push((n, 1));
            return;
        }

        let d = pollard_rho(n);
        factor_recursive(d, factors);
        factor_recursive(n / d, factors);
    }

    factor_recursive(n, &mut factors);

    // Sort and merge
    factors.sort_by_key(|&(p, _)| p);
    let mut merged: Vec<(u64, usize)> = Vec::new();
    for (p, c) in factors {
        if let Some(last) = merged.last_mut() {
            if last.0 == p {
                last.1 += c;
                continue;
            }
        }
        merged.push((p, c));
    }

    merged
}

/// Simple factorization for small numbers
///
/// # Complexity
/// O(√n)
///
/// # Example
/// ```
/// use procon_lib::prime::factorize_simple;
///
/// let factors = factorize_simple(12);
/// assert_eq!(factors, vec![(2, 2), (3, 1)]);
/// ```
pub fn factorize_simple(mut n: i64) -> Vec<(i64, usize)> {
    let mut factors = Vec::new();
    let mut d = 2;

    while d * d <= n {
        if n % d == 0 {
            let mut count = 0;
            while n % d == 0 {
                n /= d;
                count += 1;
            }
            factors.push((d, count));
        }
        d += 1;
    }

    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

/// Get all divisors
///
/// # Example
/// ```
/// use procon_lib::prime::divisors;
///
/// let mut divs = divisors(12);
/// divs.sort();
/// assert_eq!(divs, vec![1, 2, 3, 4, 6, 12]);
/// ```
pub fn divisors(n: i64) -> Vec<i64> {
    let mut result = Vec::new();
    let mut d = 1;

    while d * d <= n {
        if n % d == 0 {
            result.push(d);
            if d != n / d {
                result.push(n / d);
            }
        }
        d += 1;
    }

    result
}

/// Euler's totient function
///
/// # Example
/// ```
/// use procon_lib::prime::euler_phi;
///
/// assert_eq!(euler_phi(12), 4);  // 1, 5, 7, 11
/// assert_eq!(euler_phi(7), 6);   // 1, 2, 3, 4, 5, 6
/// ```
pub fn euler_phi(mut n: i64) -> i64 {
    let mut result = n;
    let mut d = 2;

    while d * d <= n {
        if n % d == 0 {
            result -= result / d;
            while n % d == 0 {
                n /= d;
            }
        }
        d += 1;
    }

    if n > 1 {
        result -= result / n;
    }

    result
}

/// Mobius function
///
/// Returns 0 if n has squared prime factor, otherwise (-1)^k where k is number of prime factors.
pub fn mobius(n: i64) -> i32 {
    let factors = factorize_simple(n);
    for &(_, count) in &factors {
        if count > 1 {
            return 0;
        }
    }
    if factors.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sieve() {
        let is_prime = sieve(20);
        let primes: Vec<usize> = (0..=20).filter(|&i| is_prime[i]).collect();
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19]);
    }

    #[test]
    fn test_linear_sieve() {
        let (is_prime, spf) = linear_sieve(20);
        assert!(is_prime[7]);
        assert!(!is_prime[12]);
        assert_eq!(spf[12], 2);
        assert_eq!(spf[15], 3);
    }

    #[test]
    fn test_is_prime() {
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(1_000_000_007));
        assert!(is_prime(998244353));
        assert!(!is_prime(1_000_000_006));
    }

    #[test]
    fn test_is_prime_large() {
        // Large primes
        assert!(is_prime(1_000_000_000_000_000_003));
        // Large composite
        assert!(!is_prime(1_000_000_000_000_000_000));
    }

    #[test]
    fn test_pollard_rho() {
        let n = 91u64;
        let f = pollard_rho(n);
        assert!(f == 7 || f == 13);
        assert_eq!(n % f, 0);
    }

    #[test]
    fn test_factorize() {
        assert_eq!(factorize(12), vec![(2, 2), (3, 1)]);
        assert_eq!(factorize(1_000_000_007), vec![(1_000_000_007, 1)]);
        assert_eq!(factorize(1), vec![]);
    }

    #[test]
    fn test_divisors() {
        let mut divs = divisors(12);
        divs.sort();
        assert_eq!(divs, vec![1, 2, 3, 4, 6, 12]);
    }

    #[test]
    fn test_euler_phi() {
        assert_eq!(euler_phi(12), 4);
        assert_eq!(euler_phi(7), 6);
        assert_eq!(euler_phi(1), 1);
    }

    #[test]
    fn test_mobius() {
        assert_eq!(mobius(1), 1);
        assert_eq!(mobius(2), -1);
        assert_eq!(mobius(6), 1); // 2 * 3
        assert_eq!(mobius(4), 0); // 2^2
    }
}
