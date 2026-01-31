//! 包括的テスト
//!
//! 各問題のエッジケース、境界条件、ストレステストを網羅

use typical90::testing::{random_array, random_permutation, random_tree, stress_test};

// ============================================================
// 001 - Yokan Party テスト
// ============================================================
mod yokan_party {
    fn solve(l: i64, k: usize, a: &[i64]) -> i64 {
        let can_divide = |min_len: i64| -> bool {
            let pieces = a
                .iter()
                .chain(std::iter::once(&l))
                .fold((0usize, 0i64), |(count, prev), &pos| {
                    if pos - prev >= min_len {
                        (count + 1, pos)
                    } else {
                        (count, prev)
                    }
                })
                .0;
            pieces > k
        };

        let (mut lo, mut hi) = (0i64, l + 1);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if can_divide(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    #[test]
    fn atcoder_example1() {
        assert_eq!(solve(34, 1, &[8, 13, 26]), 13);
    }

    #[test]
    fn atcoder_example2() {
        assert_eq!(solve(34, 2, &[8, 13, 26]), 8);
    }

    #[test]
    fn atcoder_example3() {
        assert_eq!(
            solve(
                1000000000,
                100000,
                &(1..=100000).map(|i| i * 10000).collect::<Vec<_>>()
            ),
            9999
        );
    }

    #[test]
    fn edge_single_cut() {
        assert_eq!(solve(10, 1, &[5]), 5);
    }

    #[test]
    fn edge_all_cuts() {
        // 全ての切り込みを使う場合
        assert_eq!(solve(10, 4, &[2, 4, 6, 8]), 2);
    }

    #[test]
    fn edge_no_cuts_used() {
        // 1つも使わない場合 (K=0)
        assert_eq!(solve(100, 0, &[10, 20, 30]), 100);
    }

    #[test]
    fn edge_maximum_l() {
        // L = 10^9
        assert_eq!(solve(1_000_000_000, 1, &[500_000_000]), 500_000_000);
    }
}

// ============================================================
// 003 - Longest Circular Road テスト
// ============================================================
mod longest_circular_road {
    use std::collections::VecDeque;

    fn solve(n: usize, edges: &[(usize, usize)]) -> usize {
        let mut graph = vec![vec![]; n];
        for &(a, b) in edges {
            graph[a].push(b);
            graph[b].push(a);
        }

        let bfs_farthest = |start: usize| -> (usize, usize) {
            let mut dist = vec![usize::MAX; n];
            let mut queue = VecDeque::from([start]);
            dist[start] = 0;

            while let Some(v) = queue.pop_front() {
                for &next in &graph[v] {
                    if dist[next] == usize::MAX {
                        dist[next] = dist[v] + 1;
                        queue.push_back(next);
                    }
                }
            }

            dist.into_iter()
                .enumerate()
                .filter(|&(_, d)| d != usize::MAX)
                .max_by_key(|&(_, d)| d)
                .unwrap_or((start, 0))
        };

        let (u, _) = bfs_farthest(0);
        let (_, diameter) = bfs_farthest(u);
        diameter + 1
    }

    #[test]
    fn atcoder_example1() {
        // パスグラフ
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        assert_eq!(solve(4, &edges), 4);
    }

    #[test]
    fn star_graph() {
        let edges = vec![(0, 1), (0, 2), (0, 3), (0, 4)];
        assert_eq!(solve(5, &edges), 3);
    }

    #[test]
    fn two_nodes() {
        let edges = vec![(0, 1)];
        assert_eq!(solve(2, &edges), 2);
    }

    #[test]
    fn single_node() {
        let edges: Vec<(usize, usize)> = vec![];
        assert_eq!(solve(1, &edges), 1);
    }

    #[test]
    fn caterpillar_tree() {
        // 毛虫型: 中央のパスから葉が生えている
        // 0-1-2-3-4 (中央パス)
        // 1-5, 2-6, 3-7 (葉)
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (1, 5), (2, 6), (3, 7)];
        assert_eq!(solve(8, &edges), 5); // 直径4 + 1
    }

    #[test]
    fn stress_random_trees() {
        for seed in 0..10 {
            let n = 100;
            let edges = super::random_tree(seed, n);
            let result = solve(n, &edges);
            // サイクル長は最低2（2頂点以上の場合）
            assert!(result >= 2);
            // サイクル長は最大n（全頂点を通る場合）
            assert!(result <= n);
        }
    }
}

// ============================================================
// 022 - Cubic Cake テスト
// ============================================================
mod cubic_cake {
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }

    fn solve(a: i64, b: i64, c: i64) -> i64 {
        let g = gcd(gcd(a, b), c);
        (a / g - 1) + (b / g - 1) + (c / g - 1)
    }

    #[test]
    fn atcoder_example1() {
        assert_eq!(solve(2, 2, 2), 0);
    }

    #[test]
    fn atcoder_example2() {
        assert_eq!(solve(2, 6, 4), 3);
    }

    #[test]
    fn cube_1x1x1() {
        assert_eq!(solve(1, 1, 1), 0);
    }

    #[test]
    fn large_values() {
        // 10^18 レベルの値
        assert_eq!(
            solve(1_000_000_000_000, 1_000_000_000_000, 1_000_000_000_000),
            0
        );
    }

    #[test]
    fn coprime() {
        // 互いに素な場合、gcd=1
        assert_eq!(solve(3, 5, 7), (3 - 1) + (5 - 1) + (7 - 1));
    }

    #[test]
    fn one_side_is_one() {
        assert_eq!(solve(1, 10, 100), 0 + 9 + 99);
    }
}

// ============================================================
// 024 - Select +/- One テスト
// ============================================================
mod select_plus_minus {
    fn solve(a: &[i64], b: &[i64], k: i64) -> bool {
        let diff: i64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
        diff <= k && (k - diff) % 2 == 0
    }

    #[test]
    fn atcoder_example1() {
        assert!(solve(&[3, 4, 1], &[5, 2, 3], 6));
    }

    #[test]
    fn atcoder_example2() {
        assert!(!solve(&[1], &[2], 2));
    }

    #[test]
    fn atcoder_example3() {
        assert!(solve(&[1], &[2], 3));
    }

    #[test]
    fn same_arrays() {
        assert!(solve(&[1, 2, 3], &[1, 2, 3], 0));
        assert!(solve(&[1, 2, 3], &[1, 2, 3], 2)); // 余り偶数
        assert!(!solve(&[1, 2, 3], &[1, 2, 3], 1)); // 余り奇数
    }

    #[test]
    fn not_enough_k() {
        assert!(!solve(&[1], &[100], 50));
    }

    #[test]
    fn large_k() {
        assert!(solve(&[1], &[2], 101)); // diff=1, k=101, (101-1)=100 偶数
        assert!(!solve(&[1], &[2], 100)); // diff=1, k=100, (100-1)=99 奇数
    }
}

// ============================================================
// 038 - Large LCS テスト
// ============================================================
mod large_lcs {
    fn solve(s: &str, t: &str) -> String {
        let s: Vec<char> = s.chars().collect();
        let t: Vec<char> = t.chars().collect();
        let n = s.len();
        let m = t.len();

        let mut dp = vec![vec![0usize; m + 1]; n + 1];

        for i in 1..=n {
            for j in 1..=m {
                if s[i - 1] == t[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        let mut result = Vec::new();
        let (mut i, mut j) = (n, m);

        while i > 0 && j > 0 {
            if s[i - 1] == t[j - 1] {
                result.push(s[i - 1]);
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        result.reverse();
        result.iter().collect()
    }

    #[test]
    fn atcoder_example() {
        let result = solve("abcde", "ace");
        assert_eq!(result, "ace");
    }

    #[test]
    fn no_common() {
        let result = solve("abc", "xyz");
        assert_eq!(result, "");
    }

    #[test]
    fn same_string() {
        let result = solve("hello", "hello");
        assert_eq!(result, "hello");
    }

    #[test]
    fn one_empty() {
        assert_eq!(solve("", "abc"), "");
        assert_eq!(solve("abc", ""), "");
    }

    #[test]
    fn single_char() {
        assert_eq!(solve("a", "a"), "a");
        assert_eq!(solve("a", "b"), "");
    }

    #[test]
    fn subsequence() {
        let result = solve("abcdefg", "aceg");
        assert_eq!(result, "aceg");
    }
}

// ============================================================
// Union-Find ストレステスト
// ============================================================
mod union_find_stress {
    use typical90::graph::UnionFind;

    #[test]
    fn large_scale() {
        let n = 100_000;
        let mut uf = UnionFind::new(n);

        // 全てを1つのグループに
        for i in 1..n {
            uf.unite(0, i);
        }

        assert!(uf.same(0, n - 1));
        assert_eq!(uf.group_size(0), n);
    }

    #[test]
    fn chain_unite() {
        let n = 10_000;
        let mut uf = UnionFind::new(n);

        // チェーン状に連結
        for i in 0..n - 1 {
            uf.unite(i, i + 1);
        }

        assert!(uf.same(0, n - 1));
    }

    #[test]
    fn many_groups() {
        let n = 10_000;
        let mut uf = UnionFind::new(n);

        // 偶数同士、奇数同士を連結
        for i in (0..n - 2).step_by(2) {
            uf.unite(i, i + 2);
        }
        for i in (1..n - 2).step_by(2) {
            uf.unite(i, i + 2);
        }

        assert!(uf.same(0, n - 2));
        assert!(uf.same(1, n - 1));
        assert!(!uf.same(0, 1));
    }
}

// ============================================================
// セグ木ストレステスト
// ============================================================
mod segtree_stress {
    use typical90::segtree::{Max, Min, SegTree, Sum};

    #[test]
    fn sum_large() {
        let n = 100_000;
        let mut seg: SegTree<Sum> = SegTree::new(n);

        for i in 0..n {
            seg.set(i, Sum(i as i64));
        }

        // 全体の和 = 0 + 1 + ... + (n-1) = n*(n-1)/2
        let expected = (n * (n - 1) / 2) as i64;
        assert_eq!(seg.query(0, n).0, expected);
    }

    #[test]
    fn max_random() {
        let arr: Vec<_> = super::random_array(42, 1000, -1000, 1000)
            .into_iter()
            .map(Max)
            .collect();
        let seg = SegTree::from_vec(&arr);

        // 愚直解と比較
        for l in (0..100).step_by(10) {
            for r in (l + 1..=100).step_by(10) {
                let expected = arr[l..r].iter().map(|x| x.0).max().unwrap();
                assert_eq!(seg.query(l, r).0, expected);
            }
        }
    }

    #[test]
    fn min_random() {
        let arr: Vec<_> = super::random_array(123, 1000, -1000, 1000)
            .into_iter()
            .map(Min)
            .collect();
        let seg = SegTree::from_vec(&arr);

        for l in (0..100).step_by(10) {
            for r in (l + 1..=100).step_by(10) {
                let expected = arr[l..r].iter().map(|x| x.0).min().unwrap();
                assert_eq!(seg.query(l, r).0, expected);
            }
        }
    }
}

// ============================================================
// ModInt テスト
// ============================================================
mod modint_stress {
    use typical90::modint::Mint998;

    #[test]
    fn fermat_little_theorem() {
        // a^(p-1) ≡ 1 (mod p) for prime p
        for a in 1..100 {
            let x = Mint998::new(a);
            assert_eq!(x.pow(998244353 - 1).val(), 1);
        }
    }

    #[test]
    fn inverse_property() {
        // a * a^(-1) ≡ 1 (mod p)
        for a in 1..100 {
            let x = Mint998::new(a);
            assert_eq!((x * x.inv()).val(), 1);
        }
    }

    #[test]
    fn distributive() {
        // (a + b) * c = a*c + b*c
        for a in 0..50 {
            for b in 0..50 {
                for c in 0..50 {
                    let ma = Mint998::new(a);
                    let mb = Mint998::new(b);
                    let mc = Mint998::new(c);
                    assert_eq!(((ma + mb) * mc).val(), (ma * mc + mb * mc).val());
                }
            }
        }
    }
}

// ============================================================
// 二分探索ストレステスト
// ============================================================
mod binary_search_stress {
    use typical90::search::{binary_search_max, binary_search_min};

    #[test]
    fn stress_max() {
        for target in 0..100 {
            let result = binary_search_max(0, 200, |x| x <= target);
            assert_eq!(result, target);
        }
    }

    #[test]
    fn stress_min() {
        for target in 0..100 {
            let result = binary_search_min(0, 200, |x| x >= target);
            assert_eq!(result, target);
        }
    }
}
