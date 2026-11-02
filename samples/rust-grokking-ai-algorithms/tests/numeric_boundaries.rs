use grokking_ai_algorithms::{clustering::tsne_loss_gradient, learning::*};

#[test]
fn adjacent_floats_can_be_separated_by_a_stump() {
    let a = f64::from_bits(1.0_f64.to_bits() + 1);
    let b = f64::from_bits(1.0_f64.to_bits() + 2);
    let stump = DecisionStump::fit(&[(a, false), (b, true)]).unwrap();
    assert!(!stump.predict(a));
    assert!(stump.predict(b));
}

#[test]
fn numeric_overflow_is_not_reported_as_a_valid_fit_or_update() {
    assert!(linear_regression(&[(f64::MAX, f64::MAX), (-f64::MAX, 0.0)]).is_none());
    assert!(q_learning_update(0.0, f64::MAX, &[f64::MAX], 1.0, 1.0).is_none());
}

#[test]
fn tsne_rejects_probabilities_that_violate_gradient_assumptions() {
    let y = vec![vec![0.0], vec![1.0]];
    assert!(tsne_loss_gradient(&[vec![0.0, 0.2], vec![0.8, 0.0]], &y).is_err());
    assert!(tsne_loss_gradient(&[vec![0.0, 1.0], vec![1.0, 0.0]], &y).is_err());
    assert!(tsne_loss_gradient(&[vec![0.1, 0.4], vec![0.4, 0.1]], &y).is_err());
}
