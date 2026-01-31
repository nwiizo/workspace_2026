// 086 - Snuke's Favorite Arrays (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_ch
//
// 各ビットは独立に考えられる
// 各ビットについて、N 個の要素が 0 か 1 かの 2^N 通りを試し、
// 全ての制約を満たすかチェック
// 満たす組み合わせの数を数え、全ビットで掛け合わせる

use proconio::input;

const MOD: u64 = 1_000_000_007;

fn main() {
    input! {
        n: usize,
        q: usize,
        constraints: [(usize, usize, usize, u64); q],
    }
    println!("{}", solve(n, q, &constraints));
}

fn solve(n: usize, _q: usize, constraints: &[(usize, usize, usize, u64)]) -> u64 {
    let mut ans = 1u64;

    // 各ビットについて
    for bit in 0..60 {
        // このビットでの制約
        // w_i の bit 番目が 1 なら、x_i, y_i, z_i の少なくとも1つが 1
        // w_i の bit 番目が 0 なら、x_i, y_i, z_i の全てが 0

        let mut cnt = 0u64;

        // 2^N 通りを全探索
        for mask in 0..(1 << n) {
            let mut ok = true;

            for &(x, y, z, w) in constraints {
                let bx = (mask >> (x - 1)) & 1;
                let by = (mask >> (y - 1)) & 1;
                let bz = (mask >> (z - 1)) & 1;
                let bw = (w >> bit) & 1;

                let or_val = bx | by | bz;
                if or_val != bw {
                    ok = false;
                    break;
                }
            }

            if ok {
                cnt += 1;
            }
        }

        ans = ans * cnt % MOD;
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let constraints = vec![(1, 2, 3, 50), (2, 3, 4, 45)];
        assert_eq!(solve(4, 2, &constraints), 13);
    }

    #[test]
    fn test_example2() {
        let constraints = vec![
            (2, 3, 6, 1152886174205865983),
            (1, 2, 8, 1116611213275394047),
        ];
        assert_eq!(solve(8, 2, &constraints), 395781543);
    }
}
