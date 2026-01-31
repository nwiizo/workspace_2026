// 089 - Partitions and Inversions (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_ck
//
// 区間 [l, r] の転倒数が K 以下となる分割の数を数える
// dp[i] = 位置 i までの分割の数
// 遷移: dp[i] = Σ dp[j] for j where [j+1, i] の転倒数 ≤ K
//
// 転倒数の計算: BIT で効率化
// 尺取り法で、転倒数 ≤ K となる区間の左端を管理

use proconio::input;

const MOD: u64 = 1_000_000_007;

fn main() {
    input! {
        n: usize,
        k: u64,
        a: [i64; n],
    }
    println!("{}", solve(n, k, &a));
}

fn solve(n: usize, k: u64, a: &[i64]) -> u64 {
    // 座標圧縮
    let mut sorted = a.to_vec();
    sorted.sort();
    sorted.dedup();

    let compress = |x: i64| -> usize { sorted.binary_search(&x).unwrap() };

    let m = sorted.len();

    // dp[i] = 位置 i で区切った時の分割数
    // dp[0] = 1 (空の状態)
    let mut dp = vec![0u64; n + 1];
    dp[0] = 1;

    // 累積和で dp の和を管理
    let mut dp_sum = vec![0u64; n + 2];
    dp_sum[1] = 1; // dp[0] = 1

    // 尺取り法
    // 現在の区間 [left, right] の転倒数を管理
    let mut left = 0;
    let mut inversions = 0u64;

    // BIT で [left, right] 内の要素を管理
    let mut bit = vec![0i64; m + 1]; // 追加: +1, 削除: -1

    fn bit_add(bit: &mut [i64], mut i: usize, delta: i64) {
        i += 1;
        while i < bit.len() {
            bit[i] += delta;
            i += i & i.wrapping_neg();
        }
    }

    fn bit_sum(bit: &[i64], mut i: usize) -> i64 {
        i += 1;
        let mut sum = 0i64;
        while i > 0 {
            sum += bit[i];
            i -= i & i.wrapping_neg();
        }
        sum
    }

    for right in 0..n {
        let x = compress(a[right]);

        // a[right] を追加したときの転倒数の増加
        // x より大きい要素の数が増加分
        let larger = bit_sum(&bit, m - 1) - bit_sum(&bit, x);
        inversions += larger as u64;
        bit_add(&mut bit, x, 1);

        // 転倒数が K を超えたら left を進める
        while inversions > k && left < right {
            let y = compress(a[left]);
            // a[left] を削除したときの転倒数の減少
            // y より大きい要素の数が減少分（y の右側にある要素との転倒がなくなる）
            // ただし、y 自身が作っていた転倒は y より小さい要素との組
            bit_add(&mut bit, y, -1);
            let smaller = bit_sum(&bit, y.saturating_sub(1));
            inversions -= smaller as u64;
            left += 1;
        }

        // dp[right + 1] = Σ dp[left..right+1]
        // = dp_sum[right + 1] - dp_sum[left]
        let range_sum = (dp_sum[right + 1] + MOD - dp_sum[left]) % MOD;
        dp[right + 1] = range_sum;
        dp_sum[right + 2] = (dp_sum[right + 1] + dp[right + 1]) % MOD;
    }

    dp[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![3, 1, 4, 2];
        assert_eq!(solve(4, 0, &a), 2);
    }

    #[test]
    fn test_example2() {
        let a = vec![5, 3, 7, 2, 1, 2, 3];
        assert_eq!(solve(7, 2, &a), 44);
    }

    #[test]
    fn test_example3() {
        let a = vec![7, 6, 5, 4, 3, 2, 1];
        assert_eq!(solve(7, 0, &a), 1);
    }
}
