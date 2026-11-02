/// Fit `y = slope * x + intercept` with the least-squares method.
pub fn linear_regression(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if points.len() < 2 || points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }
    let count = points.len() as f64;
    let mean_x = points.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = points.iter().map(|(_, y)| y).sum::<f64>() / count;
    let covariance = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let variance = points
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    if variance == 0.0 {
        return None;
    }
    let slope = covariance / variance;
    let intercept = mean_y - slope * mean_x;
    (slope.is_finite() && intercept.is_finite()).then_some((slope, intercept))
}

/// One tabular Q-learning update for a single state/action transition.
pub fn q_learning_update(
    current: f64,
    reward: f64,
    next_action_values: &[f64],
    learning_rate: f64,
    discount: f64,
) -> Option<f64> {
    if !current.is_finite()
        || !reward.is_finite()
        || next_action_values.iter().any(|value| !value.is_finite())
        || !(0.0..=1.0).contains(&learning_rate)
        || !(0.0..=1.0).contains(&discount)
    {
        return None;
    }
    let next_best = next_action_values
        .iter()
        .copied()
        .reduce(f64::max)
        .unwrap_or(0.0);
    let updated = current + learning_rate * (reward + discount * next_best - current);
    updated.is_finite().then_some(updated)
}

/// A decision stump that classifies a numeric feature using a threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionStump {
    pub threshold: f64,
    pub below: bool,
    pub above: bool,
}

impl DecisionStump {
    pub fn predict(&self, value: f64) -> bool {
        if value <= self.threshold {
            self.below
        } else {
            self.above
        }
    }

    pub fn fit(samples: &[(f64, bool)]) -> Option<Self> {
        if samples.is_empty() || samples.iter().any(|(value, _)| !value.is_finite()) {
            return None;
        }
        let mut values: Vec<f64> = samples.iter().map(|(value, _)| *value).collect();
        values.sort_by(f64::total_cmp);
        values.dedup_by(|left, right| left.total_cmp(right).is_eq());
        let thresholds = if values.len() == 1 {
            vec![values[0]]
        } else {
            values.windows(2).map(|pair| pair[0]).collect()
        };
        let mut best: Option<(usize, Self)> = None;
        for threshold in thresholds {
            for below in [false, true] {
                for above in [false, true] {
                    let stump = Self {
                        threshold,
                        below,
                        above,
                    };
                    let errors = samples
                        .iter()
                        .filter(|(value, label)| stump.predict(*value) != *label)
                        .count();
                    if best
                        .as_ref()
                        .is_none_or(|(best_errors, _)| errors < *best_errors)
                    {
                        best = Some((errors, stump));
                    }
                }
            }
        }
        best.map(|(_, stump)| stump)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_fits_a_line_and_rejects_degenerate_data() {
        let (slope, intercept) = linear_regression(&[(1.0, 3.0), (2.0, 5.0), (3.0, 7.0)]).unwrap();
        assert!((slope - 2.0).abs() < 1e-10);
        assert!((intercept - 1.0).abs() < 1e-10);
        assert_eq!(linear_regression(&[(2.0, 1.0), (2.0, 4.0)]), None);
    }

    #[test]
    fn q_learning_applies_reward_and_discounted_future_value() {
        assert_eq!(
            q_learning_update(2.0, 3.0, &[4.0, 5.0], 0.5, 0.8),
            Some(4.5)
        );
        assert_eq!(q_learning_update(1.0, 0.0, &[], 0.2, 0.9), Some(0.8));
        assert_eq!(q_learning_update(1.0, 0.0, &[], 2.0, 0.9), None);
    }

    #[test]
    fn decision_stump_finds_separating_threshold() {
        let samples = [(1.0, false), (2.0, false), (5.0, true), (8.0, true)];
        let stump = DecisionStump::fit(&samples).unwrap();
        assert!(
            samples
                .iter()
                .all(|(value, label)| stump.predict(*value) == *label)
        );
        assert_eq!(DecisionStump::fit(&[]), None);
    }
}
