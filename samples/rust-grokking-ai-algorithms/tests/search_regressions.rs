use grokking_ai_algorithms::search::{Graph, a_star, minimax};

#[test]
fn minimax_respects_three_binary_levels() {
    // max(min(max(3, 5), max(6, 9)), min(max(1, 2), max(0, -1))) = 5.
    assert_eq!(minimax(&[3, 5, 6, 9, 1, 2, 0, -1], 3, true), Some(5));
}

#[test]
fn maximum_representable_path_cost_is_reachable() {
    assert_eq!(
        a_star(&[vec![(1, u32::MAX)], vec![]], &[0, 0], 0, 1),
        Some((u32::MAX, vec![0, 1]))
    );
}

#[test]
fn dfs_finishes_a_branch_before_visiting_siblings() {
    let mut graph = Graph::new(4);
    for (from, to) in [(0, 1), (0, 2), (1, 2), (2, 3)] {
        graph.add_edge(from, to).unwrap();
    }
    assert_eq!(graph.depth_first_path(0, 3), Some(vec![0, 1, 2, 3]));
}
