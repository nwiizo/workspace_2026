//! 数学アルゴリズム

/// 最大公約数
pub fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

/// 最小公倍数
pub fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a, b) * b
}

/// 繰り返し二乗法 (mod計算)
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

/// モジュラ逆元 (m が素数の場合)
pub fn mod_inv(a: i64, m: i64) -> i64 {
    mod_pow(a, m - 2, m)
}

/// 行列の積 (mod計算)
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

/// 行列の累乗 (mod計算)
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

/// 定数
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
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(3, 7), 21);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(2, 10, MOD2), 1024);
        assert_eq!(mod_pow(3, 3, 10), 7); // 27 % 10 = 7
    }

    #[test]
    fn test_mod_inv() {
        // 2 * mod_inv(2, p) ≡ 1 (mod p)
        let inv2 = mod_inv(2, MOD2);
        assert_eq!((2 * inv2) % MOD2, 1);
    }

    #[test]
    fn test_matrix_pow() {
        // フィボナッチ行列
        // [[1,1],[1,0]]^n の [0][0] = F(n+1)
        let fib_mat = vec![vec![1i64, 1], vec![1, 0]];
        let result = matrix_pow(&fib_mat, 10, MOD2);
        // F(11) = 89
        assert_eq!(result[0][0], 89);
    }
}
