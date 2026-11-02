use crate::{
    Result,
    autodiff::{Tape, Var},
    neural::Dense,
    numeric::{matrix_width, symmetric_eigen},
    random::Rng,
};

#[derive(Clone)]
pub struct FeatureMap {
    pub channels: usize,
    pub height: usize,
    pub width: usize,
    pub values: Vec<Var>,
}
impl FeatureMap {
    pub fn new(channels: usize, height: usize, width: usize, values: Vec<Var>) -> Result<Self> {
        if [channels, height, width].contains(&0)
            || channels
                .checked_mul(height)
                .and_then(|n| n.checked_mul(width))
                != Some(values.len())
        {
            return Err("invalid feature map shape");
        }
        Ok(Self {
            channels,
            height,
            width,
            values,
        })
    }
    fn at(&self, c: usize, y: usize, x: usize) -> Var {
        self.values[(c * self.height + y) * self.width + x]
    }
    pub fn silu(mut self, t: &mut Tape) -> Self {
        self.values = self.values.iter().map(|v| t.silu(*v)).collect();
        self
    }
    pub fn concatenate(&self, other: &Self) -> Result<Self> {
        if self.height != other.height || self.width != other.width {
            return Err("skip dimensions differ");
        }
        let mut values = self.values.clone();
        values.extend_from_slice(&other.values);
        Self::new(
            self.channels + other.channels,
            self.height,
            self.width,
            values,
        )
    }
}
#[derive(Clone)]
pub struct Conv2d {
    input: usize,
    output: usize,
    kernel: usize,
    stride: usize,
    padding: usize,
    transpose: bool,
    weights: usize,
    bias: usize,
}
#[derive(Clone, Copy)]
pub struct ConvConfig {
    pub input: usize,
    pub output: usize,
    pub kernel: usize,
    pub stride: usize,
    pub padding: usize,
    pub transpose: bool,
}
impl Conv2d {
    pub fn new(p: &mut Vec<f64>, c: ConvConfig, rng: &mut Rng) -> Result<Self> {
        if [c.input, c.output, c.kernel, c.stride].contains(&0) {
            return Err("invalid convolution");
        }
        let weights = p.len();
        let scale = 1.0 / (c.input * c.kernel * c.kernel) as f64;
        p.extend(
            (0..c.output * c.input * c.kernel * c.kernel).map(|_| rng.normal() * scale.sqrt()),
        );
        let bias = p.len();
        p.extend(vec![0.0; c.output]);
        Ok(Self {
            input: c.input,
            output: c.output,
            kernel: c.kernel,
            stride: c.stride,
            padding: c.padding,
            transpose: c.transpose,
            weights,
            bias,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], x: &FeatureMap) -> Result<FeatureMap> {
        if x.channels != self.input || p.len() < self.bias + self.output {
            return Err("invalid convolution input");
        }
        let extent = |size: usize| -> Option<usize> {
            if self.transpose {
                (size - 1)
                    .checked_mul(self.stride)?
                    .checked_add(self.kernel)?
                    .checked_sub(2 * self.padding)
            } else {
                size.checked_add(2 * self.padding)?
                    .checked_sub(self.kernel)
                    .map(|n| n / self.stride + 1)
            }
        };
        let h = extent(x.height)
            .filter(|v| *v > 0)
            .ok_or("invalid convolution height")?;
        let w = extent(x.width)
            .filter(|v| *v > 0)
            .ok_or("invalid convolution width")?;
        let mut values = Vec::new();
        for oc in 0..self.output {
            for oy in 0..h {
                for ox in 0..w {
                    let mut inputs = Vec::new();
                    let mut weights = Vec::new();
                    for ic in 0..self.input {
                        for ky in 0..self.kernel {
                            for kx in 0..self.kernel {
                                let source = if self.transpose {
                                    let sy = (oy + self.padding).checked_sub(ky);
                                    let sx = (ox + self.padding).checked_sub(kx);
                                    match (sy, sx) {
                                        (Some(y), Some(x))
                                            if y % self.stride == 0 && x % self.stride == 0 =>
                                        {
                                            Some((y / self.stride, x / self.stride))
                                        }
                                        _ => None,
                                    }
                                } else {
                                    (oy * self.stride + ky)
                                        .checked_sub(self.padding)
                                        .zip((ox * self.stride + kx).checked_sub(self.padding))
                                };
                                if let Some((iy, ix)) = source {
                                    if iy < x.height && ix < x.width {
                                        inputs.push(x.at(ic, iy, ix));
                                        weights.push(
                                            p[self.weights
                                                + ((oc * self.input + ic) * self.kernel + ky)
                                                    * self.kernel
                                                + kx],
                                        );
                                    }
                                }
                            }
                        }
                    }
                    let sum = t.dot(&inputs, &weights);
                    values.push(t.add(sum, p[self.bias + oc]));
                }
            }
        }
        FeatureMap::new(self.output, h, w, values)
    }
}
fn conv(
    input: usize,
    output: usize,
    kernel: usize,
    stride: usize,
    padding: usize,
    transpose: bool,
) -> ConvConfig {
    ConvConfig {
        input,
        output,
        kernel,
        stride,
        padding,
        transpose,
    }
}

/// A complete one-level, conditional U-Net with trainable convolutions and skip connections.
pub struct UNet {
    pub parameters: Vec<f64>,
    stem: Conv2d,
    down: Conv2d,
    bridge: Conv2d,
    up: Conv2d,
    decode: Conv2d,
    out: Conv2d,
    condition: Dense,
}
impl UNet {
    pub fn new(seed: u64) -> Result<Self> {
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let stem = Conv2d::new(&mut p, conv(1, 2, 3, 1, 1, false), &mut rng)?;
        let down = Conv2d::new(&mut p, conv(2, 3, 2, 2, 0, false), &mut rng)?;
        let bridge = Conv2d::new(&mut p, conv(3, 3, 3, 1, 1, false), &mut rng)?;
        let up = Conv2d::new(&mut p, conv(3, 2, 2, 2, 0, true), &mut rng)?;
        let decode = Conv2d::new(&mut p, conv(4, 2, 3, 1, 1, false), &mut rng)?;
        let out = Conv2d::new(&mut p, conv(2, 1, 1, 1, 0, false), &mut rng)?;
        let condition = Dense::new(&mut p, 8, 3, &mut rng);
        Ok(Self {
            parameters: p,
            stem,
            down,
            bridge,
            up,
            decode,
            out,
            condition,
        })
    }
    pub fn forward(
        &self,
        t: &mut Tape,
        p: &[Var],
        x: &FeatureMap,
        timestep: usize,
        label: &[f64],
    ) -> Result<FeatureMap> {
        self.forward_controlled(t, p, x, timestep, label, None)
    }
    pub fn forward_controlled(
        &self,
        t: &mut Tape,
        p: &[Var],
        x: &FeatureMap,
        timestep: usize,
        label: &[f64],
        control: Option<&FeatureMap>,
    ) -> Result<FeatureMap> {
        if p.len() != self.parameters.len()
            || x.height % 2 != 0
            || x.width % 2 != 0
            || label.len() != 4
            || label.iter().any(|v| !v.is_finite())
        {
            return Err("U-Net requires even dimensions and a four-dimensional label");
        }
        let skip = self.stem.forward(t, p, x)?.silu(t);
        let down = self.down.forward(t, p, &skip)?.silu(t);
        let mut bridge = self.bridge.forward(t, p, &down)?;
        let mut input = crate::language::positional_encoding(timestep, 4);
        input.extend_from_slice(label);
        let condition = t.variables(&input);
        let condition = self.condition.forward(t, p, &condition);
        let spatial = bridge.height * bridge.width;
        for (i, value) in bridge.values.iter_mut().enumerate() {
            *value = t.add(*value, condition[i / spatial]);
        }
        if let Some(residual) = control {
            if residual.channels != bridge.channels
                || residual.height != bridge.height
                || residual.width != bridge.width
            {
                return Err("ControlNet shape mismatch");
            }
            for (value, residual) in bridge.values.iter_mut().zip(&residual.values) {
                *value = t.add(*value, *residual);
            }
        }
        let bridge = bridge.silu(t);
        let up = self.up.forward(t, p, &bridge)?.silu(t);
        let combined = up.concatenate(&skip)?;
        let decoded = self.decode.forward(t, p, &combined)?.silu(t);
        self.out.forward(t, p, &decoded)
    }
    pub fn predict(
        &self,
        image: &[f64],
        size: usize,
        timestep: usize,
        label: &[f64],
    ) -> Result<Vec<f64>> {
        if image.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite pixels");
        }
        let mut t = Tape::new();
        let p = t.variables(&self.parameters);
        let values = t.variables(image);
        let x = FeatureMap::new(1, size, size, values)?;
        let output = self.forward(&mut t, &p, &x, timestep, label)?;
        Ok(output.values.iter().map(|v| t.value(*v)).collect())
    }
    pub fn train_noise(
        &mut self,
        examples: &[NoiseExample],
        steps: usize,
        rate: f64,
    ) -> Result<Vec<f64>> {
        if examples.is_empty() {
            return Err("empty denoising data");
        }
        let mut history = Vec::new();
        for step in 0..steps {
            let example = &examples[step % examples.len()];
            if example
                .noisy
                .iter()
                .chain(&example.noise)
                .any(|v| !v.is_finite())
            {
                return Err("nonfinite denoising example");
            }
            let mut t = Tape::new();
            let p = t.variables(&self.parameters);
            let values = t.variables(&example.noisy);
            let x = FeatureMap::new(1, example.size, example.size, values)?;
            let prediction = self.forward(&mut t, &p, &x, example.timestep, &example.label)?;
            let loss = t.mse(&prediction.values, &example.noise)?;
            history.push(t.value(loss));
            t.backward(loss)?;
            t.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(history)
    }
}
pub struct NoiseExample {
    pub noisy: Vec<f64>,
    pub noise: Vec<f64>,
    pub size: usize,
    pub timestep: usize,
    pub label: [f64; 4],
}

/// CNN classification with convolution, global average pooling and a learned readout.
pub struct CnnClassifier {
    pub parameters: Vec<f64>,
    convolution: Conv2d,
    readout: Dense,
}
impl CnnClassifier {
    pub fn new(classes: usize, seed: u64) -> Result<Self> {
        if classes < 2 {
            return Err("classification needs at least two classes");
        }
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let convolution = Conv2d::new(&mut p, conv(1, 4, 3, 1, 1, false), &mut rng)?;
        let readout = Dense::new(&mut p, 4, classes, &mut rng);
        Ok(Self {
            parameters: p,
            convolution,
            readout,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], image: &FeatureMap) -> Result<Vec<Var>> {
        let features = self.convolution.forward(t, p, image)?.silu(t);
        let area = features.height * features.width;
        let pooled: Vec<_> = features
            .values
            .chunks(area)
            .map(|channel| {
                let sum = t.sum(channel);
                t.scale(sum, 1.0 / area as f64)
            })
            .collect();
        Ok(self.readout.forward(t, p, &pooled))
    }
    pub fn train(
        &mut self,
        images: &[Vec<f64>],
        size: usize,
        labels: &[usize],
        steps: usize,
        rate: f64,
    ) -> Result<Vec<f64>> {
        if images.is_empty()
            || images.len() != labels.len()
            || images
                .iter()
                .any(|r| r.len() != size * size || r.iter().any(|v| !v.is_finite()))
        {
            return Err("invalid classifier images");
        }
        let mut history = Vec::new();
        for _ in 0..steps {
            let mut t = Tape::new();
            let p = t.variables(&self.parameters);
            let mut losses = Vec::new();
            for (image, &label) in images.iter().zip(labels) {
                let values = t.variables(image);
                let x = FeatureMap::new(1, size, size, values)?;
                let logits = self.forward(&mut t, &p, &x)?;
                losses.push(t.cross_entropy(&logits, label)?);
            }
            let sum = t.sum(&losses);
            let loss = t.scale(sum, 1.0 / images.len() as f64);
            history.push(t.value(loss));
            t.backward(loss)?;
            t.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(history)
    }
}

/// Trainable auxiliary control branch, initialized with a zero output projection.
pub struct ControlNet {
    pub parameters: Vec<f64>,
    encode: Conv2d,
    zero: Conv2d,
}
impl ControlNet {
    pub fn new(seed: u64) -> Result<Self> {
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let encode = Conv2d::new(&mut p, conv(1, 3, 2, 2, 0, false), &mut rng)?;
        let start = p.len();
        let zero = Conv2d::new(&mut p, conv(3, 3, 1, 1, 0, false), &mut rng)?;
        p[start..].fill(0.0);
        Ok(Self {
            parameters: p,
            encode,
            zero,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], control: &FeatureMap) -> Result<FeatureMap> {
        let encoded = self.encode.forward(t, p, control)?.silu(t);
        self.zero.forward(t, p, &encoded)
    }
}

pub struct ImagePromptAdapter {
    pub parameters: Vec<f64>,
    key: Dense,
    value: Dense,
}
impl ImagePromptAdapter {
    pub fn new(image_features: usize, width: usize, seed: u64) -> Result<Self> {
        if image_features == 0 || width == 0 {
            return Err("invalid adapter dimensions");
        }
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let key = Dense::new(&mut p, image_features, width, &mut rng);
        let value = Dense::new(&mut p, image_features, width, &mut rng);
        Ok(Self {
            parameters: p,
            key,
            value,
        })
    }
    /// Decoupled image cross-attention, added to already computed text attention.
    pub fn forward(
        &self,
        t: &mut Tape,
        p: &[Var],
        queries: &[Vec<Var>],
        image: &[Vec<Var>],
        text_output: &[Vec<Var>],
        scale: f64,
    ) -> Result<Vec<Vec<Var>>> {
        if image.is_empty()
            || image.iter().any(|r| r.len() != self.key.input)
            || p.len() != self.parameters.len()
            || !scale.is_finite()
            || text_output.len() != queries.len()
            || text_output.iter().any(|r| r.len() != self.key.output)
        {
            return Err("invalid adapter input");
        }
        let keys: Vec<_> = image.iter().map(|r| self.key.forward(t, p, r)).collect();
        let values: Vec<_> = image.iter().map(|r| self.value.forward(t, p, r)).collect();
        let attended = crate::language::attention(t, queries, &keys, &values, false)?;
        Ok(attended
            .iter()
            .zip(text_output)
            .map(|(image, text)| {
                image
                    .iter()
                    .zip(text)
                    .map(|(a, b)| {
                        let scaled = t.scale(*a, scale);
                        t.add(scaled, *b)
                    })
                    .collect()
            })
            .collect())
    }
}

/// A learned 2x upscaler: feature extraction followed by transposed convolution.
pub struct Upscaler {
    pub parameters: Vec<f64>,
    encode: Conv2d,
    up: Conv2d,
}
impl Upscaler {
    pub fn new(seed: u64) -> Result<Self> {
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let encode = Conv2d::new(&mut p, conv(1, 4, 3, 1, 1, false), &mut rng)?;
        let up = Conv2d::new(&mut p, conv(4, 1, 2, 2, 0, true), &mut rng)?;
        Ok(Self {
            parameters: p,
            encode,
            up,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], x: &FeatureMap) -> Result<FeatureMap> {
        let hidden = self.encode.forward(t, p, x)?.silu(t);
        self.up.forward(t, p, &hidden)
    }
}

pub struct Lora {
    pub base: Vec<f64>,
    pub parameters: Vec<f64>,
    input: usize,
    output: usize,
    rank: usize,
    alpha: f64,
}
impl Lora {
    pub fn new(
        base: Vec<f64>,
        input: usize,
        output: usize,
        rank: usize,
        alpha: f64,
        seed: u64,
    ) -> Result<Self> {
        if [input, output, rank].contains(&0)
            || base.len() != input * output
            || base.iter().any(|v| !v.is_finite())
            || !alpha.is_finite()
        {
            return Err("invalid LoRA dimensions");
        }
        let mut rng = Rng::new(seed);
        let mut p: Vec<_> = (0..rank * input).map(|_| rng.normal() * 0.1).collect();
        p.extend(vec![0.0; output * rank]);
        Ok(Self {
            base,
            parameters: p,
            input,
            output,
            rank,
            alpha,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], x: &[Var]) -> Result<Vec<Var>> {
        if x.len() != self.input || p.len() != self.parameters.len() {
            return Err("invalid LoRA input");
        }
        let hidden: Vec<_> = (0..self.rank)
            .map(|i| t.dot(x, &p[i * self.input..(i + 1) * self.input]))
            .collect();
        let start = self.rank * self.input;
        Ok((0..self.output)
            .map(|i| {
                let frozen = t.variables(&self.base[i * self.input..(i + 1) * self.input]);
                let base = t.dot(x, &frozen);
                let change = t.dot(
                    &hidden,
                    &p[start + i * self.rank..start + (i + 1) * self.rank],
                );
                let change = t.scale(change, self.alpha / self.rank as f64);
                t.add(base, change)
            })
            .collect())
    }
}

/// Group normalization across each channel group and its spatial coordinates.
/// Gamma and beta have one trainable scalar per channel. One image, CHW layout.
pub fn group_norm(
    t: &mut Tape,
    x: &FeatureMap,
    groups: usize,
    gamma: &[Var],
    beta: &[Var],
) -> Result<FeatureMap> {
    if groups == 0
        || x.channels % groups != 0
        || gamma.len() != x.channels
        || beta.len() != x.channels
    {
        return Err("invalid group normalization dimensions");
    }
    let channels_per_group = x.channels / groups;
    let area = x.height * x.width;
    let mut output = Vec::new();
    for group in 0..groups {
        let start = group * channels_per_group;
        let expanded_gamma: Vec<_> = (start..start + channels_per_group)
            .flat_map(|c| std::iter::repeat_n(gamma[c], area))
            .collect();
        let expanded_beta: Vec<_> = (start..start + channels_per_group)
            .flat_map(|c| std::iter::repeat_n(beta[c], area))
            .collect();
        output.extend(crate::language::layer_norm(
            t,
            &x.values[start * area..(start + channels_per_group) * area],
            &expanded_gamma,
            &expanded_beta,
        )?);
    }
    FeatureMap::new(x.channels, x.height, x.width, output)
}

/// Non-overlapping 2x2 pooling; ties for max pooling route gradient to the first maximum.
pub fn pool2d(t: &mut Tape, x: &FeatureMap, maximum: bool) -> Result<FeatureMap> {
    if x.height % 2 != 0 || x.width % 2 != 0 {
        return Err("pooling requires even dimensions");
    }
    let mut output = Vec::new();
    for c in 0..x.channels {
        for y in (0..x.height).step_by(2) {
            for z in (0..x.width).step_by(2) {
                let start = (c * x.height + y) * x.width + z;
                let window = [
                    x.values[start],
                    x.values[start + 1],
                    x.values[start + x.width],
                    x.values[start + x.width + 1],
                ];
                let value = if maximum {
                    window.iter().copied().fold(window[0], |best, v| {
                        if t.value(v) > t.value(best) { v } else { best }
                    })
                } else {
                    let sum = t.sum(&window);
                    t.scale(sum, 0.25)
                };
                output.push(value);
            }
        }
    }
    FeatureMap::new(x.channels, x.height / 2, x.width / 2, output)
}

/// Fréchet distance on provided feature vectors. Inception embeddings make this FID.
pub fn frechet_feature_distance(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<f64> {
    let d = matrix_width(a)?;
    if matrix_width(b)? != d || a.len() < 2 || b.len() < 2 {
        return Err("insufficient feature samples");
    }
    let stats = |x: &[Vec<f64>]| {
        let mean: Vec<_> = (0..d)
            .map(|j| x.iter().map(|r| r[j]).sum::<f64>() / x.len() as f64)
            .collect();
        let covariance: Vec<Vec<_>> = (0..d)
            .map(|i| {
                (0..d)
                    .map(|j| {
                        x.iter()
                            .map(|r| (r[i] - mean[i]) * (r[j] - mean[j]))
                            .sum::<f64>()
                            / (x.len() - 1) as f64
                    })
                    .collect()
            })
            .collect();
        (mean, covariance)
    };
    let (ma, ca) = stats(a);
    let (mb, cb) = stats(b);
    let (eigen, vectors) = symmetric_eigen(&ca)?;
    let sqrt: Vec<Vec<f64>> = (0..d)
        .map(|i| {
            (0..d)
                .map(|j| {
                    (0..d)
                        .map(|k| vectors[i][k] * eigen[k].max(0.0).sqrt() * vectors[j][k])
                        .sum()
                })
                .collect()
        })
        .collect();
    let temp: Vec<Vec<f64>> = (0..d)
        .map(|i| {
            (0..d)
                .map(|j| (0..d).map(|k| sqrt[i][k] * cb[k][j]).sum())
                .collect()
        })
        .collect();
    let mut product: Vec<Vec<f64>> = (0..d)
        .map(|i| {
            (0..d)
                .map(|j| (0..d).map(|k| temp[i][k] * sqrt[k][j]).sum())
                .collect()
        })
        .collect();
    let unsymmetrized = product.clone();
    for (i, row) in product.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = (unsymmetrized[i][j] + unsymmetrized[j][i]) / 2.0;
        }
    }
    let (spectrum, _) = symmetric_eigen(&product)?;
    Ok(
        (crate::numeric::distance2(&ma, &mb) + (0..d).map(|i| ca[i][i] + cb[i][i]).sum::<f64>()
            - 2.0 * spectrum.iter().map(|v| v.max(0.0).sqrt()).sum::<f64>())
        .max(0.0),
    )
}

/// Canny: Gaussian smoothing, Sobel gradients, non-maximum suppression and hysteresis.
pub fn canny(image: &[f64], width: usize, height: usize, low: f64, high: f64) -> Result<Vec<bool>> {
    if width < 3
        || height < 3
        || image.len() != width * height
        || image.iter().any(|v| !v.is_finite())
        || !low.is_finite()
        || !high.is_finite()
        || low < 0.0
        || high <= low
    {
        return Err("invalid edge detector input");
    }
    let mut smooth = vec![0.0; image.len()];
    for y in 0..height {
        for x in 0..width {
            for dy in 0..3 {
                for dx in 0..3 {
                    let sy = (y + dy).saturating_sub(1).min(height - 1);
                    let sx = (x + dx).saturating_sub(1).min(width - 1);
                    smooth[y * width + x] +=
                        image[sy * width + sx] * [1.0, 2.0, 1.0][dx] * [1.0, 2.0, 1.0][dy] / 16.0;
                }
            }
        }
    }
    let mut magnitude = vec![0.0; image.len()];
    let mut angle = vec![0.0; image.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let (mut gx, mut gy) = (0.0, 0.0);
            for dy in 0..3 {
                for dx in 0..3 {
                    let value = smooth[(y + dy - 1) * width + x + dx - 1];
                    gx += value * [-1.0, 0.0, 1.0][dx] * [1.0, 2.0, 1.0][dy];
                    gy += value * [-1.0, 0.0, 1.0][dy] * [1.0, 2.0, 1.0][dx];
                }
            }
            magnitude[y * width + x] = gx.hypot(gy);
            angle[y * width + x] = gy.atan2(gx).to_degrees().rem_euclid(180.0);
        }
    }
    let mut suppressed = vec![0.0; image.len()];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = y * width + x;
            let a = angle[i];
            let (before, after) = if !(22.5..157.5).contains(&a) {
                (i - 1, i + 1)
            } else if a < 67.5 {
                (i - width - 1, i + width + 1)
            } else if a < 112.5 {
                (i - width, i + width)
            } else {
                (i - width + 1, i + width - 1)
            };
            if magnitude[i] >= magnitude[before] && magnitude[i] >= magnitude[after] {
                suppressed[i] = magnitude[i];
            }
        }
    }
    let mut edges = vec![false; image.len()];
    let mut stack = Vec::new();
    for (i, &value) in suppressed.iter().enumerate() {
        if value >= high {
            edges[i] = true;
            stack.push(i);
        }
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        for sy in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            for sx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                let j = sy * width + sx;
                if !edges[j] && suppressed[j] > 0.0 && suppressed[j] >= low {
                    edges[j] = true;
                    stack.push(j);
                }
            }
        }
    }
    Ok(edges)
}
