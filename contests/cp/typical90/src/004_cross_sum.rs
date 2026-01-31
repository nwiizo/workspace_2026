// 004 - Cross Sum (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_d
//
// 問題: H×Wのグリッドが与えられる。各マス(i,j)について、
// 同じ行と同じ列の要素の和を求めよ（自分自身は1回だけ数える）。
//
// 解法: 行の和と列の和を前計算
// - row_sum[i] = i行目の全要素の和
// - col_sum[j] = j列目の全要素の和
// - 答え = row_sum[i] + col_sum[j] - a[i][j]
//   (自分自身は行と列で2回数えているので1回引く)

use proconio::input;

fn main() {
    input! {
        h: usize,
        w: usize,
        a: [[i64; w]; h],
    }
    solve(h, w, &a);
}

fn solve(h: usize, w: usize, a: &[Vec<i64>]) {
    // 行の和を前計算
    let row_sum: Vec<i64> = a.iter().map(|row| row.iter().sum()).collect();

    // 列の和を前計算
    let col_sum: Vec<i64> = (0..w).map(|j| (0..h).map(|i| a[i][j]).sum()).collect();

    // 各マスの答えを計算
    let mut result = Vec::with_capacity(h);
    for i in 0..h {
        let row: Vec<String> = (0..w)
            .map(|j| (row_sum[i] + col_sum[j] - a[i][j]).to_string())
            .collect();
        result.push(row.join(" "));
    }

    for line in result {
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_example1() {
        // 3x3 グリッド
        // 1 2 3
        // 4 5 6
        // 7 8 9
        //
        // row_sum = [6, 15, 24]
        // col_sum = [12, 15, 18]
        //
        // ans[0][0] = 6 + 12 - 1 = 17
        // ans[0][1] = 6 + 15 - 2 = 19
        // ans[1][1] = 15 + 15 - 5 = 25

        let a = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let row_sum: Vec<i64> = a.iter().map(|row| row.iter().sum()).collect();
        let col_sum: Vec<i64> = (0..3).map(|j| (0..3).map(|i| a[i][j]).sum()).collect();

        assert_eq!(row_sum, vec![6, 15, 24]);
        assert_eq!(col_sum, vec![12, 15, 18]);
        assert_eq!(row_sum[0] + col_sum[0] - a[0][0], 17);
        assert_eq!(row_sum[1] + col_sum[1] - a[1][1], 25);
    }
}
