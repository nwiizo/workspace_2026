use grokking_ai_algorithms::{
    boosting::*, clustering::*, evolution::*, numeric::*, random::Rng, search::*, supervised::*,
    swarm::*,
};

#[test]
fn alpha_beta_matches_minimax_and_prunes() {
    let leaves = [3, 5, 6, 9, 1, 2, 0, -1];
    let (value, visited) = alpha_beta(&leaves, 3, true).unwrap();
    assert_eq!(Some(value), minimax(&leaves, 3, true));
    assert!(visited < leaves.len());
    assert!(minimax(&[1, 2, 3], 2, true).is_none());
    assert_eq!(binary_search(&[-4, 0, 3, 9], 3), Some(2));
    assert_eq!(binary_search(&[], 3), None);
    let mut rng = Rng::new(3);
    for _ in 0..100 {
        let v: Vec<_> = (0..16).map(|_| (rng.next_u64() % 100) as i32).collect();
        assert_eq!(minimax(&v, 4, false), alpha_beta(&v, 4, false).map(|v| v.0));
    }
}

#[test]
fn binary_and_real_operators_preserve_expected_genes() {
    let mut rng = Rng::new(0);
    assert_eq!(
        crossover(&[0, 0, 0, 0], &[1, 1, 1, 1], 1, 3).unwrap(),
        [0, 1, 1, 0]
    );
    assert_eq!(crossover(&[0, 0], &[1, 1], 0, 1).unwrap(), [1, 0]);
    assert_eq!(
        uniform_crossover(&[2, 2], &[2, 2], &mut rng).unwrap(),
        [2, 2]
    );
    assert_eq!(
        arithmetic_crossover(&[2.0, 4.0], &[4.0, 8.0], 0.25).unwrap(),
        [3.5, 7.0]
    );
    let mut bits = [false; 12];
    bit_string_mutation(&mut bits, &mut rng).unwrap();
    assert_eq!(bits.iter().filter(|v| **v).count(), 1);
    flip_bit_mutation(&mut bits);
    assert_eq!(bits.iter().filter(|v| **v).count(), 11);
    let mut values = [0.5];
    real_mutation(&mut values, (0.0, 1.0), true, &mut rng).unwrap();
    assert!([0.0, 1.0].contains(&values[0]));
    real_mutation(&mut values, (0.0, 1.0), false, &mut rng).unwrap();
    assert!((0.0..=1.0).contains(&values[0]));
    for selection in [
        Selection::Roulette,
        Selection::Rank,
        Selection::Tournament(3),
        Selection::Elite,
    ] {
        assert!(select(&[0.0, 1.0, 3.0], selection, &mut rng).unwrap() < 3);
    }
    assert_eq!(
        select(&[0.0, 0.0, 3.0], Selection::Roulette, &mut rng).unwrap(),
        2
    );
    assert_eq!(
        select(&[0.0, 1.0, 3.0], Selection::Elite, &mut rng).unwrap(),
        2
    );
    assert!(select(&[f64::NAN], Selection::Elite, &mut rng).is_err());
}

#[test]
fn every_permutation_operator_preserves_every_city_once() {
    let mut rng = Rng::new(9);
    for _ in 0..30 {
        let mut a: Vec<_> = (0..8).collect();
        let mut b = a.clone();
        rng.shuffle(&mut a);
        rng.shuffle(&mut b);
        for method in [
            PermutationCrossover::Order,
            PermutationCrossover::PartiallyMapped,
            PermutationCrossover::Cycle,
        ] {
            let mut child = permutation_crossover(&a, &b, 2..6, method).unwrap();
            child.sort_unstable();
            assert_eq!(child, (0..8).collect::<Vec<_>>());
        }
        order_mutation(&mut a, &mut rng).unwrap();
        a.sort_unstable();
        assert_eq!(a, (0..8).collect::<Vec<_>>());
    }
}

#[test]
fn genetic_knapsack_matches_exhaustive_optimum() {
    let items = [(2, 3), (3, 4), (4, 5), (5, 8)];
    let exhaustive = (0u32..16)
        .filter_map(|mask| {
            let (mut weight, mut value) = (0, 0);
            for (i, (w, v)) in items.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    weight += w;
                    value += v;
                }
            }
            (weight <= 7).then_some(value)
        })
        .max()
        .unwrap();
    let (_, value) = knapsack(
        &items,
        7,
        GeneticConfig {
            population: 32,
            generations: 80,
            mutation: 0.1,
        },
        4,
    )
    .unwrap();
    assert_eq!(value, exhaustive as f64);
    let (_, loss) = evolve_real(2, (-5.0, 5.0), 300, 5, |x| {
        (x[0] - 2.0).powi(2) + (x[1] + 1.0).powi(2)
    })
    .unwrap();
    assert!(loss < 0.002, "loss={loss}");
}

#[test]
fn tree_encoding_and_genetic_programs_have_behavior() {
    let a = Expr::Add(Box::new(Expr::X), Box::new(Expr::Constant(2.0)));
    let b = Expr::Multiply(Box::new(Expr::X), Box::new(Expr::X));
    let mut child = tree_crossover(&a, &b, &[true], &[]).unwrap();
    assert_eq!(child.evaluate(3.0), 12.0);
    child.change_node(&[]).unwrap();
    assert_eq!(child.evaluate(3.0), 27.0);
    let data: Vec<_> = (-3..=3).map(|x| (x as f64, (x * x + x) as f64)).collect();
    let (program, loss) = genetic_programming(&data, 15, 7).unwrap();
    assert!(loss < 1.0, "loss={loss}, program={program:?}");
}

#[test]
fn ants_find_square_tour_and_particles_find_booth_minimum() {
    let d = vec![
        vec![0.0, 1.0, 2.0, 1.0],
        vec![1.0, 0.0, 1.0, 2.0],
        vec![2.0, 1.0, 0.0, 1.0],
        vec![1.0, 2.0, 1.0, 0.0],
    ];
    let (mut tour, length) = ant_colony(
        &d,
        AntConfig {
            ants: 12,
            iterations: 30,
            evaporation: 0.3,
            alpha: 1.0,
            beta: 2.0,
        },
        5,
    )
    .unwrap();
    assert_eq!(length, 4.0);
    tour.sort_unstable();
    assert_eq!(tour, [0, 1, 2, 3]);
    let (position, loss) = particle_swarm(
        &[(-10.0, 10.0); 2],
        ParticleConfig {
            particles: 24,
            iterations: 160,
            inertia: 0.6,
            cognitive: 1.5,
            social: 1.5,
        },
        4,
        |x| booth(x[0], x[1]),
    )
    .unwrap();
    assert!(loss < 1e-8, "{position:?} {loss}");
    assert!((position[0] - 1.0).abs() < 1e-3);
    assert!((position[1] - 3.0).abs() < 1e-3);
}

#[test]
fn regression_variants_and_classifiers_learn_distinct_targets() {
    let coefficients =
        polynomial_regression(&[-2.0, -1.0, 0.0, 1.0, 2.0], &[9.0, 2.0, 1.0, 6.0, 17.0], 2)
            .unwrap();
    for (a, b) in coefficients.iter().zip([1.0, 2.0, 3.0]) {
        assert!((a - b).abs() < 1e-10);
    }
    let x = vec![
        vec![-2.0, 0.0],
        vec![-1.0, 0.0],
        vec![1.0, 0.0],
        vec![2.0, 0.0],
    ];
    let logistic = LinearModel::fit(
        &x,
        &[0.0, 0.0, 1.0, 1.0],
        LinearObjective::Logistic,
        500,
        0.1,
    )
    .unwrap();
    assert!(logistic.score(&[-1.5, 0.0]).unwrap() < 0.0);
    assert!(logistic.score(&[1.5, 0.0]).unwrap() > 0.0);
    let svm = LinearModel::fit(
        &x,
        &[-1.0, -1.0, 1.0, 1.0],
        LinearObjective::Hinge(0.01),
        300,
        0.05,
    )
    .unwrap();
    assert!(svm.score(&[1.5, 0.0]).unwrap() > 1.0);
    let lasso = LinearModel::fit(
        &x,
        &[-4.0, -2.0, 2.0, 4.0],
        LinearObjective::Lasso(0.1),
        500,
        0.05,
    )
    .unwrap();
    assert_eq!(lasso.weights[1], 0.0);
    assert!((lasso.weights[0] - 2.0).abs() < 0.1);
    assert_eq!(
        knn(&x, &[0.0, 0.0, 1.0, 1.0], &[1.9, 0.0], 1, true).unwrap(),
        1.0
    );
    assert_eq!(
        knn(&x, &[0.0, 0.0, 1.0, 3.0], &[1.9, 0.0], 2, false).unwrap(),
        2.0
    );
    let nb = GaussianNaiveBayes::fit(&x, &[0, 0, 1, 1]).unwrap();
    assert_eq!(nb.predict(&[-1.5, 0.0]).unwrap(), 0);
    assert_eq!(nb.predict(&[1.5, 0.0]).unwrap(), 1);
}

#[test]
fn recursive_tree_learns_xor_and_prunes_on_validation() {
    let x = vec![
        vec![0.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 0.0],
        vec![1.0, 1.0],
    ];
    let y = [0, 1, 1, 0];
    for kind in [Impurity::Gini, Impurity::Entropy] {
        let mut tree = DecisionTree::fit(&x, &y, 3, kind).unwrap();
        for (row, label) in x.iter().zip(y) {
            assert_eq!(tree.predict(row).unwrap(), label);
        }
        tree.prune(&x, &[1, 1, 1, 1]).unwrap();
        assert!(matches!(tree.root, TreeNode::Leaf(_)));
    }
    assert!((impurity(&[0, 0, 1, 1], Impurity::Gini) - 0.5).abs() < 1e-12);
    assert!((impurity(&[0, 0, 1, 1], Impurity::Entropy) - 1.0).abs() < 1e-12);
}

#[test]
fn boosting_learns_missing_value_routing() {
    let x = vec![
        vec![0.0],
        vec![1.0],
        vec![2.0],
        vec![3.0],
        vec![f64::NAN],
        vec![f64::NAN],
    ];
    let y = [0.0, 0.0, 5.0, 5.0, 5.0, 5.0];
    let model = XgBoost::fit(
        &x,
        &y,
        BoostConfig {
            rounds: 60,
            depth: 2,
            rate: 0.2,
            lambda: 0.1,
            gamma: 0.0,
        },
    )
    .unwrap();
    for (row, target) in x.iter().zip(y) {
        assert!((model.predict(row).unwrap() - target).abs() < 0.05);
    }
}

#[test]
fn preprocessing_and_metrics_have_known_answers() {
    let missing = [Some(1.0), Some(2.0), Some(2.0), Some(7.0), None];
    assert_eq!(impute(&missing, Imputation::Mean).unwrap()[4], 3.0);
    assert_eq!(impute(&missing, Imputation::Median).unwrap()[4], 2.0);
    assert_eq!(impute(&missing, Imputation::Mode).unwrap()[4], 2.0);
    assert_eq!(label_encode(&["b", "a", "b"]).1, [1, 0, 1]);
    assert_eq!(one_hot(1, 3).unwrap(), [0.0, 1.0, 0.0]);
    assert_eq!(min_max_scale(&[1.0, 3.0, 5.0]).unwrap(), [0.0, 0.5, 1.0]);
    let m = binary_metrics(&[true, true, false, false], &[true, false, false, false]).unwrap();
    assert_eq!(m.precision, Some(1.0));
    assert_eq!(m.recall, Some(0.5));
    assert_eq!(m.accuracy, 0.75);
}

#[test]
fn clustering_algorithms_recover_two_groups_without_label_assumptions() {
    let x = vec![
        vec![0.0],
        vec![0.1],
        vec![0.2],
        vec![5.0],
        vec![5.1],
        vec![5.2],
    ];
    let (_, k) = k_means(&x, 2, 20, 3).unwrap();
    let (_, m) = mean_shift(&x, 0.5, 20).unwrap();
    let a = agglomerative(&x, 2).unwrap();
    for labels in [k, m, a] {
        assert_eq!(labels[0], labels[2]);
        assert_eq!(labels[3], labels[5]);
        assert_ne!(labels[0], labels[3]);
    }
    let mut with_noise = x;
    with_noise.push(vec![100.0]);
    let d = dbscan(&with_noise, 0.3, 2).unwrap();
    assert_eq!(d[6], None);
    assert_eq!(d[0], d[2]);
    assert_ne!(d[0], d[3]);
}

#[test]
fn pca_recovers_a_line_and_eigenvectors_are_orthogonal() {
    let x = vec![
        vec![-2.0, -4.0],
        vec![-1.0, -2.0],
        vec![1.0, 2.0],
        vec![2.0, 4.0],
    ];
    let pca = Pca::fit(&x, 2).unwrap();
    assert!(pca.eigenvalues[1].abs() < 1e-10);
    assert!(pca.transform(&[3.0, 6.0]).unwrap()[1].abs() < 1e-10);
    let (_, v) = symmetric_eigen(&[vec![2.0, 1.0], vec![1.0, 2.0]]).unwrap();
    assert!((v[0][0] * v[0][1] + v[1][0] * v[1][1]).abs() < 1e-10);
}

#[test]
fn tsne_gradient_matches_finite_difference_and_training_reduces_kl() {
    let x = vec![vec![0.0], vec![0.1], vec![5.0], vec![5.1]];
    let p = tsne_probabilities(&x, 1.5).unwrap();
    let mut y = vec![
        vec![0.1, 0.2],
        vec![-0.3, 0.2],
        vec![0.4, -0.1],
        vec![0.0, 0.1],
    ];
    let (_, gradient) = tsne_loss_gradient(&p, &y).unwrap();
    let h = 1e-6;
    y[0][0] += h;
    let plus = tsne_loss_gradient(&p, &y).unwrap().0;
    y[0][0] -= 2.0 * h;
    let minus = tsne_loss_gradient(&p, &y).unwrap().0;
    assert!(((plus - minus) / (2.0 * h) - gradient[0][0]).abs() < 1e-6);
    let start = tsne(&x, 1.5, 0, 1.0, 3).unwrap();
    let end = tsne(&x, 1.5, 300, 1.0, 3).unwrap();
    assert!(
        tsne_loss_gradient(&p, &end).unwrap().0 < tsne_loss_gradient(&p, &start).unwrap().0 * 0.5
    );
}
