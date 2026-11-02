//! Chapter 8, including algorithms named in figure 8.30.
use crate::{
    Result,
    numeric::{distance2, dot, matrix_width, sigmoid, solve_linear},
    random::Rng,
};
use std::collections::BTreeMap;

pub fn polynomial_regression(x: &[f64], y: &[f64], degree: usize) -> Result<Vec<f64>> {
    if degree > 32
        || x.len() != y.len()
        || x.len() <= degree
        || x.iter().chain(y).any(|v| !v.is_finite())
    {
        return Err("invalid regression data");
    }
    let rows: Vec<Vec<_>> = x
        .iter()
        .map(|v| (0..=degree).map(|p| v.powi(p as i32)).collect())
        .collect();
    let matrix: Vec<Vec<_>> = (0..=degree)
        .map(|i| {
            (0..=degree)
                .map(|j| rows.iter().map(|r| r[i] * r[j]).sum())
                .collect()
        })
        .collect();
    let rhs: Vec<_> = (0..=degree)
        .map(|i| rows.iter().zip(y).map(|(r, v)| r[i] * v).sum())
        .collect();
    solve_linear(&matrix, &rhs)
}

#[derive(Clone, Debug)]
pub struct LinearModel {
    pub weights: Vec<f64>,
    pub bias: f64,
}
#[derive(Clone, Copy)]
pub enum LinearObjective {
    LeastSquares,
    Lasso(f64),
    Logistic,
    Hinge(f64),
}
impl LinearModel {
    pub fn score(&self, x: &[f64]) -> Result<f64> {
        if x.len() != self.weights.len() || x.iter().any(|v| !v.is_finite()) {
            return Err("invalid feature vector");
        }
        Ok(dot(&self.weights, x) + self.bias)
    }
    pub fn fit(
        x: &[Vec<f64>],
        y: &[f64],
        objective: LinearObjective,
        epochs: usize,
        rate: f64,
    ) -> Result<Self> {
        let d = matrix_width(x)?;
        if y.len() != x.len()
            || y.iter().any(|v| !v.is_finite())
            || !rate.is_finite()
            || rate <= 0.0
        {
            return Err("invalid targets or rate");
        }
        match objective {
            LinearObjective::Logistic if y.iter().any(|v| ![0.0, 1.0].contains(v)) => {
                return Err("binary targets required");
            }
            LinearObjective::Hinge(l)
                if !l.is_finite() || l < 0.0 || y.iter().any(|v| ![-1.0, 1.0].contains(v)) =>
            {
                return Err("hinge requires +/-1 labels and nonnegative regularization");
            }
            LinearObjective::Lasso(l) if !l.is_finite() || l < 0.0 => {
                return Err("invalid regularization");
            }
            _ => (),
        }
        let mut model = Self {
            weights: vec![0.0; d],
            bias: 0.0,
        };
        for _ in 0..epochs {
            let mut gradient = vec![0.0; d];
            let mut bias = 0.0;
            for (row, &target) in x.iter().zip(y) {
                let score = dot(&model.weights, row) + model.bias;
                let residual = match objective {
                    LinearObjective::Logistic => sigmoid(score) - target,
                    LinearObjective::Hinge(_) => {
                        if target * score < 1.0 {
                            -target
                        } else {
                            0.0
                        }
                    }
                    _ => score - target,
                };
                for (g, v) in gradient.iter_mut().zip(row) {
                    *g += residual * v / x.len() as f64;
                }
                bias += residual / x.len() as f64;
            }
            for (w, g) in model.weights.iter_mut().zip(gradient) {
                let penalty = if let LinearObjective::Hinge(lambda) = objective {
                    lambda * *w
                } else {
                    0.0
                };
                *w -= rate * (g + penalty);
                if let LinearObjective::Lasso(lambda) = objective {
                    *w = w.signum() * (w.abs() - rate * lambda).max(0.0);
                }
            }
            model.bias -= rate * bias;
            if !model.bias.is_finite() || model.weights.iter().any(|w| !w.is_finite()) {
                return Err("training diverged");
            }
        }
        Ok(model)
    }
}

pub fn knn(
    x: &[Vec<f64>],
    y: &[f64],
    query: &[f64],
    k: usize,
    classification: bool,
) -> Result<f64> {
    let d = matrix_width(x)?;
    if query.len() != d
        || query.iter().any(|v| !v.is_finite())
        || y.len() != x.len()
        || y.iter().any(|v| !v.is_finite())
        || k == 0
        || k > x.len()
    {
        return Err("invalid KNN input");
    }
    let mut order: Vec<_> = (0..x.len()).collect();
    order.sort_by(|&a, &b| distance2(&x[a], query).total_cmp(&distance2(&x[b], query)));
    if !classification {
        return Ok(order[..k].iter().map(|&i| y[i]).sum::<f64>() / k as f64);
    }
    let mut labels: Vec<_> = order[..k].iter().map(|&i| y[i]).collect();
    labels.sort_by(f64::total_cmp);
    Ok(*labels
        .iter()
        .max_by_key(|&&v| labels.iter().filter(|&&u| u == v).count())
        .expect("positive k"))
}

#[derive(Clone, Copy)]
pub enum Impurity {
    Gini,
    Entropy,
}
pub fn impurity(labels: &[usize], kind: Impurity) -> f64 {
    let mut counts = BTreeMap::new();
    for &label in labels {
        *counts.entry(label).or_insert(0usize) += 1;
    }
    if labels.is_empty() {
        return 0.0;
    }
    let p = counts.values().map(|&n| n as f64 / labels.len() as f64);
    match kind {
        Impurity::Gini => 1.0 - p.map(|v| v * v).sum::<f64>(),
        Impurity::Entropy => -p.map(|v| v * v.log2()).sum::<f64>(),
    }
}
#[derive(Clone, Debug)]
pub enum TreeNode {
    Leaf(usize),
    Split {
        feature: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
        majority: usize,
    },
}
#[derive(Clone, Debug)]
pub struct DecisionTree {
    pub root: TreeNode,
    width: usize,
}
fn majority(labels: &[usize]) -> usize {
    let mut counts = BTreeMap::new();
    for &label in labels {
        *counts.entry(label).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map_or(0, |(label, _)| label)
}
impl TreeNode {
    fn predict(&self, x: &[f64]) -> usize {
        match self {
            Self::Leaf(c) => *c,
            Self::Split {
                feature,
                threshold,
                left,
                right,
                ..
            } => if x[*feature] <= *threshold {
                left
            } else {
                right
            }
            .predict(x),
        }
    }
}
impl DecisionTree {
    pub fn fit(x: &[Vec<f64>], y: &[usize], max_depth: usize, kind: Impurity) -> Result<Self> {
        Self::fit_with_features(x, y, max_depth, kind, &mut |width| (0..width).collect())
    }
    fn fit_with_features(
        x: &[Vec<f64>],
        y: &[usize],
        max_depth: usize,
        kind: Impurity,
        features: &mut impl FnMut(usize) -> Vec<usize>,
    ) -> Result<Self> {
        let width = matrix_width(x)?;
        if y.len() != x.len() || max_depth > 64 {
            return Err("invalid labels/depth");
        }
        fn build(
            x: &[Vec<f64>],
            y: &[usize],
            indices: &[usize],
            depth: usize,
            kind: Impurity,
            features: &mut impl FnMut(usize) -> Vec<usize>,
        ) -> TreeNode {
            let labels: Vec<_> = indices.iter().map(|&i| y[i]).collect();
            let common = majority(&labels);
            if depth == 0 || labels.iter().all(|v| *v == labels[0]) {
                return TreeNode::Leaf(common);
            }
            struct Split {
                score: f64,
                feature: usize,
                threshold: f64,
                left: Vec<usize>,
                right: Vec<usize>,
            }
            let mut best: Option<Split> = None;
            for f in features(x[0].len()) {
                let mut values: Vec<_> = indices.iter().map(|&i| x[i][f]).collect();
                values.sort_by(f64::total_cmp);
                values.dedup();
                for &threshold in values.iter().take(values.len() - 1) {
                    let (l, r): (Vec<_>, Vec<_>) =
                        indices.iter().copied().partition(|&i| x[i][f] <= threshold);
                    let ly: Vec<_> = l.iter().map(|&i| y[i]).collect();
                    let ry: Vec<_> = r.iter().map(|&i| y[i]).collect();
                    let score = (l.len() as f64 * impurity(&ly, kind)
                        + r.len() as f64 * impurity(&ry, kind))
                        / indices.len() as f64;
                    if best.as_ref().is_none_or(|v| score < v.score) {
                        best = Some(Split {
                            score,
                            feature: f,
                            threshold,
                            left: l,
                            right: r,
                        });
                    }
                }
            }
            // Allow equal impurity at a split: XOR becomes separable at the next level.
            if let Some(Split {
                feature,
                threshold,
                left: l,
                right: r,
                ..
            }) = best
            {
                TreeNode::Split {
                    feature,
                    threshold,
                    left: Box::new(build(x, y, &l, depth - 1, kind, features)),
                    right: Box::new(build(x, y, &r, depth - 1, kind, features)),
                    majority: common,
                }
            } else {
                TreeNode::Leaf(common)
            }
        }
        Ok(Self {
            root: build(
                x,
                y,
                &(0..x.len()).collect::<Vec<_>>(),
                max_depth,
                kind,
                features,
            ),
            width,
        })
    }
    pub fn predict(&self, x: &[f64]) -> Result<usize> {
        if x.len() != self.width || x.iter().any(|v| !v.is_finite()) {
            return Err("invalid features");
        }
        Ok(self.root.predict(x))
    }
    /// Reduced-error pruning on a held-out validation set.
    pub fn prune(&mut self, x: &[Vec<f64>], y: &[usize]) -> Result<()> {
        if matrix_width(x)? != self.width || x.len() != y.len() {
            return Err("invalid validation set");
        }
        fn prune_node(node: &mut TreeNode, x: &[Vec<f64>], y: &[usize], ids: &[usize]) {
            if ids.is_empty() {
                return;
            }
            if let TreeNode::Split {
                feature,
                threshold,
                left,
                right,
                majority,
            } = node
            {
                let (a, b): (Vec<_>, Vec<_>) = ids
                    .iter()
                    .copied()
                    .partition(|&i| x[i][*feature] <= *threshold);
                prune_node(left, x, y, &a);
                prune_node(right, x, y, &b);
                let label = *majority;
                let leaf_errors = ids.iter().filter(|&&i| y[i] != label).count();
                let tree_errors = ids.iter().filter(|&&i| node.predict(&x[i]) != y[i]).count();
                if leaf_errors <= tree_errors {
                    *node = TreeNode::Leaf(label);
                }
            }
        }
        prune_node(&mut self.root, x, y, &(0..x.len()).collect::<Vec<_>>());
        Ok(())
    }
}

/// Bootstrap aggregation and a newly sampled feature subset at every split.
pub struct RandomForest {
    pub trees: Vec<DecisionTree>,
}
impl RandomForest {
    pub fn fit(
        x: &[Vec<f64>],
        y: &[usize],
        trees: usize,
        depth: usize,
        features_per_split: usize,
        seed: u64,
    ) -> Result<Self> {
        let width = matrix_width(x)?;
        if y.len() != x.len() || trees == 0 || features_per_split == 0 || features_per_split > width
        {
            return Err("invalid forest configuration");
        }
        let mut rng = Rng::new(seed);
        let mut forest = Vec::new();
        for _ in 0..trees {
            let indices: Vec<_> = (0..x.len())
                .map(|_| rng.index(x.len()).expect("nonempty training set"))
                .collect();
            let rows: Vec<_> = indices.iter().map(|&i| x[i].clone()).collect();
            let labels: Vec<_> = indices.iter().map(|&i| y[i]).collect();
            let tree =
                DecisionTree::fit_with_features(&rows, &labels, depth, Impurity::Gini, &mut |d| {
                    let mut features: Vec<_> = (0..d).collect();
                    rng.shuffle(&mut features);
                    features.truncate(features_per_split);
                    features
                })?;
            forest.push(tree);
        }
        Ok(Self { trees: forest })
    }
    pub fn predict(&self, x: &[f64]) -> Result<usize> {
        if self.trees.is_empty() {
            return Err("empty forest");
        }
        let votes: Result<Vec<_>> = self.trees.iter().map(|tree| tree.predict(x)).collect();
        Ok(majority(&votes?))
    }
}

pub struct GaussianNaiveBayes {
    classes: Vec<usize>,
    means: Vec<Vec<f64>>,
    variances: Vec<Vec<f64>>,
    priors: Vec<f64>,
}
impl GaussianNaiveBayes {
    pub fn fit(x: &[Vec<f64>], y: &[usize]) -> Result<Self> {
        let d = matrix_width(x)?;
        if x.len() != y.len() {
            return Err("invalid labels");
        }
        let mut classes = y.to_vec();
        classes.sort_unstable();
        classes.dedup();
        let mut means = Vec::new();
        let mut variances = Vec::new();
        let mut priors = Vec::new();
        for &label in &classes {
            let rows: Vec<_> = x
                .iter()
                .zip(y)
                .filter(|(_, c)| **c == label)
                .map(|(r, _)| r)
                .collect();
            let mean: Vec<_> = (0..d)
                .map(|j| rows.iter().map(|r| r[j]).sum::<f64>() / rows.len() as f64)
                .collect();
            let var: Vec<_> = (0..d)
                .map(|j| {
                    (rows.iter().map(|r| (r[j] - mean[j]).powi(2)).sum::<f64>() / rows.len() as f64)
                        .max(1e-9)
                })
                .collect();
            means.push(mean);
            variances.push(var);
            priors.push(rows.len() as f64 / x.len() as f64);
        }
        Ok(Self {
            classes,
            means,
            variances,
            priors,
        })
    }
    pub fn predict(&self, x: &[f64]) -> Result<usize> {
        if x.len() != self.means[0].len() || x.iter().any(|v| !v.is_finite()) {
            return Err("invalid features");
        }
        let scores: Vec<_> = (0..self.classes.len())
            .map(|i| {
                self.priors[i].ln()
                    - 0.5
                        * x.iter()
                            .enumerate()
                            .map(|(j, v)| {
                                (std::f64::consts::TAU * self.variances[i][j]).ln()
                                    + (v - self.means[i][j]).powi(2) / self.variances[i][j]
                            })
                            .sum::<f64>()
            })
            .collect();
        Ok(self.classes[(0..scores.len())
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .expect("nonempty")])
    }
}

#[derive(Clone, Copy)]
pub enum Imputation {
    Mean,
    Median,
    Mode,
}
pub fn impute(values: &[Option<f64>], strategy: Imputation) -> Result<Vec<f64>> {
    let mut known: Vec<_> = values.iter().flatten().copied().collect();
    if known.is_empty() || known.iter().any(|v| !v.is_finite()) {
        return Err("no finite observations");
    }
    known.sort_by(f64::total_cmp);
    let fill = match strategy {
        Imputation::Mean => known.iter().sum::<f64>() / known.len() as f64,
        Imputation::Median => known[(known.len() - 1) / 2] / 2.0 + known[known.len() / 2] / 2.0,
        Imputation::Mode => *known
            .iter()
            .max_by_key(|&&v| known.iter().filter(|&&u| u == v).count())
            .expect("nonempty"),
    };
    Ok(values.iter().map(|v| v.unwrap_or(fill)).collect())
}
/// KNN imputation from the originally observed entries, independent of fill order.
/// Distances use shared observed features, scaled by total/shared feature count.
/// With no comparable donor, use the observed column mean; empty columns error.
pub fn knn_impute(rows: &[Vec<Option<f64>>], k: usize) -> Result<Vec<Vec<f64>>> {
    let width = rows.first().map_or(0, Vec::len);
    if width == 0
        || k == 0
        || rows
            .iter()
            .any(|r| r.len() != width || r.iter().flatten().any(|v| !v.is_finite()))
    {
        return Err("invalid imputation table");
    }
    let mut result = Vec::new();
    for row in rows {
        let mut filled = Vec::new();
        for column in 0..width {
            if let Some(value) = row[column] {
                filled.push(value);
                continue;
            }
            let mut donors = Vec::new();
            let mut known = Vec::new();
            for candidate in rows {
                if let Some(value) = candidate[column] {
                    known.push(value);
                    let pairs: Vec<_> = row
                        .iter()
                        .zip(candidate)
                        .filter_map(|(a, b)| a.zip(*b))
                        .collect();
                    if !pairs.is_empty() {
                        let distance = pairs.iter().map(|(a, b)| (a - b).powi(2)).sum::<f64>()
                            * width as f64
                            / pairs.len() as f64;
                        donors.push((distance, value));
                    }
                }
            }
            if known.is_empty() {
                return Err("column has no observed entries");
            }
            donors.sort_by(|a, b| a.0.total_cmp(&b.0));
            let value = if donors.is_empty() {
                known.iter().map(|v| v / known.len() as f64).sum()
            } else {
                let count = k.min(donors.len());
                donors[..count].iter().map(|(_, v)| v / count as f64).sum()
            };
            filled.push(value);
        }
        result.push(filled);
    }
    Ok(result)
}
pub fn label_encode(values: &[&str]) -> (Vec<String>, Vec<usize>) {
    let mut labels: Vec<_> = values.iter().map(|v| v.to_string()).collect();
    labels.sort();
    labels.dedup();
    let encoded = values
        .iter()
        .map(|v| {
            labels
                .binary_search_by(|s| s.as_str().cmp(v))
                .expect("known label")
        })
        .collect();
    (labels, encoded)
}
pub fn one_hot(label: usize, classes: usize) -> Result<Vec<f64>> {
    if label >= classes {
        return Err("label outside classes");
    }
    let mut row = vec![0.0; classes];
    row[label] = 1.0;
    Ok(row)
}
pub fn min_max_scale(values: &[f64]) -> Result<Vec<f64>> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err("invalid values");
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok(values
        .iter()
        .map(|v| {
            if min == max {
                0.0
            } else {
                (v - min) / (max - min)
            }
        })
        .collect())
}
#[derive(Debug, PartialEq)]
pub struct BinaryMetrics {
    pub accuracy: f64,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub specificity: Option<f64>,
    pub npv: Option<f64>,
}
pub fn binary_metrics(actual: &[bool], predicted: &[bool]) -> Result<BinaryMetrics> {
    if actual.is_empty() || actual.len() != predicted.len() {
        return Err("invalid observations");
    }
    let (mut tp, mut tn, mut fp, mut fn_) = (0.0, 0.0, 0.0, 0.0);
    for (&a, &p) in actual.iter().zip(predicted) {
        match (a, p) {
            (true, true) => tp += 1.0,
            (false, false) => tn += 1.0,
            (false, true) => fp += 1.0,
            (true, false) => fn_ += 1.0,
        }
    }
    let ratio = |n: f64, d: f64| if d == 0.0 { None } else { Some(n / d) };
    Ok(BinaryMetrics {
        accuracy: (tp + tn) / actual.len() as f64,
        precision: ratio(tp, tp + fp),
        recall: ratio(tp, tp + fn_),
        specificity: ratio(tn, tn + fp),
        npv: ratio(tn, tn + fn_),
    })
}
