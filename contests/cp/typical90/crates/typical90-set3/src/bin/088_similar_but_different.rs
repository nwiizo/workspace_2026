// 088 - Similar but Different Ways (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_cj
//
// 半分全列挙 (Meet in the middle)
// 合計が同じで異なる部分集合を2つ見つける
// 禁止ペアの条件は出力時に確認

use proconio::input;
use std::collections::HashMap;

fn main() {
    input! {
        n: usize,
        q: usize,
        a: [i64; n],
        forbidden: [(usize, usize); q],
    }

    if let Some((set1, set2)) = solve(n, &a, &forbidden) {
        println!("{}", set1.len());
        let s1: Vec<String> = set1.iter().map(|x| x.to_string()).collect();
        println!("{}", s1.join(" "));
        println!("{}", set2.len());
        let s2: Vec<String> = set2.iter().map(|x| x.to_string()).collect();
        println!("{}", s2.join(" "));
    }
}

fn solve(n: usize, a: &[i64], forbidden: &[(usize, usize)]) -> Option<(Vec<usize>, Vec<usize>)> {
    // 禁止ペアのセット
    let mut forbidden_set = std::collections::HashSet::new();
    for &(x, y) in forbidden {
        forbidden_set.insert((x.min(y), x.max(y)));
    }

    // 部分集合の列挙: 和 -> 部分集合のリスト
    let mut sum_to_subsets: HashMap<i64, Vec<u64>> = HashMap::new();

    // 全ての部分集合を列挙 (N ≤ 88 だが和 ≤ 8888 なので重複が多い)
    // N が大きい場合は半分全列挙が必要だが、ここでは愚直解
    // 実際には N ≤ 88 で全探索は無理なので、鳩の巣原理を使う

    // 鳩の巣原理: 部分集合は 2^N 個、和の種類は高々 8889 個
    // 平均 2^N / 8889 個の部分集合が同じ和を持つ
    // N = 44 なら 2^44 / 8889 ≈ 2 * 10^9 個なので、
    // 少なくとも 2 つの異なる部分集合が同じ和を持つ

    // 実装: 前半と後半に分けて、それぞれの部分集合の和を列挙

    let half = n / 2;

    // 前半の部分集合
    let mut first_half: HashMap<i64, Vec<u64>> = HashMap::new();
    for mask in 0..(1u64 << half) {
        let mut sum = 0i64;
        for i in 0..half {
            if (mask >> i) & 1 == 1 {
                sum += a[i];
            }
        }
        first_half.entry(sum).or_insert_with(Vec::new).push(mask);
    }

    // 後半の部分集合
    let rest = n - half;
    let mut second_half: HashMap<i64, Vec<u64>> = HashMap::new();
    for mask in 0..(1u64 << rest) {
        let mut sum = 0i64;
        for i in 0..rest {
            if (mask >> i) & 1 == 1 {
                sum += a[half + i];
            }
        }
        second_half.entry(sum).or_insert_with(Vec::new).push(mask);
    }

    // 全ての部分集合を結合
    for (&sum1, masks1) in &first_half {
        for &mask1 in masks1 {
            for (&sum2, masks2) in &second_half {
                for &mask2 in masks2 {
                    let total_sum = sum1 + sum2;
                    let combined_mask = mask1 | (mask2 << half);
                    if combined_mask != 0 {
                        sum_to_subsets
                            .entry(total_sum)
                            .or_insert_with(Vec::new)
                            .push(combined_mask);
                    }
                }
            }
        }
    }

    // 同じ和を持つ異なる部分集合のペアを探す
    for (_sum, subsets) in &sum_to_subsets {
        if subsets.len() >= 2 {
            // 異なるペアを試す
            for i in 0..subsets.len() {
                for j in (i + 1)..subsets.len() {
                    let mask1 = subsets[i];
                    let mask2 = subsets[j];

                    if mask1 == mask2 {
                        continue;
                    }

                    // 禁止ペアのチェック
                    let check_forbidden = |mask: u64| -> bool {
                        for &(x, y) in forbidden {
                            let bx = (mask >> (x - 1)) & 1;
                            let by = (mask >> (y - 1)) & 1;
                            if bx == 1 && by == 1 {
                                return false;
                            }
                        }
                        true
                    };

                    if check_forbidden(mask1) && check_forbidden(mask2) {
                        // 有効なペア
                        let to_vec = |mask: u64| -> Vec<usize> {
                            (0..n)
                                .filter(|&i| (mask >> i) & 1 == 1)
                                .map(|i| i + 1)
                                .collect()
                        };
                        return Some((to_vec(mask1), to_vec(mask2)));
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let a = vec![3, 1, 3, 2, 3];
        let forbidden = vec![(1, 2), (1, 4)];
        let result = solve(5, &a, &forbidden);
        assert!(result.is_some());
        let (set1, set2) = result.unwrap();

        // 和が等しいことを確認
        let sum1: i64 = set1.iter().map(|&i| a[i - 1]).sum();
        let sum2: i64 = set2.iter().map(|&i| a[i - 1]).sum();
        assert_eq!(sum1, sum2);

        // 集合が異なることを確認
        assert_ne!(set1, set2);
    }
}
