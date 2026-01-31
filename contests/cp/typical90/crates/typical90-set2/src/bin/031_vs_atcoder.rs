// 031 - VS AtCoder (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_ae
//
// ゲーム理論（Sprague-Grundy定理）
// - 各山のGrundy数を計算し、XORを取る
// - XOR = 0 なら後攻勝ち、そうでなければ先攻勝ち
//
// 遷移:
// - (W, B) → (W-1, B+W)  (W >= 1)
// - (W, B) → (W, B-k)    (1 <= k <= B/2, B >= 2)
//
// 敗北状態: (W=0, B<=1) で手番 → Grundy数 = 0

use proconio::input;
use std::collections::HashMap;

fn main() {
    input! {
        n: usize,
        w: [usize; n],
        b: [usize; n],
    }
    println!("{}", solve(n, &w, &b));
}

fn solve(n: usize, w: &[usize], b: &[usize]) -> &'static str {
    let mut memo: HashMap<(usize, usize), usize> = HashMap::new();

    let mut xor_sum = 0;
    for i in 0..n {
        xor_sum ^= grundy(w[i], b[i], &mut memo);
    }

    if xor_sum != 0 { "First" } else { "Second" }
}

fn grundy(w: usize, b: usize, memo: &mut HashMap<(usize, usize), usize>) -> usize {
    // 敗北状態: 操作不能
    if w == 0 && b <= 1 {
        return 0;
    }

    if let Some(&g) = memo.get(&(w, b)) {
        return g;
    }

    let mut reachable = Vec::new();

    // 操作1: 白石を1個減らし、青石をw個増やす
    if w >= 1 {
        reachable.push(grundy(w - 1, b + w, memo));
    }

    // 操作2: 青石をk個減らす (1 <= k <= b/2)
    if b >= 2 {
        for k in 1..=b / 2 {
            reachable.push(grundy(w, b - k, memo));
        }
    }

    // mex (minimum excludant) を計算
    let g = mex(&reachable);
    memo.insert((w, b), g);
    g
}

fn mex(set: &[usize]) -> usize {
    let mut exists = vec![false; set.len() + 1];
    for &x in set {
        if x < exists.len() {
            exists[x] = true;
        }
    }
    for i in 0.. {
        if i >= exists.len() || !exists[i] {
            return i;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // N=1, W=[0], B=[2]
        assert_eq!(solve(1, &[0], &[2]), "First");
    }

    #[test]
    fn test_example2() {
        // N=2, W=[10,10], B=[10,10]
        assert_eq!(solve(2, &[10, 10], &[10, 10]), "Second");
    }

    #[test]
    fn test_example3() {
        // N=1, W=[1], B=[1]
        assert_eq!(solve(1, &[1], &[1]), "Second");
    }
}
