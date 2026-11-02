use crate::{Result, autodiff::Tape, random::Rng, vision::FeatureMap, vision::UNet};
#[derive(Clone, Copy)]
pub enum NoiseSchedule {
    Linear,
    Cosine,
    Sigmoid,
}
pub struct Diffusion {
    pub beta: Vec<f64>,
    pub alpha_bar: Vec<f64>,
}
impl Diffusion {
    pub fn new(steps: usize, schedule: NoiseSchedule) -> Result<Self> {
        if steps == 0 {
            return Err("zero diffusion steps");
        }
        let beta: Vec<_> = (0..steps)
            .map(|i| {
                let t = i as f64 / (steps - 1).max(1) as f64;
                match schedule {
                    NoiseSchedule::Linear => 0.0001 + (0.02 - 0.0001) * t,
                    NoiseSchedule::Sigmoid => {
                        0.0001 + (0.02 - 0.0001) * crate::numeric::sigmoid(-6.0 + 12.0 * t)
                    }
                    NoiseSchedule::Cosine => {
                        let f = |s: f64| {
                            (((s + 0.008) / 1.008) * std::f64::consts::FRAC_PI_2)
                                .cos()
                                .powi(2)
                        };
                        (1.0 - f((i + 1) as f64 / steps as f64) / f(i as f64 / steps as f64))
                            .clamp(1e-8, 0.999)
                    }
                }
            })
            .collect();
        Self::from_beta(beta)
    }
    pub fn from_beta(beta: Vec<f64>) -> Result<Self> {
        if beta.is_empty()
            || beta
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.0 || *v >= 1.0)
        {
            return Err("beta must lie strictly between zero and one");
        }
        let mut product = 1.0;
        let alpha_bar = beta
            .iter()
            .map(|v| {
                product *= 1.0 - v;
                product
            })
            .collect();
        Ok(Self { beta, alpha_bar })
    }
    pub fn forward(&self, clean: &[f64], noise: &[f64], step: usize) -> Result<Vec<f64>> {
        if step >= self.beta.len()
            || clean.is_empty()
            || clean.len() != noise.len()
            || clean.iter().chain(noise).any(|v| !v.is_finite())
        {
            return Err("invalid forward diffusion input");
        }
        let a = self.alpha_bar[step];
        Ok(clean
            .iter()
            .zip(noise)
            .map(|(x, e)| a.sqrt() * x + (1.0 - a).sqrt() * e)
            .collect())
    }
    pub fn reverse_step(
        &self,
        noisy: &[f64],
        predicted_noise: &[f64],
        step: usize,
        rng: &mut Rng,
    ) -> Result<Vec<f64>> {
        if step >= self.beta.len()
            || noisy.is_empty()
            || noisy.len() != predicted_noise.len()
            || noisy.iter().chain(predicted_noise).any(|v| !v.is_finite())
        {
            return Err("invalid reverse diffusion input");
        }
        let beta = self.beta[step];
        let alpha = 1.0 - beta;
        let abar = self.alpha_bar[step];
        let previous = if step == 0 {
            1.0
        } else {
            self.alpha_bar[step - 1]
        };
        let stddev = (beta * (1.0 - previous) / (1.0 - abar)).sqrt();
        Ok(noisy
            .iter()
            .zip(predicted_noise)
            .map(|(x, e)| {
                (x - beta * e / (1.0 - abar).sqrt()) / alpha.sqrt()
                    + if step == 0 {
                        0.0
                    } else {
                        stddev * rng.normal()
                    }
            })
            .collect())
    }
    pub fn generate(
        &self,
        model: &UNet,
        size: usize,
        label: &[f64],
        seed: u64,
        known: Option<(&[f64], &[bool])>,
    ) -> Result<Vec<f64>> {
        if size < 2 || size % 2 != 0 {
            return Err("image size must be positive and even");
        }
        if let Some((pixels, mask)) = known {
            if pixels.len() != size * size
                || mask.len() != pixels.len()
                || pixels.iter().any(|v| !v.is_finite())
            {
                return Err("invalid inpainting mask");
            }
        }
        let mut rng = Rng::new(seed);
        let mut image: Vec<_> = (0..size * size).map(|_| rng.normal()).collect();
        for step in (0..self.beta.len()).rev() {
            if let Some((pixels, mask)) = known {
                let noise: Vec<_> = pixels.iter().map(|_| rng.normal()).collect();
                let context = self.forward(pixels, &noise, step)?;
                for (i, edit) in mask.iter().enumerate() {
                    if !edit {
                        image[i] = context[i];
                    }
                }
            }
            let predicted = model.predict(&image, size, step, label)?;
            image = self.reverse_step(&image, &predicted, step, &mut rng)?;
        }
        if let Some((pixels, mask)) = known {
            for (i, edit) in mask.iter().enumerate() {
                if !edit {
                    image[i] = pixels[i];
                }
            }
        }
        if image.iter().any(|v| !v.is_finite()) {
            return Err("sampling diverged");
        }
        Ok(image)
    }
    /// Upsample a generated image and refine it from an intermediate noise level.
    pub fn high_resolution_fix(
        &self,
        model: &UNet,
        image: &[f64],
        size: usize,
        start_step: usize,
        label: &[f64],
        seed: u64,
    ) -> Result<Vec<f64>> {
        if image.len() != size * size || size == 0 || start_step >= self.beta.len() {
            return Err("invalid high-resolution input");
        }
        let new_size = size * 2;
        let enlarged: Vec<_> = (0..new_size * new_size)
            .map(|i| image[(i / new_size / 2) * size + (i % new_size / 2)])
            .collect();
        let mut rng = Rng::new(seed);
        let noise: Vec<_> = enlarged.iter().map(|_| rng.normal()).collect();
        let mut current = self.forward(&enlarged, &noise, start_step)?;
        for step in (0..=start_step).rev() {
            let predicted = model.predict(&current, new_size, step, label)?;
            current = self.reverse_step(&current, &predicted, step, &mut rng)?;
        }
        Ok(current)
    }
}
pub fn normalize_pixels(bytes: &[u8]) -> Vec<f64> {
    bytes.iter().map(|v| f64::from(*v) / 127.5 - 1.0).collect()
}

/// REINFORCE on one stochastic denoising transition, with a learned linear image reward.
/// The detached reward baseline is the score at the current transition mean.
pub fn reinforce_denoiser(
    model: &mut UNet,
    diffusion: &Diffusion,
    example: &crate::vision::NoiseExample,
    reward_weights: &[f64],
    updates: usize,
    rate: f64,
    seed: u64,
) -> Result<()> {
    let step = example.timestep;
    if step == 0
        || step >= diffusion.beta.len()
        || reward_weights.len() != example.noisy.len()
        || reward_weights.iter().any(|v| !v.is_finite())
    {
        return Err("reward training needs a stochastic transition");
    }
    let beta = diffusion.beta[step];
    let abar = diffusion.alpha_bar[step];
    let previous = diffusion.alpha_bar[step - 1];
    let variance = beta * (1.0 - previous) / (1.0 - abar);
    let mut rng = Rng::new(seed);
    for _ in 0..updates {
        let mut t = Tape::new();
        let p = t.variables(&model.parameters);
        let values = t.variables(&example.noisy);
        let x = FeatureMap::new(1, example.size, example.size, values)?;
        let eps = model.forward(&mut t, &p, &x, step, &example.label)?;
        let mut means = Vec::new();
        let mut sampled = Vec::new();
        for (&noisy, &prediction) in example.noisy.iter().zip(&eps.values) {
            let correction = t.scale(prediction, -beta / (1.0 - abar).sqrt());
            let mean = t.offset(correction, noisy);
            let mean = t.scale(mean, 1.0 / (1.0 - beta).sqrt());
            sampled.push(t.value(mean) + variance.sqrt() * rng.normal());
            means.push(mean);
        }
        let advantage: f64 = sampled
            .iter()
            .zip(&means)
            .zip(reward_weights)
            .map(|((x, mean), w)| (x - t.value(*mean)) * w)
            .sum();
        let terms: Vec<_> = means
            .iter()
            .zip(&sampled)
            .map(|(mean, x)| {
                let difference = t.offset(*mean, -x);
                t.mul(difference, difference)
            })
            .collect();
        let sum = t.sum(&terms);
        let loss = t.scale(sum, advantage / (2.0 * variance));
        t.backward(loss)?;
        t.sgd(&mut model.parameters, &p, rate)?;
    }
    Ok(())
}
pub fn outpainting_canvas(
    image: &[f64],
    width: usize,
    height: usize,
    padding: usize,
) -> Result<(Vec<f64>, Vec<bool>, usize, usize)> {
    if width == 0
        || height == 0
        || image.len() != width * height
        || image.iter().any(|v| !v.is_finite())
    {
        return Err("invalid canvas");
    }
    let w = width + 2 * padding;
    let h = height + 2 * padding;
    let mut canvas = vec![0.0; w * h];
    let mut mask = vec![true; w * h];
    for y in 0..height {
        for x in 0..width {
            let target = (y + padding) * w + x + padding;
            canvas[target] = image[y * width + x];
            mask[target] = false;
        }
    }
    Ok((canvas, mask, w, h))
}
