//! Small hyperparameter searches. The supplied objective is a validation loss.
use crate::{
    Result,
    numeric::{distance2, dot, matrix_width, solve_linear},
    random::Rng,
};

/// Enumerate the Cartesian product of discrete hyperparameter choices.
pub fn cartesian_grid(axes: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    if axes.is_empty()
        || axes
            .iter()
            .any(|a| a.is_empty() || a.iter().any(|v| !v.is_finite()))
    {
        return Err("invalid grid axes");
    }
    let mut grid = vec![Vec::new()];
    for axis in axes {
        grid = grid
            .into_iter()
            .flat_map(|row| {
                axis.iter().map(move |&v| {
                    let mut next = row.clone();
                    next.push(v);
                    next
                })
            })
            .collect();
    }
    Ok(grid)
}

pub fn random_search(
    bounds: &[(f64, f64)],
    trials: usize,
    seed: u64,
    mut objective: impl FnMut(&[f64]) -> Result<f64>,
) -> Result<(Vec<f64>, f64)> {
    if bounds.is_empty()
        || trials == 0
        || bounds
            .iter()
            .any(|(a, b)| !a.is_finite() || !b.is_finite() || a >= b || !(b - a).is_finite())
    {
        return Err("invalid random search bounds/budget");
    }
    let mut rng = Rng::new(seed);
    let mut best: Option<(Vec<f64>, f64)> = None;
    for _ in 0..trials {
        let candidate: Vec<_> = bounds
            .iter()
            .map(|(a, b)| a + rng.uniform() * (b - a))
            .collect();
        let score = objective(&candidate)?;
        if !score.is_finite() {
            return Err("nonfinite validation loss");
        }
        if best.as_ref().is_none_or(|v| score < v.1) {
            best = Some((candidate, score));
        }
    }
    best.ok_or("empty search")
}

/// Exact Gaussian process with a fixed squared-exponential kernel and Gaussian noise.
pub struct GaussianProcess {
    inputs: Vec<Vec<f64>>,
    covariance: Vec<Vec<f64>>,
    alpha: Vec<f64>,
    mean: f64,
    length_scale: f64,
}
impl GaussianProcess {
    pub fn fit(x: &[Vec<f64>], y: &[f64], length_scale: f64, noise: f64) -> Result<Self> {
        matrix_width(x)?;
        if x.len() != y.len()
            || y.iter().any(|v| !v.is_finite())
            || !length_scale.is_finite()
            || length_scale <= 0.0
            || !noise.is_finite()
            || noise <= 0.0
        {
            return Err("invalid Gaussian process data");
        }
        let covariance: Vec<Vec<_>> = x
            .iter()
            .enumerate()
            .map(|(i, a)| {
                x.iter()
                    .enumerate()
                    .map(|(j, b)| kernel(a, b, length_scale) + if i == j { noise } else { 0.0 })
                    .collect()
            })
            .collect();
        let mean = y.iter().map(|v| v / y.len() as f64).sum();
        let alpha = solve_linear(&covariance, &y.iter().map(|v| v - mean).collect::<Vec<_>>())?;
        Ok(Self {
            inputs: x.to_vec(),
            covariance,
            alpha,
            mean,
            length_scale,
        })
    }
    /// Posterior mean and latent variance (observation noise is not added).
    pub fn predict(&self, query: &[f64]) -> Result<(f64, f64)> {
        if query.len() != self.inputs[0].len() || query.iter().any(|v| !v.is_finite()) {
            return Err("invalid GP query");
        }
        let k: Vec<_> = self
            .inputs
            .iter()
            .map(|x| kernel(x, query, self.length_scale))
            .collect();
        let projected = solve_linear(&self.covariance, &k)?;
        Ok((
            self.mean + dot(&k, &self.alpha),
            (1.0 - dot(&k, &projected)).max(0.0),
        ))
    }
}
fn kernel(a: &[f64], b: &[f64], scale: f64) -> f64 {
    (-0.5 * distance2(a, b) / scale.powi(2)).exp()
}

/// Bayesian optimization on a finite candidate set using a GP lower confidence bound.
/// Minimize mean - 2 * posterior_std, evaluate, then refit the GP. No point repeats.
pub fn bayesian_search(
    candidates: &[Vec<f64>],
    budget: usize,
    seed: u64,
    mut objective: impl FnMut(&[f64]) -> Result<f64>,
) -> Result<(Vec<f64>, f64)> {
    matrix_width(candidates)?;
    if budget == 0 {
        return Err("zero evaluation budget");
    }
    let mut rng = Rng::new(seed);
    let mut observed = Vec::new();
    let mut scores = Vec::new();
    let mut evaluated = vec![false; candidates.len()];
    for _ in 0..budget.min(candidates.len()) {
        let index = if observed.is_empty() {
            rng.index(candidates.len()).ok_or("empty candidate set")?
        } else {
            let gp = GaussianProcess::fit(&observed, &scores, 1.0, 1e-6)?;
            let mut best: Option<(usize, f64)> = None;
            for (i, candidate) in candidates
                .iter()
                .enumerate()
                .filter(|(i, _)| !evaluated[*i])
            {
                let (mean, variance) = gp.predict(candidate)?;
                let acquisition = mean - 2.0 * variance.sqrt();
                if best.is_none_or(|v| acquisition < v.1) {
                    best = Some((i, acquisition));
                }
            }
            best.ok_or("no remaining candidates")?.0
        };
        let score = objective(&candidates[index])?;
        if !score.is_finite() {
            return Err("nonfinite validation loss");
        }
        evaluated[index] = true;
        observed.push(candidates[index].clone());
        scores.push(score);
    }
    let index = (0..scores.len())
        .min_by(|&a, &b| scores[a].total_cmp(&scores[b]))
        .ok_or("empty search")?;
    Ok((observed[index].clone(), scores[index]))
}
