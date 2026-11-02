use crate::{
    Result,
    language::{Transformer, TransformerConfig},
};
use std::fmt::Write;

/// Versioned, exact f64 bit representation plus the six architecture dimensions.
/// This returns text; callers choose a file location. No user configuration is read.
pub fn transformer_checkpoint(model: &Transformer) -> Result<String> {
    if model.parameters.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite checkpoint weights");
    }
    let c = model.config;
    let mut text = format!(
        "RGA_TRANSFORMER_V1\n{} {} {} {} {} {}\n",
        c.vocabulary, c.width, c.heads, c.feed_forward, c.layers, c.context
    );
    for value in &model.parameters {
        writeln!(&mut text, "{:016x}", value.to_bits()).expect("writing to String is infallible");
    }
    Ok(text)
}
pub fn restore_transformer(text: &str) -> Result<Transformer> {
    let mut fields = text.split_whitespace();
    if fields.next() != Some("RGA_TRANSFORMER_V1") {
        return Err("unsupported checkpoint version");
    }
    let mut dimension = || {
        fields
            .next()
            .ok_or("missing dimensions")?
            .parse::<usize>()
            .map_err(|_| "invalid dimension")
    };
    let c = TransformerConfig {
        vocabulary: dimension()?,
        width: dimension()?,
        heads: dimension()?,
        feed_forward: dimension()?,
        layers: dimension()?,
        context: dimension()?,
    };
    let weights: Result<Vec<_>> = fields
        .map(|v| {
            u64::from_str_radix(v, 16)
                .map(f64::from_bits)
                .map_err(|_| "invalid weight bits")
        })
        .collect();
    let weights = weights?;
    if weights.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite checkpoint weights");
    }
    // Reject impossible sizes before allocating a network from an untrusted header.
    let dense = |input: usize, output: usize| input.saturating_mul(output).saturating_add(output);
    let block = 4usize
        .saturating_mul(dense(c.width, c.width))
        .saturating_add(dense(c.width, c.feed_forward))
        .saturating_add(dense(c.feed_forward, c.width))
        .saturating_add(4usize.saturating_mul(c.width));
    let expected = c
        .vocabulary
        .saturating_mul(c.width)
        .saturating_add(c.layers.saturating_mul(block))
        .saturating_add(dense(c.width, c.vocabulary));
    if weights.len() != expected {
        return Err("checkpoint weight count does not match architecture");
    }
    let mut model = Transformer::new(c, 0)?;
    model.parameters = weights;
    Ok(model)
}

pub struct EarlyStopping {
    patience: usize,
    min_delta: f64,
    best: f64,
    stale: usize,
}
impl EarlyStopping {
    pub fn new(patience: usize, min_delta: f64) -> Result<Self> {
        if patience == 0 || !min_delta.is_finite() || min_delta < 0.0 {
            return Err("invalid early stopping settings");
        }
        Ok(Self {
            patience,
            min_delta,
            best: f64::INFINITY,
            stale: 0,
        })
    }
    pub fn observe(&mut self, validation_loss: f64) -> Result<bool> {
        if !validation_loss.is_finite() {
            return Err("nonfinite validation loss");
        }
        if validation_loss < self.best - self.min_delta {
            self.best = validation_loss;
            self.stale = 0;
        } else {
            self.stale += 1;
        }
        Ok(self.stale >= self.patience)
    }
}
pub fn grid_search<T: Clone>(
    candidates: &[T],
    mut validation: impl FnMut(&T) -> Result<f64>,
) -> Result<(T, f64)> {
    let mut best: Option<(T, f64)> = None;
    for candidate in candidates {
        let score = validation(candidate)?;
        if !score.is_finite() {
            return Err("nonfinite validation score");
        }
        if best.as_ref().is_none_or(|v| score < v.1) {
            best = Some((candidate.clone(), score));
        }
    }
    best.ok_or("empty hyperparameter grid")
}
