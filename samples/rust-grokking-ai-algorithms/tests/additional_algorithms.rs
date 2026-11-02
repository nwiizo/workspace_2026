use grokking_ai_algorithms::{supervised::*, training::grid_search, tuning::*};

#[test]
fn random_forest_learns_xor_with_bootstrap_and_random_features() {
    let mut x = Vec::new();
    let mut y = Vec::new();
    for _ in 0..8 {
        for (a, b) in [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)] {
            x.push(vec![a, b]);
            y.push(usize::from(a != b));
        }
    }
    let forest = RandomForest::fit(&x, &y, 31, 4, 1, 9).unwrap();
    for (row, &target) in x.iter().zip(&y) {
        assert_eq!(forest.predict(row).unwrap(), target);
    }
    assert_eq!(forest.trees.len(), 31);
    assert!(RandomForest::fit(&x, &y, 0, 4, 1, 9).is_err());
}

#[test]
fn knn_imputation_uses_observed_neighbors_and_handles_empty_columns() {
    let rows = vec![
        vec![Some(0.0), Some(10.0)],
        vec![Some(1.0), Some(20.0)],
        vec![Some(0.1), None],
        vec![None, Some(19.0)],
    ];
    let filled = knn_impute(&rows, 1).unwrap();
    assert_eq!(filled[2], [0.1, 10.0]);
    assert_eq!(filled[3], [1.0, 19.0]);
    assert!(knn_impute(&[vec![None, None]], 1).is_err());
}

#[test]
fn gaussian_process_interpolates_and_quantifies_distance_from_observations() {
    let gp = GaussianProcess::fit(&[vec![-1.0], vec![1.0]], &[2.0, 4.0], 1.0, 1e-8).unwrap();
    let (mean, variance) = gp.predict(&[-1.0]).unwrap();
    assert!((mean - 2.0).abs() < 1e-6);
    assert!(variance < 1e-6);
    assert!(gp.predict(&[5.0]).unwrap().1 > 0.9);
}

#[test]
fn grid_random_and_bayesian_search_evaluate_real_objectives() {
    let grid = cartesian_grid(&[vec![0.01, 0.1], vec![4.0, 8.0]]).unwrap();
    assert_eq!(grid.len(), 4);
    let (best, score) =
        grid_search(&grid, |x| Ok((x[0] - 0.1).powi(2) + (x[1] - 8.0).powi(2))).unwrap();
    assert_eq!((best, score), (vec![0.1, 8.0], 0.0));
    let random = random_search(&[(-2.0, 2.0)], 100, 3, |x| Ok((x[0] - 0.35).powi(2))).unwrap();
    assert!(random.1 < 0.005);
    let candidates: Vec<_> = (-30..=30).map(|v| vec![v as f64 / 10.0]).collect();
    let mut seen = Vec::new();
    let (_, loss) = bayesian_search(&candidates, 15, 3, |x| {
        assert!(!seen.contains(&x[0]));
        seen.push(x[0]);
        Ok((x[0] - 0.4).powi(2))
    })
    .unwrap();
    assert_eq!(seen.len(), 15);
    assert!(loss < 0.011, "{loss}");
}
