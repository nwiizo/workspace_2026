// 078 - Easy Graph Problem (★2)
// https://atcoder.jp/contests/typical90/tasks/typical90_bz
//
// 各頂点について、自分より小さい番号の隣接頂点がちょうど1つある頂点を数える

use proconio::input;

fn main() {
    input! {
        n: usize,
        m: usize,
        edges: [(usize, usize); m],
    }
    println!("{}", solve(n, &edges));
}

fn solve(n: usize, edges: &[(usize, usize)]) -> usize {
    // 各頂点について、自分より小さい番号の隣接頂点の数を数える
    let mut smaller_count = vec![0; n + 1];

    for &(a, b) in edges {
        // a と b のうち大きい方のカウントを増やす
        if a < b {
            smaller_count[b] += 1;
        } else {
            smaller_count[a] += 1;
        }
    }

    // カウントが 1 の頂点を数える
    smaller_count.iter().filter(|&&c| c == 1).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        let edges = vec![(1, 2), (1, 3), (3, 2), (5, 2), (4, 2)];
        assert_eq!(solve(5, &edges), 3);
    }

    #[test]
    fn test_example2() {
        let edges = vec![(1, 2)];
        assert_eq!(solve(2, &edges), 1);
    }

    #[test]
    fn test_example3() {
        // 完全グラフ K7
        let mut edges = vec![];
        for i in 1..=7 {
            for j in (i + 1)..=7 {
                edges.push((i, j));
            }
        }
        assert_eq!(solve(7, &edges), 0);
    }
}
