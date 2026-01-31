// 046 - I Love 46 (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_at
//
// A_i + B_j + C_k ≡ 0 (mod 46) となる (i,j,k) の個数
//
// 各配列で 46 で割った余りごとにカウント
// 答え = Σ cnt_a[a] * cnt_b[b] * cnt_c[c] where (a + b + c) % 46 == 0
//
// 計算量: O(N + 46^2)

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; n],
        b: [i64; n],
        c: [i64; n],
    }
    println!("{}", solve(&a, &b, &c));
}

fn solve(a: &[i64], b: &[i64], c: &[i64]) -> i64 {
    let mut cnt_a = [0i64; 46];
    let mut cnt_b = [0i64; 46];
    let mut cnt_c = [0i64; 46];

    for &x in a {
        cnt_a[(x % 46) as usize] += 1;
    }
    for &x in b {
        cnt_b[(x % 46) as usize] += 1;
    }
    for &x in c {
        cnt_c[(x % 46) as usize] += 1;
    }

    let mut ans = 0i64;
    for ra in 0..46 {
        for rb in 0..46 {
            let rc = (46 * 2 - ra - rb) % 46;
            ans += cnt_a[ra] * cnt_b[rb] * cnt_c[rc];
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![10, 13, 93];
        let b = vec![5, 27, 35];
        let c = vec![55, 28, 52];
        assert_eq!(solve(&a, &b, &c), 3);
    }

    #[test]
    fn test_example2() {
        let a = vec![10, 56, 102];
        let b = vec![16, 62, 108];
        let c = vec![20, 66, 112];
        assert_eq!(solve(&a, &b, &c), 27);
    }
}
