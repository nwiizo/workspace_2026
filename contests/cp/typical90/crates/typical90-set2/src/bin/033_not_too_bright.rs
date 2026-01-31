// 033 - Not Too Bright (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_ag
//
// 問題: H×Wのグリッドに電球を置く。2×2の範囲に2個以上置けない。
//       最大何個置けるか。
//
// 解法: 市松模様に配置
//       H=1 または W=1 の場合は全マスに置ける
//       それ以外は ceil(H/2) × ceil(W/2)

use proconio::input;

fn main() {
    input! {
        h: usize,
        w: usize,
    }
    println!("{}", solve(h, w));
}

fn solve(h: usize, w: usize) -> usize {
    if h == 1 || w == 1 {
        h * w
    } else {
        // 1マスおきに配置
        h.div_ceil(2) * w.div_ceil(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // 2×2 → 1個
        assert_eq!(solve(2, 2), 1);
    }

    #[test]
    fn example2() {
        // 3×3 → ceil(3/2) × ceil(3/2) = 2 × 2 = 4
        assert_eq!(solve(3, 3), 4);
    }

    #[test]
    fn single_row() {
        // 1×5 → 5個
        assert_eq!(solve(1, 5), 5);
    }

    #[test]
    fn single_column() {
        // 5×1 → 5個
        assert_eq!(solve(5, 1), 5);
    }

    #[test]
    fn large() {
        // 100×100 → 50×50 = 2500
        assert_eq!(solve(100, 100), 2500);
    }
}
