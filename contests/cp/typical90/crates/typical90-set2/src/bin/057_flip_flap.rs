// 057 - Flip Flap (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_be
//
// ============================================================
// GF(2) 上のガウス消去法
// ============================================================
//
// 各スイッチを M ビットのベクトルとして表現
// 目標状態 S を達成するスイッチの組み合わせを数える
//
// これは GF(2) 上の連立方程式 Ax = S
// - 解が存在しない → 0
// - 解が存在する → 2^(自由変数の数)
//
// 計算量: O(NM × min(N, M))
//
// ============================================================

use proconio::input;

const MOD: u64 = 998244353;

fn main() {
    input! {
        n: usize,
        m: usize,
    }

    // 各スイッチが反転させるパネルを読み取り
    let mut switches: Vec<Vec<u64>> = Vec::with_capacity(n);
    for _ in 0..n {
        input! {
            t: usize,
            panels: [usize; t],
        }
        // ビットベクトルに変換
        let mut vec = vec![0u64; (m + 63) / 64];
        for p in panels {
            let idx = (p - 1) / 64;
            let bit = (p - 1) % 64;
            vec[idx] |= 1u64 << bit;
        }
        switches.push(vec);
    }

    input! {
        target: [u64; m],
    }

    // 目標状態をビットベクトルに変換
    let mut target_vec = vec![0u64; (m + 63) / 64];
    for (i, &s) in target.iter().enumerate() {
        if s == 1 {
            let idx = i / 64;
            let bit = i % 64;
            target_vec[idx] |= 1u64 << bit;
        }
    }

    println!("{}", solve(n, m, &switches, &target_vec));
}

fn solve(n: usize, m: usize, switches: &[Vec<u64>], target: &[u64]) -> u64 {
    let _words = (m + 63) / 64;

    // 拡大係数行列を構築 [スイッチベクトル | 目標ベクトル]
    // 行: M個のパネル方程式、列: N個のスイッチ + 1個の目標
    // 転置して考える: 行=スイッチ, 列=パネル+1(目標)

    // 行列: N行 × (M+1)列 を扱う
    // 各行: スイッチiのビットベクトル + 目標との対応

    // 実際には、列ごとに処理する方が効率的
    // パネルjについて、どのスイッチが影響するかを行として持つ

    // 簡単のため、M×N 行列を使って処理
    // mat[j] = パネルjに対応する行 (N+1ビット: スイッチN個 + 目標1ビット)

    let cols = n + 1;
    let col_words = (cols + 63) / 64;
    let mut mat: Vec<Vec<u64>> = vec![vec![0u64; col_words]; m];

    // 各パネルについて、どのスイッチが影響するか設定
    for (switch_idx, switch) in switches.iter().enumerate() {
        for panel in 0..m {
            let sw_idx = panel / 64;
            let sw_bit = panel % 64;
            if sw_idx < switch.len() && (switch[sw_idx] >> sw_bit) & 1 == 1 {
                let col_idx = switch_idx / 64;
                let col_bit = switch_idx % 64;
                mat[panel][col_idx] |= 1u64 << col_bit;
            }
        }
    }

    // 目標ベクトルを最後の列に設定
    for panel in 0..m {
        let t_idx = panel / 64;
        let t_bit = panel % 64;
        if t_idx < target.len() && (target[t_idx] >> t_bit) & 1 == 1 {
            let col_idx = n / 64;
            let col_bit = n % 64;
            mat[panel][col_idx] |= 1u64 << col_bit;
        }
    }

    // ガウス消去法
    let mut rank = 0;
    for col in 0..n {
        // ピボット探索
        let col_idx = col / 64;
        let col_bit = col % 64;

        let mut pivot = None;
        for row in rank..m {
            if (mat[row][col_idx] >> col_bit) & 1 == 1 {
                pivot = Some(row);
                break;
            }
        }

        let pivot = match pivot {
            Some(p) => p,
            None => continue,
        };

        // 行交換
        mat.swap(rank, pivot);

        // 消去
        let pivot_row = mat[rank].clone();
        for row in 0..m {
            if row != rank && (mat[row][col_idx] >> col_bit) & 1 == 1 {
                for w in 0..col_words {
                    mat[row][w] ^= pivot_row[w];
                }
            }
        }

        rank += 1;
    }

    // 解の存在チェック
    // ピボットがない行で目標ビットが1なら解なし
    let target_col_idx = n / 64;
    let target_col_bit = n % 64;

    for row in rank..m {
        // この行はすべてのスイッチ列が0
        // 目標列が1なら矛盾
        if (mat[row][target_col_idx] >> target_col_bit) & 1 == 1 {
            return 0;
        }
    }

    // 自由変数の数 = N - rank
    let free = n - rank;

    // 答え = 2^free mod MOD
    mod_pow(2, free as u64, MOD)
}

fn mod_pow(mut base: u64, mut exp: u64, m: u64) -> u64 {
    let mut result = 1u64;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn solve_simple(n: usize, m: usize, switches_raw: &[Vec<usize>], target: &[u64]) -> u64 {
        let words = (m + 63) / 64;
        let mut switches = Vec::with_capacity(n);
        for sw in switches_raw {
            let mut vec = vec![0u64; words];
            for &p in sw {
                let idx = p / 64;
                let bit = p % 64;
                vec[idx] |= 1u64 << bit;
            }
            switches.push(vec);
        }
        let mut target_vec = vec![0u64; words];
        for (i, &s) in target.iter().enumerate() {
            if s == 1 {
                let idx = i / 64;
                let bit = i % 64;
                target_vec[idx] |= 1u64 << bit;
            }
        }
        solve(n, m, &switches, &target_vec)
    }

    #[test]
    fn test_example1() {
        // スイッチ1: パネル1,2 反転 → [0, 1] (0-indexed)
        // スイッチ2: パネル2,3 反転 → [1, 2] (0-indexed)
        // 目標: [1, 0, 1]
        let switches = vec![vec![0, 1], vec![1, 2]];
        let target = vec![1, 0, 1];
        assert_eq!(solve_simple(2, 3, &switches, &target), 1);
    }

    #[test]
    fn test_no_solution() {
        // 解がない場合
        // スイッチ1: [0]
        // 目標: [0, 1] → パネル1を表にできない
        let switches = vec![vec![0]];
        let target = vec![0, 1];
        assert_eq!(solve_simple(1, 2, &switches, &target), 0);
    }
}
