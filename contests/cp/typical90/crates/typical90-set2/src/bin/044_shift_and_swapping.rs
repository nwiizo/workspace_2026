// 044 - Shift and Swapping (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_ar
//
// 配列を実際にシフトせず、オフセットで仮想インデックスを管理
//
// 右シフト: 最後の要素が最初に移動
// offset を使って、仮想インデックス → 実インデックスに変換
//
// 右シフト1回で offset = (offset - 1 + N) % N
// 実インデックス = (仮想インデックス + offset) % N

use proconio::input;

fn main() {
    input! {
        n: usize,
        q: usize,
        mut a: [i64; n],
        queries: [(usize, usize, usize); q],
    }
    solve(n, &mut a, &queries);
}

fn solve(n: usize, a: &mut [i64], queries: &[(usize, usize, usize)]) {
    let mut offset = 0usize;

    for &(t, x, y) in queries {
        match t {
            1 => {
                // x項とy項を交換 (1-indexed)
                let real_x = (x - 1 + offset) % n;
                let real_y = (y - 1 + offset) % n;
                a.swap(real_x, real_y);
            }
            2 => {
                // 右シフト
                offset = (offset + n - 1) % n;
            }
            3 => {
                // x項を出力 (1-indexed)
                let real_x = (x - 1 + offset) % n;
                println!("{}", a[real_x]);
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_example1() {
        // 初期: [6, 17, 2, 4, 17, 19, 1, 7]
        // 右シフト後 offset=7 として手動でシミュレート
        let mut a = vec![6, 17, 2, 4, 17, 19, 1, 7];
        let n = 8;
        let mut offset = 0usize;

        // Query 1: 右シフト
        offset = (offset + n - 1) % n;

        // Query 2: swap(7, 2)
        let real_x = (7 - 1 + offset) % n;
        let real_y = (2 - 1 + offset) % n;
        a.swap(real_x, real_y);

        // Query 3: swap(2, 6)
        let real_x = (2 - 1 + offset) % n;
        let real_y = (6 - 1 + offset) % n;
        a.swap(real_x, real_y);

        // Query 4: swap(4, 5)
        let real_x = (4 - 1 + offset) % n;
        let real_y = (5 - 1 + offset) % n;
        a.swap(real_x, real_y);

        // Query 5: get(4)
        let real_x = (4 - 1 + offset) % n;
        assert_eq!(a[real_x], 4);
    }
}
