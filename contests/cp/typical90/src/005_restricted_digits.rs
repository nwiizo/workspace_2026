// 005 - Restricted Digits (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_e
//
// 問題: N桁の正整数で、各桁が数字集合Cのみで構成され、
// Bで割り切れるものの個数を求めよ（mod 10^9+7）。
//
// 解法: 行列累乗
// - dp[i][r] = i桁目までで、mod B の余りがrであるものの個数
// - 遷移: dp[i+1][(r*10 + d) % B] += dp[i][r]  (d ∈ C)
// - これは行列の形で書ける → N桁分の遷移は行列のN乗
// - 計算量: O(B^3 log N)

use proconio::input;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        n: i64,
        b: usize,
        k: usize,
        c: [usize; k],
    }
    println!("{}", solve(n, b, &c));
}

#[allow(clippy::needless_range_loop)]
fn solve(n: i64, b: usize, c: &[usize]) -> i64 {
    // B×B の遷移行列を構築
    // matrix[r_next][r] = (余り r から r_next への遷移回数)
    let mut matrix = vec![vec![0i64; b]; b];
    for r in 0..b {
        for &d in c {
            let r_next = (r * 10 + d) % b;
            matrix[r_next][r] = (matrix[r_next][r] + 1) % MOD;
        }
    }

    // 初期状態: 0桁で余り0が1通り
    // ただし、N桁の正整数なので先頭は0以外
    // → 1桁目の遷移は別で計算

    // 実際には、最初の1桁分は0以外の数字で始める必要がある
    // 初期ベクトル: 0桁目で余り0が1通り
    // N乗した後、余り0の個数を見る

    // ただし、問題では「N桁の正整数」なので先頭0は不可
    // このコードでは c に0が含まれていても先頭0を許している
    // 厳密には c から0を除いた1桁目と、その後の遷移を分けるべき

    // 簡易版: 先頭0も許す場合
    // 初期ベクトル v[0] = 1, v[i] = 0 (i > 0)
    // N乗後の v[0] が答え

    // 正しい実装: 先頭1桁は0以外のみ
    let c_nonzero: Vec<usize> = c.iter().copied().filter(|&d| d != 0).collect();

    // 1桁目の状態を計算（先頭は0以外）
    let mut init = vec![0i64; b];
    for &d in &c_nonzero {
        let r = d % b;
        init[r] = (init[r] + 1) % MOD;
    }

    if n == 1 {
        // 1桁の場合は init[0] が答え
        return init[0];
    }

    // 残りの N-1 桁分の遷移を行列累乗で計算
    let trans = matrix_pow(&matrix, (n - 1) as u64, b);

    // 行列とベクトルの積
    let mut result = vec![0i64; b];
    for i in 0..b {
        for j in 0..b {
            result[i] = (result[i] + trans[i][j] * init[j]) % MOD;
        }
    }

    result[0]
}

// 行列の積
fn matrix_mul(a: &[Vec<i64>], b_mat: &[Vec<i64>], size: usize) -> Vec<Vec<i64>> {
    let mut c = vec![vec![0i64; size]; size];
    for i in 0..size {
        for k in 0..size {
            if a[i][k] == 0 {
                continue;
            }
            for j in 0..size {
                c[i][j] = (c[i][j] + a[i][k] * b_mat[k][j]) % MOD;
            }
        }
    }
    c
}

// 行列の累乗
#[allow(clippy::needless_range_loop)]
fn matrix_pow(mat: &[Vec<i64>], mut exp: u64, size: usize) -> Vec<Vec<i64>> {
    // 単位行列
    let mut result = vec![vec![0i64; size]; size];
    for i in 0..size {
        result[i][i] = 1;
    }

    let mut base = mat.to_vec();

    while exp > 0 {
        if exp & 1 == 1 {
            result = matrix_mul(&result, &base, size);
        }
        base = matrix_mul(&base, &base, size);
        exp >>= 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // N=2, B=7, C={1,4,7}
        // 2桁で7で割り切れる: 14, 77
        assert_eq!(solve(2, 7, &[1, 4, 7]), 2);
    }

    #[test]
    fn test_example2() {
        // N=1, B=2, C={1,2,3}
        // 1桁で2で割り切れる: 2
        assert_eq!(solve(1, 2, &[1, 2, 3]), 1);
    }
}
