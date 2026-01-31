// 062 - Paint All (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_bj
//
// 逆順で考える：最後に使うアイテムから順に決める
//
// 全てのボールが黒い状態から開始し、
// 「使える」アイテムを順に取り出して白に戻す

use proconio::input;
use proconio::marker::Usize1;
use std::collections::VecDeque;

fn main() {
    input! {
        n: usize,
        items: [(Usize1, Usize1); n], // (A_i, B_i) 0-indexed
    }

    match solve(n, &items) {
        Some(order) => {
            for x in order {
                println!("{}", x + 1); // 1-indexed
            }
        }
        None => println!("-1"),
    }
}

fn solve(n: usize, items: &[(usize, usize)]) -> Option<Vec<usize>> {
    // 各ボールを参照しているアイテムのリスト
    let mut refs: Vec<Vec<usize>> = vec![vec![]; n];
    for (i, &(a, b)) in items.iter().enumerate() {
        refs[a].push(i);
        refs[b].push(i);
    }

    // 逆順で考える：最後に使えるアイテムから順に決める
    // 全て黒の状態から、アイテムを「戻す」（ボールを白くする）
    // アイテムiを戻せる条件：A_i == i または B_i == i
    // （自分自身のボールが参照されていれば、そのボールが白の時に使える）

    let mut is_white = vec![false; n]; // 逆順なので最初は全て黒
    let mut used = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut queue = VecDeque::new();

    // 最後に使えるアイテム：A_i == i または B_i == i
    for i in 0..n {
        let (a, b) = items[i];
        if a == i || b == i {
            queue.push_back(i);
            used[i] = true;
        }
    }

    while let Some(item) = queue.pop_front() {
        order.push(item);

        // ボールitemを白に戻す
        is_white[item] = true;

        // このボールを参照するアイテムをチェック
        for &j in &refs[item] {
            if !used[j] {
                // jが使えるようになったかチェック
                // A_j または B_j が白なら使える
                let (a, b) = items[j];
                if is_white[a] || is_white[b] {
                    used[j] = true;
                    queue.push_back(j);
                }
            }
        }
    }

    if order.len() == n {
        order.reverse(); // 逆順にして正順に
        Some(order)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let items = vec![(2, 3), (0, 2), (1, 2), (1, 0)]; // 0-indexed
        let result = solve(4, &items);
        assert!(result.is_some());
        // 順序が正しいかは複数解があるので検証は省略
    }

    #[test]
    fn test_example2() {
        let items = vec![(0, 0), (1, 1), (2, 2)];
        let result = solve(3, &items);
        assert!(result.is_some());
    }

    #[test]
    fn test_example3() {
        let items = vec![(2, 3), (3, 4), (0, 0), (4, 0), (2, 1)];
        let result = solve(5, &items);
        assert!(result.is_none());
    }
}
