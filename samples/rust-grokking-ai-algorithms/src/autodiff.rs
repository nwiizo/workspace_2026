//! Scalar reverse-mode differentiation. Var handles belong to the Tape that created them.
use crate::{Result, numeric::sigmoid};
#[derive(Clone, Copy, Debug)]
pub struct Var(usize);
struct Node {
    value: f64,
    gradient: f64,
    parents: Vec<(usize, f64)>,
}
#[derive(Default)]
pub struct Tape {
    nodes: Vec<Node>,
}
impl Tape {
    pub fn new() -> Self {
        Self::default()
    }
    fn node(&mut self, value: f64, parents: Vec<(usize, f64)>) -> Var {
        let id = Var(self.nodes.len());
        self.nodes.push(Node {
            value,
            gradient: 0.0,
            parents,
        });
        id
    }
    pub fn constant(&mut self, value: f64) -> Var {
        self.node(value, vec![])
    }
    pub fn variables(&mut self, values: &[f64]) -> Vec<Var> {
        values.iter().map(|v| self.constant(*v)).collect()
    }
    pub fn value(&self, id: Var) -> f64 {
        self.nodes[id.0].value
    }
    pub fn gradient(&self, id: Var) -> f64 {
        self.nodes[id.0].gradient
    }
    pub fn add(&mut self, a: Var, b: Var) -> Var {
        self.node(self.value(a) + self.value(b), vec![(a.0, 1.0), (b.0, 1.0)])
    }
    pub fn sub(&mut self, a: Var, b: Var) -> Var {
        self.node(self.value(a) - self.value(b), vec![(a.0, 1.0), (b.0, -1.0)])
    }
    pub fn mul(&mut self, a: Var, b: Var) -> Var {
        self.node(
            self.value(a) * self.value(b),
            vec![(a.0, self.value(b)), (b.0, self.value(a))],
        )
    }
    pub fn scale(&mut self, a: Var, factor: f64) -> Var {
        self.node(self.value(a) * factor, vec![(a.0, factor)])
    }
    pub fn offset(&mut self, a: Var, amount: f64) -> Var {
        self.node(self.value(a) + amount, vec![(a.0, 1.0)])
    }
    pub fn exp(&mut self, a: Var) -> Var {
        let e = self.value(a).exp();
        self.node(e, vec![(a.0, e)])
    }
    pub fn log(&mut self, a: Var) -> Var {
        self.node(self.value(a).ln(), vec![(a.0, 1.0 / self.value(a))])
    }
    pub fn tanh(&mut self, a: Var) -> Var {
        let y = self.value(a).tanh();
        self.node(y, vec![(a.0, 1.0 - y * y)])
    }
    pub fn sigmoid(&mut self, a: Var) -> Var {
        let y = sigmoid(self.value(a));
        self.node(y, vec![(a.0, y * (1.0 - y))])
    }
    pub fn relu(&mut self, a: Var) -> Var {
        self.node(
            self.value(a).max(0.0),
            vec![(a.0, if self.value(a) > 0.0 { 1.0 } else { 0.0 })],
        )
    }
    pub fn silu(&mut self, a: Var) -> Var {
        let s = self.sigmoid(a);
        self.mul(a, s)
    }
    pub fn powf(&mut self, a: Var, p: f64) -> Var {
        self.node(
            self.value(a).powf(p),
            vec![(a.0, p * self.value(a).powf(p - 1.0))],
        )
    }
    pub fn sum(&mut self, values: &[Var]) -> Var {
        self.node(
            values.iter().map(|v| self.value(*v)).sum(),
            values.iter().map(|v| (v.0, 1.0)).collect(),
        )
    }
    pub fn dot(&mut self, a: &[Var], b: &[Var]) -> Var {
        assert_eq!(a.len(), b.len(), "validated dot dimensions");
        let value = a
            .iter()
            .zip(b)
            .map(|(x, y)| self.value(*x) * self.value(*y))
            .sum();
        let parents = a
            .iter()
            .zip(b)
            .flat_map(|(x, y)| [(x.0, self.value(*y)), (y.0, self.value(*x))])
            .collect();
        self.node(value, parents)
    }
    pub fn softmax(&mut self, logits: &[Var]) -> Vec<Var> {
        let max = logits
            .iter()
            .map(|v| self.value(*v))
            .fold(f64::NEG_INFINITY, f64::max);
        let exp: Vec<_> = logits
            .iter()
            .map(|v| (self.value(*v) - max).exp())
            .collect();
        let total: f64 = exp.iter().sum();
        let p: Vec<_> = exp.iter().map(|v| v / total).collect();
        p.iter()
            .enumerate()
            .map(|(i, &v)| {
                self.node(
                    v,
                    logits
                        .iter()
                        .enumerate()
                        .map(|(j, l)| (l.0, v * (if i == j { 1.0 } else { 0.0 } - p[j])))
                        .collect(),
                )
            })
            .collect()
    }
    /// Stable log-sum-exp cross entropy; gradients are p - one_hot(target).
    pub fn cross_entropy(&mut self, logits: &[Var], target: usize) -> Result<Var> {
        if target >= logits.len() || logits.iter().any(|v| !self.value(*v).is_finite()) {
            return Err("invalid cross entropy input");
        }
        let max = logits
            .iter()
            .map(|v| self.value(*v))
            .fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<_> = logits
            .iter()
            .map(|v| (self.value(*v) - max).exp())
            .collect();
        let total: f64 = exps.iter().sum();
        Ok(self.node(
            max + total.ln() - self.value(logits[target]),
            logits
                .iter()
                .enumerate()
                .map(|(i, v)| (v.0, exps[i] / total - if i == target { 1.0 } else { 0.0 }))
                .collect(),
        ))
    }
    pub fn mse(&mut self, predictions: &[Var], targets: &[f64]) -> Result<Var> {
        if predictions.is_empty()
            || predictions.len() != targets.len()
            || targets.iter().any(|v| !v.is_finite())
        {
            return Err("invalid MSE targets");
        }
        let scale = 1.0 / targets.len() as f64;
        let value = predictions
            .iter()
            .zip(targets)
            .map(|(p, t)| (self.value(*p) - t).powi(2) * scale)
            .sum();
        let parents = predictions
            .iter()
            .zip(targets)
            .map(|(p, t)| (p.0, 2.0 * scale * (self.value(*p) - t)))
            .collect();
        Ok(self.node(value, parents))
    }
    pub fn backward(&mut self, loss: Var) -> Result<()> {
        if !self.value(loss).is_finite() {
            return Err("nonfinite loss");
        }
        for node in &mut self.nodes {
            node.gradient = 0.0;
        }
        self.nodes[loss.0].gradient = 1.0;
        for i in (0..=loss.0).rev() {
            let gradient = self.nodes[i].gradient;
            for (parent, derivative) in self.nodes[i].parents.clone() {
                self.nodes[parent].gradient += gradient * derivative;
            }
        }
        if self.nodes.iter().any(|n| !n.gradient.is_finite()) {
            return Err("nonfinite gradient");
        }
        Ok(())
    }
    pub fn sgd(&self, parameters: &mut [f64], variables: &[Var], rate: f64) -> Result<()> {
        if parameters.len() != variables.len() || !rate.is_finite() || rate <= 0.0 {
            return Err("invalid optimizer input");
        }
        let updated: Vec<_> = parameters
            .iter()
            .zip(variables)
            .map(|(p, v)| p - rate * self.gradient(*v))
            .collect();
        if updated.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite update");
        }
        parameters.copy_from_slice(&updated);
        Ok(())
    }
}
