// 010 - Score Sum Queries (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_j
//
// 問題: N人の生徒がいて、各生徒はクラス1かクラス2に所属し、点数を持つ。
// Q個のクエリで区間[L, R]のクラスごとの点数合計を求めよ。
//
// 解法: 累積和
// - クラスごとに累積和を前計算
// - 区間和は O(1) で計算可能

use proconio::input;

fn main() {
    input! {
        n: usize,
        students: [(usize, i64); n], // (class, score)
        q: usize,
        queries: [(usize, usize); q], // (l, r) 1-indexed
    }
    solve(n, &students, q, &queries);
}

fn solve(n: usize, students: &[(usize, i64)], _q: usize, queries: &[(usize, usize)]) {
    // クラスごとの累積和
    // prefix1[i] = 最初のi人のうちクラス1の点数合計
    // prefix2[i] = 最初のi人のうちクラス2の点数合計
    let mut prefix1 = vec![0i64; n + 1];
    let mut prefix2 = vec![0i64; n + 1];

    for (i, &(class, score)) in students.iter().enumerate() {
        prefix1[i + 1] = prefix1[i];
        prefix2[i + 1] = prefix2[i];
        if class == 1 {
            prefix1[i + 1] += score;
        } else {
            prefix2[i + 1] += score;
        }
    }

    // クエリ処理
    for &(l, r) in queries {
        let sum1 = prefix1[r] - prefix1[l - 1];
        let sum2 = prefix2[r] - prefix2[l - 1];
        println!("{} {}", sum1, sum2);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cumulative_sum() {
        // 生徒: (1, 10), (2, 20), (1, 30), (2, 40), (1, 50)
        let students = vec![(1, 10), (2, 20), (1, 30), (2, 40), (1, 50)];
        let n = students.len();

        let mut prefix1 = vec![0i64; n + 1];
        let mut prefix2 = vec![0i64; n + 1];

        for (i, &(class, score)) in students.iter().enumerate() {
            prefix1[i + 1] = prefix1[i];
            prefix2[i + 1] = prefix2[i];
            if class == 1 {
                prefix1[i + 1] += score;
            } else {
                prefix2[i + 1] += score;
            }
        }

        // 区間[1, 5]: クラス1: 10+30+50=90, クラス2: 20+40=60
        assert_eq!(prefix1[5] - prefix1[0], 90);
        assert_eq!(prefix2[5] - prefix2[0], 60);

        // 区間[2, 4]: クラス1: 30, クラス2: 20+40=60
        assert_eq!(prefix1[4] - prefix1[1], 30);
        assert_eq!(prefix2[4] - prefix2[1], 60);
    }
}
