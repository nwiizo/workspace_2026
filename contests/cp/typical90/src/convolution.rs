//! 畳み込み (NTT: Number Theoretic Transform)
//!
//! mod 998244353 での多項式乗算を O(N log N) で計算

/// mod 998244353 の原始根
const MOD: i64 = 998_244_353;
const PRIMITIVE_ROOT: i64 = 3;

/// mod p での累乗
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

/// Number Theoretic Transform (NTT)
///
/// # Arguments
/// * `a` - 変換対象の配列（長さは2のべき乗）
/// * `inverse` - true なら逆変換
fn ntt(a: &mut [i64], inverse: bool) {
    let n = a.len();
    if n == 1 {
        return;
    }

    // bit reversal
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }

    // Cooley-Tukey
    let mut len = 2;
    while len <= n {
        let ang = if inverse {
            (MOD - 1) / len as i64
        } else {
            (MOD - 1) - (MOD - 1) / len as i64
        };
        let wlen = mod_pow(PRIMITIVE_ROOT, ang, MOD);

        let mut i = 0;
        while i < n {
            let mut w = 1i64;
            for k in 0..len / 2 {
                let u = a[i + k];
                let v = a[i + k + len / 2] * w % MOD;
                a[i + k] = (u + v) % MOD;
                a[i + k + len / 2] = (u - v + MOD) % MOD;
                w = w * wlen % MOD;
            }
            i += len;
        }
        len *= 2;
    }

    if inverse {
        let n_inv = mod_pow(n as i64, MOD - 2, MOD);
        for x in a.iter_mut() {
            *x = *x * n_inv % MOD;
        }
    }
}

/// 畳み込み (多項式乗算)
///
/// a(x) * b(x) mod 998244353 を計算
///
/// # Example
/// ```
/// use typical90::convolution::convolution;
///
/// // (1 + 2x) * (3 + 4x) = 3 + 10x + 8x^2
/// let a = vec![1, 2];
/// let b = vec![3, 4];
/// let c = convolution(&a, &b);
/// assert_eq!(c, vec![3, 10, 8]);
/// ```
pub fn convolution(a: &[i64], b: &[i64]) -> Vec<i64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }

    let result_len = a.len() + b.len() - 1;
    let mut n = 1;
    while n < result_len {
        n *= 2;
    }

    let mut fa: Vec<i64> = a.iter().map(|&x| x % MOD).collect();
    let mut fb: Vec<i64> = b.iter().map(|&x| x % MOD).collect();
    fa.resize(n, 0);
    fb.resize(n, 0);

    ntt(&mut fa, false);
    ntt(&mut fb, false);

    for i in 0..n {
        fa[i] = fa[i] * fb[i] % MOD;
    }

    ntt(&mut fa, true);
    fa.truncate(result_len);
    fa
}

/// 任意mod での畳み込み (Garner's algorithm)
///
/// mod m での多項式乗算を計算（m は任意）
///
/// 3つの素数 (NTT-friendly) を使って CRT で復元
///
/// # Example
/// ```
/// use typical90::convolution::convolution_mod;
///
/// // (1 + 2x) * (3 + 4x) = 3 + 10x + 8x^2
/// let a = vec![1, 2];
/// let b = vec![3, 4];
/// let c = convolution_mod(&a, &b, 1_000_000_007);
/// assert_eq!(c, vec![3, 10, 8]);
/// ```
pub fn convolution_mod(a: &[i64], b: &[i64], m: i64) -> Vec<i64> {
    // 3つのNTT-friendly素数
    const MOD1: i64 = 167_772_161; // 2^25 * 5 + 1
    const MOD2: i64 = 469_762_049; // 2^26 * 7 + 1
    const MOD3: i64 = 998_244_353; // 2^23 * 119 + 1

    fn conv_single(a: &[i64], b: &[i64], p: i64, g: i64) -> Vec<i64> {
        if a.is_empty() || b.is_empty() {
            return vec![];
        }
        let result_len = a.len() + b.len() - 1;
        let mut n = 1;
        while n < result_len {
            n *= 2;
        }

        let mut fa: Vec<i64> = a.iter().map(|&x| x.rem_euclid(p)).collect();
        let mut fb: Vec<i64> = b.iter().map(|&x| x.rem_euclid(p)).collect();
        fa.resize(n, 0);
        fb.resize(n, 0);

        ntt_mod(&mut fa, false, p, g);
        ntt_mod(&mut fb, false, p, g);

        for i in 0..n {
            fa[i] = fa[i] * fb[i] % p;
        }

        ntt_mod(&mut fa, true, p, g);
        fa.truncate(result_len);
        fa
    }

    fn ntt_mod(a: &mut [i64], inverse: bool, p: i64, g: i64) {
        let n = a.len();
        if n == 1 {
            return;
        }

        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                a.swap(i, j);
            }
        }

        let mut len = 2;
        while len <= n {
            let ang = if inverse {
                (p - 1) / len as i64
            } else {
                (p - 1) - (p - 1) / len as i64
            };
            let wlen = mod_pow(g, ang, p);

            let mut i = 0;
            while i < n {
                let mut w = 1i64;
                for k in 0..len / 2 {
                    let u = a[i + k];
                    let v = a[i + k + len / 2] * w % p;
                    a[i + k] = (u + v) % p;
                    a[i + k + len / 2] = (u - v + p) % p;
                    w = w * wlen % p;
                }
                i += len;
            }
            len *= 2;
        }

        if inverse {
            let n_inv = mod_pow(n as i64, p - 2, p);
            for x in a.iter_mut() {
                *x = *x * n_inv % p;
            }
        }
    }

    if a.is_empty() || b.is_empty() {
        return vec![];
    }

    let c1 = conv_single(a, b, MOD1, 3);
    let c2 = conv_single(a, b, MOD2, 3);
    let c3 = conv_single(a, b, MOD3, 3);

    // CRT で復元
    let m1_inv_m2 = mod_pow(MOD1, MOD2 - 2, MOD2);
    let m12 = MOD1 as i128 * MOD2 as i128;
    let m12_inv_m3 = mod_pow((m12 % MOD3 as i128) as i64, MOD3 - 2, MOD3);

    let n = c1.len();
    let mut result = vec![0i64; n];

    for i in 0..n {
        let v1 = c1[i];
        let v2 = ((c2[i] - v1).rem_euclid(MOD2) * m1_inv_m2).rem_euclid(MOD2);
        let v12 = v1 as i128 + v2 as i128 * MOD1 as i128;
        let v3 =
            (((c3[i] as i128 - v12).rem_euclid(MOD3 as i128)) as i64 * m12_inv_m3).rem_euclid(MOD3);
        let x = (v12 + v3 as i128 * m12).rem_euclid(m as i128);
        result[i] = x as i64;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convolution() {
        // (1 + 2x) * (3 + 4x) = 3 + 10x + 8x^2
        let a = vec![1, 2];
        let b = vec![3, 4];
        let c = convolution(&a, &b);
        assert_eq!(c, vec![3, 10, 8]);
    }

    #[test]
    fn test_convolution_large() {
        // (1 + x + x^2) * (1 + x + x^2) = 1 + 2x + 3x^2 + 2x^3 + x^4
        let a = vec![1, 1, 1];
        let c = convolution(&a, &a);
        assert_eq!(c, vec![1, 2, 3, 2, 1]);
    }

    #[test]
    fn test_convolution_empty() {
        let a: Vec<i64> = vec![];
        let b = vec![1, 2, 3];
        assert_eq!(convolution(&a, &b), Vec::<i64>::new());
    }

    #[test]
    fn test_convolution_mod() {
        let a = vec![1, 2];
        let b = vec![3, 4];
        let c = convolution_mod(&a, &b, 1_000_000_007);
        assert_eq!(c, vec![3, 10, 8]);
    }

    #[test]
    fn test_convolution_mod_large_values() {
        let a = vec![1_000_000, 2_000_000];
        let b = vec![3_000_000, 4_000_000];
        let m = 1_000_000_007i64;
        let c = convolution_mod(&a, &b, m);
        // 3*10^12, 10*10^12, 8*10^12
        assert_eq!(c[0], (3_000_000_000_000i64 % m));
        assert_eq!(c[1], (10_000_000_000_000i64 % m));
        assert_eq!(c[2], (8_000_000_000_000i64 % m));
    }
}
