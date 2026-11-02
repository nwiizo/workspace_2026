use crate::{
    Result,
    autodiff::{Tape, Var},
    numeric::{dot, matrix_width},
    random::Rng,
};

#[derive(Clone, Copy)]
pub enum Activation {
    Linear,
    Step,
    Sigmoid,
    Tanh,
    Relu,
    LeakyRelu,
    Silu,
}
impl Activation {
    pub fn apply(self, tape: &mut Tape, x: Var) -> Var {
        match self {
            Self::Linear => x,
            Self::Step => tape.constant(if tape.value(x) >= 0.0 { 1.0 } else { 0.0 }),
            Self::Sigmoid => tape.sigmoid(x),
            Self::Tanh => tape.tanh(x),
            Self::Relu => tape.relu(x),
            Self::LeakyRelu => {
                if tape.value(x) >= 0.0 {
                    x
                } else {
                    tape.scale(x, 0.01)
                }
            }
            Self::Silu => tape.silu(x),
        }
    }
}
#[derive(Clone)]
pub struct Dense {
    pub input: usize,
    pub output: usize,
    weights: usize,
    bias: usize,
}
impl Dense {
    pub fn new(parameters: &mut Vec<f64>, input: usize, output: usize, rng: &mut Rng) -> Self {
        let weights = parameters.len();
        parameters.extend((0..input * output).map(|_| rng.normal() / (input as f64).sqrt()));
        let bias = parameters.len();
        parameters.resize(bias + output, 0.0);
        Self {
            input,
            output,
            weights,
            bias,
        }
    }
    pub fn forward(&self, tape: &mut Tape, p: &[Var], x: &[Var]) -> Vec<Var> {
        assert_eq!(x.len(), self.input, "validated dense input");
        (0..self.output)
            .map(|i| {
                let sum = tape.dot(
                    x,
                    &p[self.weights + i * self.input..self.weights + (i + 1) * self.input],
                );
                tape.add(sum, p[self.bias + i])
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct Mlp {
    pub parameters: Vec<f64>,
    layers: Vec<Dense>,
    hidden: Activation,
    output: Activation,
}
impl Mlp {
    pub fn new(sizes: &[usize], hidden: Activation, output: Activation, seed: u64) -> Result<Self> {
        if sizes.len() < 2 || sizes.contains(&0) {
            return Err("invalid layer sizes");
        }
        let mut parameters = Vec::new();
        let mut rng = Rng::new(seed);
        let layers = sizes
            .windows(2)
            .map(|s| Dense::new(&mut parameters, s[0], s[1], &mut rng))
            .collect();
        Ok(Self {
            parameters,
            layers,
            hidden,
            output,
        })
    }
    pub fn input_size(&self) -> usize {
        self.layers[0].input
    }
    pub fn output_size(&self) -> usize {
        self.layers[self.layers.len() - 1].output
    }
    pub fn forward(&self, tape: &mut Tape, p: &[Var], x: &[Var]) -> Result<Vec<Var>> {
        if x.len() != self.input_size() || p.len() != self.parameters.len() {
            return Err("invalid network dimensions");
        }
        let mut values = x.to_vec();
        for (i, layer) in self.layers.iter().enumerate() {
            values = layer.forward(tape, p, &values);
            let activation = if i + 1 == self.layers.len() {
                self.output
            } else {
                self.hidden
            };
            values = values.iter().map(|v| activation.apply(tape, *v)).collect();
        }
        Ok(values)
    }
    pub fn predict(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite input");
        }
        let mut tape = Tape::new();
        let p = tape.variables(&self.parameters);
        let x = tape.variables(x);
        let values = self.forward(&mut tape, &p, &x)?;
        Ok(values.iter().map(|v| tape.value(*v)).collect())
    }
    pub fn train_mse(
        &mut self,
        x: &[Vec<f64>],
        y: &[Vec<f64>],
        epochs: usize,
        rate: f64,
    ) -> Result<Vec<f64>> {
        if matrix_width(x)? != self.input_size()
            || matrix_width(y)? != self.output_size()
            || x.len() != y.len()
        {
            return Err("invalid dataset");
        }
        let mut history = Vec::new();
        for _ in 0..epochs {
            let mut tape = Tape::new();
            let p = tape.variables(&self.parameters);
            let mut losses = Vec::new();
            for (row, target) in x.iter().zip(y) {
                let input = tape.variables(row);
                let prediction = self.forward(&mut tape, &p, &input)?;
                losses.push(tape.mse(&prediction, target)?);
            }
            let sum = tape.sum(&losses);
            let loss = tape.scale(sum, 1.0 / x.len() as f64);
            history.push(tape.value(loss));
            tape.backward(loss)?;
            tape.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(history)
    }
}

pub struct Perceptron {
    pub weights: Vec<f64>,
    pub bias: f64,
}
impl Perceptron {
    pub fn fit(x: &[Vec<f64>], y: &[bool], epochs: usize, rate: f64) -> Result<Self> {
        let d = matrix_width(x)?;
        if y.len() != x.len() || !rate.is_finite() || rate <= 0.0 {
            return Err("invalid perceptron settings");
        }
        let mut model = Self {
            weights: vec![0.0; d],
            bias: 0.0,
        };
        for _ in 0..epochs {
            for (row, &target) in x.iter().zip(y) {
                let predicted = dot(row, &model.weights) + model.bias >= 0.0;
                let error = f64::from(target) - f64::from(predicted);
                for (w, v) in model.weights.iter_mut().zip(row) {
                    *w += rate * error * v;
                }
                model.bias += rate * error;
            }
        }
        Ok(model)
    }
    pub fn predict(&self, x: &[f64]) -> Result<bool> {
        if x.len() != self.weights.len() || x.iter().any(|v| !v.is_finite()) {
            return Err("invalid features");
        }
        Ok(dot(x, &self.weights) + self.bias >= 0.0)
    }
}

#[derive(Clone, Copy)]
pub enum RecurrentKind {
    Rnn,
    Lstm,
}
pub struct Recurrent {
    pub parameters: Vec<f64>,
    input: usize,
    hidden: usize,
    gates: Dense,
    readout: Dense,
    kind: RecurrentKind,
}
impl Recurrent {
    pub fn new(
        input: usize,
        hidden: usize,
        output: usize,
        kind: RecurrentKind,
        seed: u64,
    ) -> Result<Self> {
        if [input, hidden, output].contains(&0) {
            return Err("invalid recurrent dimensions");
        }
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let gates = Dense::new(
            &mut p,
            input + hidden,
            hidden
                * if matches!(kind, RecurrentKind::Lstm) {
                    4
                } else {
                    1
                },
            &mut rng,
        );
        let readout = Dense::new(&mut p, hidden, output, &mut rng);
        Ok(Self {
            parameters: p,
            input,
            hidden,
            gates,
            readout,
            kind,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], sequence: &[Vec<f64>]) -> Result<Vec<Var>> {
        if matrix_width(sequence)? != self.input || p.len() != self.parameters.len() {
            return Err("invalid sequence");
        }
        let mut h = t.variables(&vec![0.0; self.hidden]);
        let mut cell = h.clone();
        for row in sequence {
            let mut input = t.variables(row);
            input.extend_from_slice(&h);
            let gates = self.gates.forward(t, p, &input);
            if matches!(self.kind, RecurrentKind::Rnn) {
                h = gates.iter().map(|v| t.tanh(*v)).collect();
            } else {
                for j in 0..self.hidden {
                    let forget = t.sigmoid(gates[j]);
                    let update = t.sigmoid(gates[self.hidden + j]);
                    let candidate = t.tanh(gates[2 * self.hidden + j]);
                    let output = t.sigmoid(gates[3 * self.hidden + j]);
                    let old = t.mul(forget, cell[j]);
                    let new = t.mul(update, candidate);
                    cell[j] = t.add(old, new);
                    let activated = t.tanh(cell[j]);
                    h[j] = t.mul(output, activated);
                }
            }
        }
        Ok(self.readout.forward(t, p, &h))
    }
    pub fn train(
        &mut self,
        sequences: &[Vec<Vec<f64>>],
        targets: &[Vec<f64>],
        epochs: usize,
        rate: f64,
    ) -> Result<Vec<f64>> {
        if sequences.is_empty()
            || sequences.len() != targets.len()
            || matrix_width(targets)? != self.readout.output
        {
            return Err("invalid sequence targets");
        }
        let mut history = Vec::new();
        for _ in 0..epochs {
            let mut t = Tape::new();
            let p = t.variables(&self.parameters);
            let mut losses = Vec::new();
            for (seq, target) in sequences.iter().zip(targets) {
                let out = self.forward(&mut t, &p, seq)?;
                losses.push(t.mse(&out, target)?);
            }
            let sum = t.sum(&losses);
            let loss = t.scale(sum, 1.0 / sequences.len() as f64);
            history.push(t.value(loss));
            t.backward(loss)?;
            t.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(history)
    }
}

pub struct Gan {
    pub generator: Mlp,
    pub discriminator: Mlp,
}
impl Gan {
    pub fn new(seed: u64) -> Result<Self> {
        Ok(Self {
            generator: Mlp::new(&[1, 4, 1], Activation::Tanh, Activation::Linear, seed)?,
            discriminator: Mlp::new(&[1, 4, 1], Activation::Tanh, Activation::Sigmoid, seed + 1)?,
        })
    }
    pub fn train_step(&mut self, real: &[f64], noise: &[f64], rate: f64) -> Result<(f64, f64)> {
        if real.is_empty()
            || real.len() != noise.len()
            || real.iter().chain(noise).any(|v| !v.is_finite())
        {
            return Err("invalid GAN batch");
        }
        let mut t = Tape::new();
        let dp = t.variables(&self.discriminator.parameters);
        let mut terms = Vec::new();
        for (&x, &z) in real.iter().zip(noise) {
            let real_input = t.variables(&[x]);
            let real_score = self.discriminator.forward(&mut t, &dp, &real_input)?[0];
            let fake = self.generator.predict(&[z])?;
            let fake_input = t.variables(&fake);
            let fake_score = self.discriminator.forward(&mut t, &dp, &fake_input)?[0];
            let r = t.offset(real_score, 1e-12);
            let r = t.log(r);
            let f = t.scale(fake_score, -1.0);
            let f = t.offset(f, 1.0 + 1e-12);
            let f = t.log(f);
            terms.push(t.add(r, f));
        }
        let sum = t.sum(&terms);
        let loss = t.scale(sum, -1.0 / real.len() as f64);
        let d_loss = t.value(loss);
        t.backward(loss)?;
        t.sgd(&mut self.discriminator.parameters, &dp, rate)?;
        let mut t = Tape::new();
        let gp = t.variables(&self.generator.parameters);
        let dp = t.variables(&self.discriminator.parameters);
        let mut terms = Vec::new();
        for &z in noise {
            let input = t.variables(&[z]);
            let fake = self.generator.forward(&mut t, &gp, &input)?;
            let score = self.discriminator.forward(&mut t, &dp, &fake)?[0];
            let score = t.offset(score, 1e-12);
            terms.push(t.log(score));
        }
        let sum = t.sum(&terms);
        let loss = t.scale(sum, -1.0 / noise.len() as f64);
        let g_loss = t.value(loss);
        t.backward(loss)?;
        t.sgd(&mut self.generator.parameters, &gp, rate)?;
        Ok((d_loss, g_loss))
    }
}
