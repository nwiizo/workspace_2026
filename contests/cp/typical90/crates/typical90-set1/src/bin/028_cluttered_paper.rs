// 028 - Cluttered Paper (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_ab
//
// 問題: N枚の長方形の紙が置かれる。1枚以上の紙で覆われる面積を求めよ。
//
// 解法: 2次元いもす法
//       座標圧縮して累積和

use proconio::input;

fn main() {
    input! {
        n: usize,
        rects: [(i32, i32, i32, i32); n], // (x1, y1, x2, y2)
    }
    println!("{}", solve(&rects));
}

#[allow(clippy::needless_range_loop)]
fn solve(rects: &[(i32, i32, i32, i32)]) -> i64 {
    // 座標を収集
    let mut xs: Vec<i32> = rects.iter().flat_map(|&(x1, _, x2, _)| [x1, x2]).collect();
    let mut ys: Vec<i32> = rects.iter().flat_map(|&(_, y1, _, y2)| [y1, y2]).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    let x_to_idx = |x: i32| xs.binary_search(&x).unwrap();
    let y_to_idx = |y: i32| ys.binary_search(&y).unwrap();

    let w = xs.len();
    let h = ys.len();

    // いもす法用の2次元配列
    let mut diff = vec![vec![0i32; h + 1]; w + 1];

    for &(x1, y1, x2, y2) in rects {
        let (ix1, ix2) = (x_to_idx(x1), x_to_idx(x2));
        let (iy1, iy2) = (y_to_idx(y1), y_to_idx(y2));

        diff[ix1][iy1] += 1;
        diff[ix1][iy2] -= 1;
        diff[ix2][iy1] -= 1;
        diff[ix2][iy2] += 1;
    }

    // 累積和を取る
    for i in 0..w {
        for j in 0..h {
            if i > 0 {
                diff[i][j] += diff[i - 1][j];
            }
        }
    }
    for i in 0..w {
        for j in 0..h {
            if j > 0 {
                diff[i][j] += diff[i][j - 1];
            }
        }
    }

    // 面積を計算
    let mut total = 0i64;
    for i in 0..w - 1 {
        for j in 0..h - 1 {
            if diff[i][j] > 0 {
                let area = (xs[i + 1] - xs[i]) as i64 * (ys[j + 1] - ys[j]) as i64;
                total += area;
            }
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_rect() {
        // (0,0)-(2,3) → 面積6
        let rects = vec![(0, 0, 2, 3)];
        assert_eq!(solve(&rects), 6);
    }

    #[test]
    fn overlapping_rects() {
        // (0,0)-(2,2) と (1,1)-(3,3)
        // 重複部分は1回だけカウント
        // 面積: 4 + 4 - 1 = 7
        let rects = vec![(0, 0, 2, 2), (1, 1, 3, 3)];
        assert_eq!(solve(&rects), 7);
    }

    #[test]
    fn no_overlap() {
        // (0,0)-(1,1) と (2,2)-(3,3)
        // 面積: 1 + 1 = 2
        let rects = vec![(0, 0, 1, 1), (2, 2, 3, 3)];
        assert_eq!(solve(&rects), 2);
    }
}
