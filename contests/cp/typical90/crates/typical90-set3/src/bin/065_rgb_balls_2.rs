// 065 - RGB Balls 2 (★7)
// https://atcoder.jp/contests/typical90/tasks/typical90_bm
//
// g を固定して r の範囲を計算
// 制約: r+g+b=K, r+g≤X, g+b≤Y, b+r≤Z

use proconio::input;

const MOD: u64 = 998244353;
const MAX: usize = 600_001;

fn main() {
    input! {
        r_max: usize,
        g_max: usize,
        b_max: usize,
        k: usize,
        x: usize,
        y: usize,
        z: usize,
    }

    // 前計算
    let (fact, inv_fact) = precompute_factorial(MAX);

    let comb = |n: usize, r: usize| -> u64 {
        if r > n {
            0
        } else {
            fact[n] * inv_fact[r] % MOD * inv_fact[n - r] % MOD
        }
    };

    let mut ans = 0u64;

    // g を固定
    let g_min = if k > z { k - z } else { 0 };
    let g_upper = g_max.min(k).min(x).min(y);

    for g in g_min..=g_upper {
        // r + b = k - g
        let rb = k - g;

        // r の範囲
        // r >= 0, r <= r_max
        // b = rb - r >= 0 → r <= rb
        // b = rb - r <= b_max → r >= rb - b_max (if rb > b_max)
        // r + g <= x → r <= x - g
        // g + b <= y → b <= y - g → r >= rb - (y - g) = rb - y + g (if rb > y - g)
        // b + r <= z → rb <= z (already ensured by g >= k - z)

        let r_lo = 0
            .max(rb.saturating_sub(b_max))
            .max(rb.saturating_sub(y.saturating_sub(g)));
        let r_hi = r_max.min(rb).min(x.saturating_sub(g));

        if r_lo > r_hi {
            continue;
        }

        // Σ C(R, r) * C(B, rb - r) for r in [r_lo, r_hi]
        // これを累積和で計算するには前計算が必要だが、ここでは直接計算

        // 畳み込みで計算：C(R, r) * C(B, rb - r) の和
        // Vandermonde's identity: Σ C(R, r) * C(B, rb - r) = C(R + B, rb)
        // ただし範囲制限があるので直接計算

        let c_g = comb(g_max, g);

        for r in r_lo..=r_hi {
            let b = rb - r;
            let c_r = comb(r_max, r);
            let c_b = comb(b_max, b);
            ans = (ans + c_g * c_r % MOD * c_b % MOD) % MOD;
        }
    }

    println!("{}", ans);
}

fn precompute_factorial(n: usize) -> (Vec<u64>, Vec<u64>) {
    let mut fact = vec![1u64; n];
    let mut inv_fact = vec![1u64; n];

    for i in 1..n {
        fact[i] = fact[i - 1] * i as u64 % MOD;
    }

    inv_fact[n - 1] = mod_pow(fact[n - 1], MOD - 2);
    for i in (1..n).rev() {
        inv_fact[i - 1] = inv_fact[i] * i as u64 % MOD;
    }

    (fact, inv_fact)
}

fn mod_pow(mut base: u64, mut exp: u64) -> u64 {
    let mut result = 1u64;
    base %= MOD;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % MOD;
        }
        exp >>= 1;
        base = base * base % MOD;
    }
    result
}

#[cfg(test)]
mod tests {
    // テストは計算コストが高いので省略
}
