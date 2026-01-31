// 080 - Let's Share Bit (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_cb
//
// 包除原理
// 「全ての A_i と AND が非ゼロ」= 「少なくとも1つ共通ビットがある」
// 補集合: 「少なくとも1つの A_i と AND がゼロ」
//
// f(S) = A_i (i ∈ S) の OR を取ったときの補集合のビット数
// = D - popcount(OR of A_i for i in S)
//
// 2^f(S) が「S に含まれる全ての i について A_i & x = 0 となる x の個数」
//
// 包除原理で計算

use proconio::input;

fn main() {
    input! {
        n: usize,
        d: usize,
        a: [u64; n],
    }
    println!("{}", solve(n, d, &a));
}

fn solve(n: usize, d: usize, a: &[u64]) -> u64 {
    // 全体の個数 = 2^D
    let total = 1u64 << d;

    // 包除原理
    // 「少なくとも1つの A_i と AND が 0」を数えて全体から引く
    // Σ (-1)^(|S|+1) * 2^(D - popcount(OR of A_i for i in S))

    let mut bad = 0i64; // 少なくとも1つと AND が 0 になる個数

    for mask in 1..(1u64 << n) {
        // S に含まれる A_i の OR を計算
        let mut or_val = 0u64;
        let mut cnt = 0;
        for i in 0..n {
            if (mask >> i) & 1 == 1 {
                or_val |= a[i];
                cnt += 1;
            }
        }

        // D - popcount(or_val) 個のビットが自由
        let free_bits = d as u32 - or_val.count_ones();
        let count = 1i64 << free_bits;

        // 包除: |S| が奇数なら +、偶数なら -
        if cnt % 2 == 1 {
            bad += count;
        } else {
            bad -= count;
        }
    }

    // 全体から bad を引く
    (total as i64 - bad) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![1, 3, 4, 5];
        assert_eq!(solve(4, 3, &a), 2);
    }

    #[test]
    fn test_example2() {
        let a = vec![1050624, 32772, 493952, 144, 869120];
        assert_eq!(solve(5, 21, &a), 869120);
    }
}
