// 064 - Uplift (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_bl
//
// 差分配列 D[i] = A[i+1] - A[i] を管理
// 不便さ = Σ|D[i]|
//
// 区間[L, R]を V 変化させると：
// - D[L-1] += V (L > 1)
// - D[R] -= V (R < N)

use proconio::input;

fn main() {
    input! {
        n: usize,
        q: usize,
        a: [i64; n],
        queries: [(usize, usize, i64); q],
    }

    // 差分配列を構築
    let mut d: Vec<i64> = (0..n - 1).map(|i| a[i + 1] - a[i]).collect();

    // 不便さ = Σ|D[i]|
    let mut inconvenience: i64 = d.iter().map(|&x| x.abs()).sum();

    for (l, r, v) in queries {
        // D[L-1] += V (1-indexed の L なので 0-indexed では L-2)
        if l > 1 {
            let idx = l - 2;
            inconvenience -= d[idx].abs();
            d[idx] += v;
            inconvenience += d[idx].abs();
        }

        // D[R] -= V (1-indexed の R なので 0-indexed では R-1)
        if r < n {
            let idx = r - 1;
            inconvenience -= d[idx].abs();
            d[idx] -= v;
            inconvenience += d[idx].abs();
        }

        println!("{}", inconvenience);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_example1() {
        let a = vec![1i64, 2, 3];
        let mut d: Vec<i64> = (0..2).map(|i| a[i + 1] - a[i]).collect();
        // d = [1, 1], 不便さ = 2

        let mut inconvenience: i64 = d.iter().map(|&x| x.abs()).sum();
        assert_eq!(inconvenience, 2);

        // Query 1: L=2, R=3, V=1
        // D[0] += 1 (L-2=0), D[2] は範囲外
        inconvenience -= d[0].abs();
        d[0] += 1;
        inconvenience += d[0].abs();
        // d = [2, 1], 不便さ = 3
        assert_eq!(inconvenience, 3);

        // Query 2: L=1, R=2, V=-1
        // D[-1] は範囲外, D[1] -= -1 = D[1] += 1
        inconvenience -= d[1].abs();
        d[1] -= -1;
        inconvenience += d[1].abs();
        // d = [2, 2], 不便さ = 4
        assert_eq!(inconvenience, 4);

        // Query 3: L=1, R=3, V=2
        // 両端なので変化なし
        // d = [2, 2], 不便さ = 4
        assert_eq!(inconvenience, 4);
    }
}
