//! 包括的テスト
//!
//! 各問題のエッジケース、境界条件、ストレステストを網羅

use typical90::testing::{random_array, random_tree};

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
        // 切り込みは a[i] < L の制約があるため、L未満の位置のみ
        // N=99999 cuts at 10000, 20000, ..., 990000000
        // K=50000 pieces to create → use 50000 cuts → 50001 pieces
        // Each piece has length >= 10000
        assert_eq!(
            solve(
                1000000000,
                50000,
                &(1..100000).map(|i: i64| i * 10000).collect::<Vec<_>>()
            ),
            10000
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
        // binary_search_min は (lo, hi] の範囲で check が true となる最小値を返す
        // lo=-1 にすることで target=0 も正しくテストできる
        for target in 0..100 {
            let result = binary_search_min(-1, 200, |x| x >= target);
            assert_eq!(result, target);
        }
    }

    #[test]
    fn edge_single_element_range() {
        // 単一要素の範囲
        assert_eq!(binary_search_max(5, 6, |x| x <= 5), 5);
        assert_eq!(binary_search_min(4, 6, |x| x >= 5), 5);
    }

    #[test]
    fn edge_always_true() {
        // 常に true の場合
        assert_eq!(binary_search_max(0, 100, |_| true), 99);
        assert_eq!(binary_search_min(0, 100, |_| true), 1);
    }

    #[test]
    fn edge_always_false() {
        // 常に false の場合
        assert_eq!(binary_search_max(0, 100, |_| false), 0);
        assert_eq!(binary_search_min(0, 100, |_| false), 100);
    }
}

// ============================================================
// 034 - There Are Few Types エッジケース
// ============================================================
mod there_are_few_types {
    use std::collections::HashMap;

    fn solve(n: usize, k: usize, a: &[i64]) -> usize {
        if k == 0 {
            return 0;
        }
        let mut count: HashMap<i64, usize> = HashMap::new();
        let mut left = 0;
        let mut max_len = 0;
        for right in 0..n {
            *count.entry(a[right]).or_insert(0) += 1;
            while count.len() > k {
                let c = count.get_mut(&a[left]).unwrap();
                *c -= 1;
                if *c == 0 {
                    count.remove(&a[left]);
                }
                left += 1;
            }
            max_len = max_len.max(right - left + 1);
        }
        max_len
    }

    #[test]
    fn edge_empty() {
        assert_eq!(solve(0, 1, &[]), 0);
    }

    #[test]
    fn edge_k_zero() {
        assert_eq!(solve(5, 0, &[1, 2, 3, 4, 5]), 0);
    }

    #[test]
    fn edge_single_element() {
        assert_eq!(solve(1, 1, &[42]), 1);
        assert_eq!(solve(1, 0, &[42]), 0);
    }

    #[test]
    fn edge_all_same() {
        assert_eq!(solve(5, 1, &[7, 7, 7, 7, 7]), 5);
    }

    #[test]
    fn edge_all_different() {
        assert_eq!(solve(5, 1, &[1, 2, 3, 4, 5]), 1);
        assert_eq!(solve(5, 5, &[1, 2, 3, 4, 5]), 5);
    }

    #[test]
    fn edge_k_greater_than_types() {
        // K が種類数より大きい場合は全体が答え
        assert_eq!(solve(5, 10, &[1, 2, 3, 4, 5]), 5);
    }

    #[test]
    fn edge_large_values() {
        // 値が大きい場合
        assert_eq!(solve(3, 2, &[1_000_000_000, 1_000_000_000, 999_999_999]), 3);
    }
}

// ============================================================
// 036 - Max Manhattan エッジケース
// ============================================================
mod max_manhattan {
    fn solve(n: usize, points: &[(i64, i64)], queries: &[usize]) -> Vec<i64> {
        let transformed: Vec<(i64, i64)> = points.iter().map(|&(x, y)| (x + y, x - y)).collect();
        let mut u_min = vec![i64::MAX; n + 1];
        let mut u_max = vec![i64::MIN; n + 1];
        let mut v_min = vec![i64::MAX; n + 1];
        let mut v_max = vec![i64::MIN; n + 1];
        for i in 0..n {
            let (u, v) = transformed[i];
            u_min[i + 1] = u_min[i].min(u);
            u_max[i + 1] = u_max[i].max(u);
            v_min[i + 1] = v_min[i].min(v);
            v_max[i + 1] = v_max[i].max(v);
        }
        queries
            .iter()
            .map(|&qi| {
                let (u, v) = transformed[qi - 1];
                let max_u_diff = (u - u_min[n]).abs().max((u - u_max[n]).abs());
                let max_v_diff = (v - v_min[n]).abs().max((v - v_max[n]).abs());
                max_u_diff.max(max_v_diff)
            })
            .collect()
    }

    #[test]
    fn edge_single_point() {
        // 1点のみ: 自身への距離は0
        let result = solve(1, &[(0, 0)], &[1]);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn edge_two_points() {
        let result = solve(2, &[(0, 0), (3, 4)], &[1, 2]);
        // (0,0) から (3,4) へのマンハッタン距離 = 7
        assert_eq!(result, vec![7, 7]);
    }

    #[test]
    fn edge_negative_coords() {
        let result = solve(2, &[(-100, -100), (100, 100)], &[1]);
        // マンハッタン距離 = |200| + |200| = 400
        assert_eq!(result, vec![400]);
    }

    #[test]
    fn edge_same_points() {
        // 全て同じ点
        let result = solve(3, &[(5, 5), (5, 5), (5, 5)], &[1, 2, 3]);
        assert_eq!(result, vec![0, 0, 0]);
    }

    #[test]
    fn edge_large_coords() {
        // 座標が大きい場合
        let result = solve(2, &[(1_000_000_000, 1_000_000_000), (0, 0)], &[1]);
        assert_eq!(result, vec![2_000_000_000]);
    }
}

// ============================================================
// 039 - Tree Distance エッジケース
// ============================================================
mod tree_distance {
    fn solve(n: usize, edges: &[(usize, usize)]) -> i64 {
        if n == 1 {
            return 0;
        }
        let mut graph = vec![vec![]; n];
        for &(a, b) in edges {
            graph[a].push(b);
            graph[b].push(a);
        }
        fn dfs(v: usize, parent: isize, graph: &[Vec<usize>], size: &mut [usize]) {
            size[v] = 1;
            for &next in &graph[v] {
                if next as isize != parent {
                    dfs(next, v as isize, graph, size);
                    size[v] += size[next];
                }
            }
        }
        let mut size = vec![0usize; n];
        dfs(0, -1, &graph, &mut size);
        let mut total = 0i64;
        for &(a, b) in edges {
            let s = size[a].min(size[b]);
            total += s as i64 * (n - s) as i64;
        }
        total
    }

    #[test]
    fn edge_single_node() {
        assert_eq!(solve(1, &[]), 0);
    }

    #[test]
    fn edge_two_nodes() {
        assert_eq!(solve(2, &[(0, 1)]), 1);
    }

    #[test]
    fn edge_path_graph() {
        // パス: 0-1-2-3
        // 辺(0,1): 1*3=3, 辺(1,2): 2*2=4, 辺(2,3): 3*1=3
        // 合計: 10
        assert_eq!(solve(4, &[(0, 1), (1, 2), (2, 3)]), 10);
    }

    #[test]
    fn edge_star_graph() {
        // スター: 中心0に1,2,3,4が接続
        // 各辺は 1 * 4 = 4 を通る
        // 4辺 × 4 = 16
        assert_eq!(solve(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]), 16);
    }

    #[test]
    fn edge_complete_binary() {
        // 完全二分木: 0-1, 0-2, 1-3, 1-4, 2-5, 2-6
        //       0
        //      / \
        //     1   2
        //    / \ / \
        //   3  4 5  6
        let edges = vec![(0, 1), (0, 2), (1, 3), (1, 4), (2, 5), (2, 6)];
        let result = solve(7, &edges);
        // 各辺を通るパス数を計算
        assert!(result > 0);
    }
}

// ============================================================
// 030 - K Factors エッジケース
// ============================================================
mod k_factors {
    fn solve(n: usize, k: usize) -> usize {
        let mut factor_count = vec![0usize; n + 1];
        for p in 2..=n {
            if factor_count[p] == 0 {
                for multiple in (p..=n).step_by(p) {
                    factor_count[multiple] += 1;
                }
            }
        }
        factor_count[2..=n].iter().filter(|&&c| c == k).count()
    }

    #[test]
    fn edge_n_equals_1() {
        // N=1 では 2以上の数がない
        assert_eq!(solve(1, 1), 0);
    }

    #[test]
    fn edge_k_equals_0() {
        // 素因数0個の数は存在しない（1以外、だが1は範囲外）
        assert_eq!(solve(10, 0), 0);
    }

    #[test]
    fn edge_primes_only() {
        // K=1: 素数とその累乗
        // 2,3,4,5,7,8,9,11,13 (10以下)
        assert!(solve(10, 1) >= 4); // 少なくとも素数 2,3,5,7
    }

    #[test]
    fn edge_large_k() {
        // 大きなKでは該当なし
        assert_eq!(solve(100, 10), 0);
    }

    #[test]
    fn edge_k_equals_3() {
        // 30 = 2*3*5 が最小
        assert_eq!(solve(29, 3), 0);
        assert!(solve(30, 3) >= 1);
    }
}

// ============================================================
// DP アルゴリズム エッジケース
// ============================================================
mod dp_edge_cases {
    use typical90::dp::{edit_distance, lcs, lcs_length, lis, lis_restore};

    #[test]
    fn lis_empty() {
        let empty: Vec<i32> = vec![];
        assert_eq!(lis(&empty, true), 0);
        assert_eq!(lis(&empty, false), 0);
    }

    #[test]
    fn lis_single() {
        assert_eq!(lis(&[42], true), 1);
        assert_eq!(lis(&[42], false), 1);
    }

    #[test]
    fn lis_decreasing() {
        assert_eq!(lis(&[5, 4, 3, 2, 1], true), 1);
        assert_eq!(lis(&[5, 4, 3, 2, 1], false), 1);
    }

    #[test]
    fn lis_all_equal() {
        assert_eq!(lis(&[3, 3, 3, 3, 3], true), 1);
        assert_eq!(lis(&[3, 3, 3, 3, 3], false), 5);
    }

    #[test]
    fn lis_restore_correctness() {
        let a = vec![2, 1, 5, 3, 6, 4, 8, 9, 7];
        let indices = lis_restore(&a, true);
        // 復元された列が実際に増加列になっていることを確認
        for i in 1..indices.len() {
            assert!(a[indices[i - 1]] < a[indices[i]]);
        }
    }

    #[test]
    fn lcs_empty() {
        let empty: Vec<char> = vec![];
        let a: Vec<char> = "abc".chars().collect();
        assert_eq!(lcs_length(&empty, &a), 0);
        assert_eq!(lcs_length(&a, &empty), 0);
        assert_eq!(lcs(&empty, &a), Vec::<char>::new());
    }

    #[test]
    fn lcs_no_common() {
        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "xyz".chars().collect();
        assert_eq!(lcs_length(&a, &b), 0);
    }

    #[test]
    fn lcs_identical() {
        let a: Vec<char> = "hello".chars().collect();
        assert_eq!(lcs_length(&a, &a), 5);
        assert_eq!(lcs(&a, &a), a);
    }

    #[test]
    fn edit_distance_empty() {
        let empty: Vec<char> = vec![];
        let a: Vec<char> = "abc".chars().collect();
        assert_eq!(edit_distance(&empty, &a), 3);
        assert_eq!(edit_distance(&a, &empty), 3);
    }

    #[test]
    fn edit_distance_identical() {
        let a: Vec<char> = "hello".chars().collect();
        assert_eq!(edit_distance(&a, &a), 0);
    }

    #[test]
    fn edit_distance_single_char() {
        let a: Vec<char> = "a".chars().collect();
        let b: Vec<char> = "b".chars().collect();
        assert_eq!(edit_distance(&a, &b), 1);
    }
}

// ============================================================
// グラフアルゴリズム エッジケース
// ============================================================
mod graph_edge_cases {
    use typical90::graph::{UnionFind, dijkstra};

    #[test]
    fn union_find_single() {
        let mut uf = UnionFind::new(1);
        assert!(uf.same(0, 0));
        assert_eq!(uf.group_size(0), 1);
    }

    #[test]
    fn union_find_no_unions() {
        let mut uf = UnionFind::new(5);
        for i in 0..5 {
            for j in 0..5 {
                assert_eq!(uf.same(i, j), i == j);
            }
        }
    }

    #[test]
    fn union_find_all_united() {
        let mut uf = UnionFind::new(5);
        for i in 1..5 {
            uf.unite(0, i);
        }
        for i in 0..5 {
            for j in 0..5 {
                assert!(uf.same(i, j));
            }
        }
        assert_eq!(uf.group_size(0), 5);
    }

    #[test]
    fn dijkstra_single_node() {
        let graph: Vec<Vec<(usize, i64)>> = vec![vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist, vec![0]);
    }

    #[test]
    fn dijkstra_disconnected() {
        let graph: Vec<Vec<(usize, i64)>> = vec![vec![], vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], i64::MAX);
    }

    #[test]
    fn dijkstra_self_loop() {
        // 自己ループは無視される
        let graph: Vec<Vec<(usize, i64)>> = vec![vec![(0, 5), (1, 3)], vec![]];
        let dist = dijkstra(&graph, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 3);
    }
}

// ============================================================
// 文字列アルゴリズム エッジケース
// ============================================================
mod string_edge_cases {
    use typical90::string_algo::{count_occurrences, lcp_array, suffix_array, z_algorithm};

    #[test]
    fn z_algorithm_empty() {
        assert_eq!(z_algorithm(b""), vec![]);
    }

    #[test]
    fn z_algorithm_single() {
        assert_eq!(z_algorithm(b"a"), vec![1]);
    }

    #[test]
    fn z_algorithm_all_same() {
        assert_eq!(z_algorithm(b"aaaa"), vec![4, 3, 2, 1]);
    }

    #[test]
    fn z_algorithm_all_different() {
        assert_eq!(z_algorithm(b"abcd"), vec![4, 0, 0, 0]);
    }

    #[test]
    fn suffix_array_empty() {
        assert_eq!(suffix_array(b""), vec![]);
    }

    #[test]
    fn suffix_array_single() {
        assert_eq!(suffix_array(b"a"), vec![0]);
    }

    #[test]
    fn suffix_array_sorted() {
        // "abc" のサフィックス: abc, bc, c
        // 辞書順: abc(0), bc(1), c(2)
        assert_eq!(suffix_array(b"abc"), vec![0, 1, 2]);
    }

    #[test]
    fn lcp_array_single() {
        assert_eq!(lcp_array(b"a", &[0]), vec![]);
    }

    #[test]
    fn count_occurrences_empty_pattern() {
        let s = b"hello";
        let sa = suffix_array(s);
        assert_eq!(count_occurrences(s, &sa, b""), 0);
    }

    #[test]
    fn count_occurrences_not_found() {
        let s = b"hello";
        let sa = suffix_array(s);
        assert_eq!(count_occurrences(s, &sa, b"xyz"), 0);
    }

    #[test]
    fn count_occurrences_full_match() {
        let s = b"hello";
        let sa = suffix_array(s);
        assert_eq!(count_occurrences(s, &sa, b"hello"), 1);
    }
}
