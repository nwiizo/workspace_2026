// 079 - Two by Two (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_ca
//
// 左上から順に、各 2x2 ブロックで A[i][j] を B[i][j] に一致させていく
// 右下の要素 A[H-1][W-1] だけは直接操作できないので、最後に一致しているか確認

use proconio::input;

fn main() {
    input! {
        h: usize,
        w: usize,
        a: [[i64; w]; h],
        b: [[i64; w]; h],
    }

    match solve(h, w, &a, &b) {
        Some(ops) => {
            println!("Yes");
            println!("{}", ops);
        }
        None => println!("No"),
    }
}

fn solve(h: usize, w: usize, a: &[Vec<i64>], b: &[Vec<i64>]) -> Option<i64> {
    // 差分配列を作成
    let mut diff: Vec<Vec<i64>> = (0..h)
        .map(|i| (0..w).map(|j| b[i][j] - a[i][j]).collect())
        .collect();

    let mut total_ops = 0i64;

    // 左上から順に処理
    for i in 0..h - 1 {
        for j in 0..w - 1 {
            // diff[i][j] を 0 にするために操作
            let d = diff[i][j];
            // 2x2 ブロック全体を d だけ変化させる
            diff[i][j] -= d;
            diff[i + 1][j] -= d;
            diff[i][j + 1] -= d;
            diff[i + 1][j + 1] -= d;

            total_ops += d.abs();
        }
    }

    // 全ての diff が 0 になっているか確認
    for i in 0..h {
        for j in 0..w {
            if diff[i][j] != 0 {
                return None;
            }
        }
    }

    Some(total_ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, 0]];
        let b = vec![vec![1, 1, 0], vec![1, 1, 0], vec![0, 0, 0]];
        assert_eq!(solve(3, 3, &a, &b), Some(1));
    }

    #[test]
    fn test_example2() {
        let a = vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, 0]];
        let b = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
        assert_eq!(solve(3, 3, &a, &b), None);
    }
}
