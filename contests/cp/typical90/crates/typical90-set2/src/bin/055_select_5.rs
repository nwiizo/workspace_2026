// 055 - Select 5 (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_bc
//
// ============================================================
// 5重ループで全探索
// ============================================================
//
// N ≤ 100 なので C(100, 5) ≈ 7.5 × 10^7
// 5重ループで全探索可能
//
// 注意: 積のオーバーフローを避けるため、各ステップで mod P を取る
//
// ============================================================

use proconio::input;

fn main() {
    input! {
        n: usize,
        p: u64,
        q: u64,
        a: [u64; n],
    }
    println!("{}", solve(n, p, q, &a));
}

fn solve(n: usize, p: u64, q: u64, a: &[u64]) -> u64 {
    let mut count = 0u64;

    // 5重ループで全探索
    for i in 0..n {
        for j in (i + 1)..n {
            let prod_ij = (a[i] % p) * (a[j] % p) % p;
            for k in (j + 1)..n {
                let prod_ijk = prod_ij * (a[k] % p) % p;
                for l in (k + 1)..n {
                    let prod_ijkl = prod_ijk * (a[l] % p) % p;
                    for m in (l + 1)..n {
                        let prod_all = prod_ijkl * (a[m] % p) % p;
                        if prod_all == q {
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // 1*2*3*4*5 = 120, 120 mod 7 = 1
        let a = vec![1, 2, 3, 4, 5, 6];
        assert_eq!(solve(6, 7, 1, &a), 1);
    }

    #[test]
    fn test_example2() {
        // 任意の5個を選んでも 0*0*0*0*0 = 0, 0 mod 1 = 0
        // C(10, 5) = 252
        let a = vec![0; 10];
        assert_eq!(solve(10, 1, 0, &a), 252);
    }
}
