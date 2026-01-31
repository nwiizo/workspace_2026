// 051 - Typical Shop (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_ay
//
// ============================================================
// 半分全列挙 (Meet in the Middle)
// ============================================================
//
// N ≤ 40 なので 2^40 は直接列挙できない
// しかし 2^20 ≈ 100万 なら可能
//
// アルゴリズム:
// 1. 品物を前半(N/2個)と後半に分ける
// 2. 前半からi個選ぶ場合の合計金額リストを作成
// 3. 後半から(K-i)個選ぶ場合の合計金額リストを作成
// 4. 前半の各選び方について、後半から二分探索でマッチング
//
// 計算量: O(2^(N/2) * log(2^(N/2)))
//
// ============================================================

use proconio::input;

fn main() {
    input! {
        n: usize,
        k: usize,
        p: i64,
        a: [i64; n],
    }
    println!("{}", solve(n, k, p, &a));
}

fn solve(n: usize, k: usize, p: i64, a: &[i64]) -> i64 {
    // 前半と後半に分割
    let mid = n / 2;
    let first_half = &a[..mid];
    let second_half = &a[mid..];

    // 前半から選ぶ場合の (選んだ個数, 合計金額) のリスト
    // sums1[i] = 前半からちょうどi個選んだ場合の合計金額リスト
    let sums1 = enumerate_sums(first_half, k);

    // 後半から選ぶ場合
    let mut sums2 = enumerate_sums(second_half, k);

    // 後半のリストをソート（二分探索用）
    for list in sums2.iter_mut() {
        list.sort();
    }

    // マッチング
    let mut count = 0i64;

    for i in 0..=k.min(mid) {
        let j = k - i; // 後半から選ぶ個数
        if j > second_half.len() {
            continue;
        }

        // 前半からi個選んだ各合計について
        for &sum1 in &sums1[i] {
            if sum1 > p {
                continue;
            }
            let remaining = p - sum1;
            // 後半でremaining以下の選び方をカウント
            let cnt = upper_bound(&sums2[j], remaining);
            count += cnt as i64;
        }
    }

    count
}

// 品物リストから0〜max_count個選ぶ場合の合計金額を列挙
fn enumerate_sums(items: &[i64], max_count: usize) -> Vec<Vec<i64>> {
    let n = items.len();
    let mut result = vec![vec![]; max_count + 1];

    // bit全探索
    for mask in 0..(1u64 << n) {
        let count = mask.count_ones() as usize;
        if count > max_count {
            continue;
        }

        let sum: i64 = (0..n)
            .filter(|&i| (mask >> i) & 1 == 1)
            .map(|i| items[i])
            .sum();

        result[count].push(sum);
    }

    result
}

// sorted配列でvalue以下の要素数を返す
fn upper_bound(sorted: &[i64], value: i64) -> usize {
    sorted.partition_point(|&x| x <= value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // 品物1(3)+品物3(7)=10, 品物1(3)+品物4(5)=8
        let a = vec![3, 8, 7, 5, 11];
        assert_eq!(solve(5, 2, 10, &a), 2);
    }

    #[test]
    fn test_example2() {
        // すべて7円、1個選ぶので5通り
        let a = vec![7, 7, 7, 7, 7];
        assert_eq!(solve(5, 1, 10, &a), 5);
    }

    #[test]
    fn test_example3() {
        // 40C20 = 137846528820
        let a = vec![1; 40];
        assert_eq!(solve(40, 20, 100, &a), 137846528820);
    }
}
