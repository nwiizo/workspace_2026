//! Small exact-split, second-order gradient boosting with learned missing directions.
//! Implements the XGBoost objective, not the upstream library's distributed engine.
use crate::Result;
#[derive(Clone)]
enum Node {
    Leaf(f64),
    Split {
        feature: usize,
        threshold: f64,
        missing_left: bool,
        left: Box<Node>,
        right: Box<Node>,
    },
}
impl Node {
    fn predict(&self, x: &[f64]) -> f64 {
        match self {
            Self::Leaf(v) => *v,
            Self::Split {
                feature,
                threshold,
                missing_left,
                left,
                right,
            } => {
                let go_left = if x[*feature].is_nan() {
                    *missing_left
                } else {
                    x[*feature] <= *threshold
                };
                if go_left { left } else { right }.predict(x)
            }
        }
    }
}
pub struct XgBoost {
    trees: Vec<Node>,
    base: f64,
    rate: f64,
    width: usize,
}
#[derive(Clone, Copy)]
pub struct BoostConfig {
    pub rounds: usize,
    pub depth: usize,
    pub rate: f64,
    pub lambda: f64,
    pub gamma: f64,
}
impl XgBoost {
    pub fn fit(x: &[Vec<f64>], y: &[f64], config: BoostConfig) -> Result<Self> {
        let d = x.first().ok_or("empty data")?.len();
        if d == 0
            || x.len() != y.len()
            || x.iter()
                .any(|r| r.len() != d || r.iter().any(|v| v.is_infinite()))
            || y.iter().any(|v| !v.is_finite())
            || config.depth > 32
            || !config.rate.is_finite()
            || config.rate <= 0.0
            || !config.lambda.is_finite()
            || config.lambda < 0.0
            || !config.gamma.is_finite()
            || config.gamma < 0.0
        {
            return Err("invalid boosting input");
        }
        fn build(x: &[Vec<f64>], g: &[f64], ids: &[usize], depth: usize, c: BoostConfig) -> Node {
            let sum: f64 = ids.iter().map(|&i| g[i]).sum();
            let leaf = Node::Leaf(-sum / (ids.len() as f64 + c.lambda));
            if depth == 0 || ids.len() < 2 {
                return leaf;
            }
            struct Split {
                gain: f64,
                feature: usize,
                threshold: f64,
                missing_left: bool,
                left: Vec<usize>,
                right: Vec<usize>,
            }
            let mut best: Option<Split> = None;
            for (f, _) in x[0].iter().enumerate() {
                let mut values: Vec<_> = ids
                    .iter()
                    .map(|&i| x[i][f])
                    .filter(|v| !v.is_nan())
                    .collect();
                values.sort_by(f64::total_cmp);
                values.dedup();
                for threshold in values {
                    for missing_left in [false, true] {
                        let (left, right): (Vec<_>, Vec<_>) = ids.iter().copied().partition(|&i| {
                            if x[i][f].is_nan() {
                                missing_left
                            } else {
                                x[i][f] <= threshold
                            }
                        });
                        if left.is_empty() || right.is_empty() {
                            continue;
                        }
                        let lg: f64 = left.iter().map(|&i| g[i]).sum();
                        let rg = sum - lg;
                        let gain = 0.5
                            * (lg * lg / (left.len() as f64 + c.lambda)
                                + rg * rg / (right.len() as f64 + c.lambda)
                                - sum * sum / (ids.len() as f64 + c.lambda))
                            - c.gamma;
                        if gain > 0.0 && best.as_ref().is_none_or(|b| gain > b.gain) {
                            best = Some(Split {
                                gain,
                                feature: f,
                                threshold,
                                missing_left,
                                left,
                                right,
                            });
                        }
                    }
                }
            }
            if let Some(Split {
                feature,
                threshold,
                missing_left,
                left: l,
                right: r,
                ..
            }) = best
            {
                Node::Split {
                    feature,
                    threshold,
                    missing_left,
                    left: Box::new(build(x, g, &l, depth - 1, c)),
                    right: Box::new(build(x, g, &r, depth - 1, c)),
                }
            } else {
                leaf
            }
        }
        let base = y.iter().sum::<f64>() / y.len() as f64;
        let mut model = Self {
            trees: Vec::new(),
            base,
            rate: config.rate,
            width: d,
        };
        let mut predictions = vec![base; y.len()];
        for _ in 0..config.rounds {
            let gradient: Vec<_> = predictions.iter().zip(y).map(|(p, t)| p - t).collect();
            let tree = build(
                x,
                &gradient,
                &(0..x.len()).collect::<Vec<_>>(),
                config.depth,
                config,
            );
            for (p, row) in predictions.iter_mut().zip(x) {
                *p += config.rate * tree.predict(row);
            }
            model.trees.push(tree);
        }
        Ok(model)
    }
    pub fn predict(&self, x: &[f64]) -> Result<f64> {
        if x.len() != self.width || x.iter().any(|v| v.is_infinite()) {
            return Err("invalid features");
        }
        Ok(self.base + self.rate * self.trees.iter().map(|t| t.predict(x)).sum::<f64>())
    }
}
