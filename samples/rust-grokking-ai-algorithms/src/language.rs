use crate::{
    Result,
    autodiff::{Tape, Var},
    neural::Dense,
    numeric::softmax,
    random::Rng,
};
use std::collections::BTreeMap;

pub struct BigramModel {
    counts: Vec<Vec<f64>>,
}
impl BigramModel {
    pub fn train(tokens: &[usize], vocabulary: usize) -> Result<Self> {
        if vocabulary == 0 || tokens.len() < 2 || tokens.iter().any(|v| *v >= vocabulary) {
            return Err("invalid bigram corpus");
        }
        let mut counts = vec![vec![1.0; vocabulary]; vocabulary];
        for pair in tokens.windows(2) {
            counts[pair[0]][pair[1]] += 1.0;
        }
        Ok(Self { counts })
    }
    pub fn probabilities(&self, previous: usize) -> Result<Vec<f64>> {
        let row = self.counts.get(previous).ok_or("unknown token")?;
        let sum: f64 = row.iter().sum();
        Ok(row.iter().map(|v| v / sum).collect())
    }
}

/// Byte-level BPE: all UTF-8 bytes are representable, merges are learned then replayed.
pub struct Bpe {
    vocabulary: Vec<Vec<u8>>,
    merges: Vec<(usize, usize, usize)>,
}
impl Bpe {
    pub fn train(text: &str, merges: usize) -> Self {
        let mut model = Self {
            vocabulary: (0..=255).map(|b| vec![b]).collect(),
            merges: Vec::new(),
        };
        let mut ids: Vec<_> = text.as_bytes().iter().map(|&b| usize::from(b)).collect();
        for _ in 0..merges {
            let mut counts = BTreeMap::new();
            for pair in ids.windows(2) {
                *counts.entry((pair[0], pair[1])).or_insert(0usize) += 1;
            }
            let Some((&(a, b), &count)) = counts
                .iter()
                .max_by_key(|(pair, count)| (**count, std::cmp::Reverse(**pair)))
            else {
                break;
            };
            if count < 2 {
                break;
            }
            let mut token = model.vocabulary[a].clone();
            token.extend_from_slice(&model.vocabulary[b]);
            let id = model.vocabulary.len();
            model.vocabulary.push(token);
            model.merges.push((a, b, id));
            ids = merge_pair(&ids, a, b, id);
        }
        model
    }
    pub fn vocabulary_size(&self) -> usize {
        self.vocabulary.len()
    }
    pub fn encode(&self, text: &str) -> Vec<usize> {
        let mut ids: Vec<_> = text.as_bytes().iter().map(|&b| usize::from(b)).collect();
        for &(a, b, id) in &self.merges {
            ids = merge_pair(&ids, a, b, id);
        }
        ids
    }
    pub fn decode(&self, ids: &[usize]) -> Result<String> {
        let mut bytes = Vec::new();
        for &id in ids {
            bytes.extend_from_slice(self.vocabulary.get(id).ok_or("unknown token")?);
        }
        String::from_utf8(bytes).map_err(|_| "tokens do not form UTF-8")
    }
}
fn merge_pair(ids: &[usize], a: usize, b: usize, id: usize) -> Vec<usize> {
    let mut output = Vec::new();
    let mut i = 0;
    while i < ids.len() {
        if i + 1 < ids.len() && ids[i] == a && ids[i + 1] == b {
            output.push(id);
            i += 2;
        } else {
            output.push(ids[i]);
            i += 1;
        }
    }
    output
}
pub fn token_batches(tokens: &[usize], context: usize) -> Result<Vec<(Vec<usize>, Vec<usize>)>> {
    if context == 0 || context >= tokens.len() {
        return Err("context must have a next token");
    }
    Ok(tokens
        .windows(context + 1)
        .map(|w| (w[..context].to_vec(), w[1..].to_vec()))
        .collect())
}
pub fn positional_encoding(position: usize, width: usize) -> Vec<f64> {
    (0..width)
        .map(|i| {
            let angle = position as f64 / 10000.0_f64.powf(2.0 * (i / 2) as f64 / width as f64);
            if i % 2 == 0 { angle.sin() } else { angle.cos() }
        })
        .collect()
}
pub fn attention(
    t: &mut Tape,
    q: &[Vec<Var>],
    k: &[Vec<Var>],
    v: &[Vec<Var>],
    causal: bool,
) -> Result<Vec<Vec<Var>>> {
    let d = q.first().ok_or("empty queries")?.len();
    let value_width = v.first().ok_or("empty values")?.len();
    if d == 0
        || value_width == 0
        || k.len() != v.len()
        || q.iter().chain(k).any(|r| r.len() != d)
        || v.iter().any(|r| r.len() != value_width)
        || (causal && q.len() != k.len())
    {
        return Err("invalid attention shapes");
    }
    let mut output = Vec::new();
    for (i, query) in q.iter().enumerate() {
        let end = if causal { i + 1 } else { k.len() };
        let scores: Vec<_> = k[..end]
            .iter()
            .map(|key| {
                let score = t.dot(query, key);
                t.scale(score, 1.0 / (d as f64).sqrt())
            })
            .collect();
        let weights = t.softmax(&scores);
        let row = (0..value_width)
            .map(|j| {
                let column: Vec<_> = v[..end].iter().map(|r| r[j]).collect();
                t.dot(&weights, &column)
            })
            .collect();
        output.push(row);
    }
    Ok(output)
}
pub fn layer_norm(t: &mut Tape, x: &[Var], gamma: &[Var], beta: &[Var]) -> Result<Vec<Var>> {
    if x.is_empty() || x.len() != gamma.len() || x.len() != beta.len() {
        return Err("invalid layer norm shapes");
    }
    let sum = t.sum(x);
    let mean = t.scale(sum, 1.0 / x.len() as f64);
    let centered: Vec<_> = x.iter().map(|v| t.sub(*v, mean)).collect();
    let squares: Vec<_> = centered.iter().map(|v| t.mul(*v, *v)).collect();
    let sum = t.sum(&squares);
    let variance = t.scale(sum, 1.0 / x.len() as f64);
    let stabilized = t.offset(variance, 1e-5);
    let inverse = t.powf(stabilized, -0.5);
    Ok(centered
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let normalized = t.mul(*v, inverse);
            let scaled = t.mul(normalized, gamma[i]);
            t.add(scaled, beta[i])
        })
        .collect())
}
#[derive(Clone, Copy)]
pub struct TransformerConfig {
    pub vocabulary: usize,
    pub width: usize,
    pub heads: usize,
    pub feed_forward: usize,
    pub layers: usize,
    pub context: usize,
}
struct Block {
    query: Dense,
    key: Dense,
    value: Dense,
    project: Dense,
    up: Dense,
    down: Dense,
    norm1: usize,
    norm2: usize,
}
pub struct Transformer {
    pub parameters: Vec<f64>,
    pub config: TransformerConfig,
    blocks: Vec<Block>,
    readout: Dense,
}
impl Transformer {
    pub fn new(config: TransformerConfig, seed: u64) -> Result<Self> {
        let c = config;
        if [
            c.vocabulary,
            c.width,
            c.heads,
            c.feed_forward,
            c.layers,
            c.context,
        ]
        .contains(&0)
            || c.width % c.heads != 0
        {
            return Err("invalid transformer dimensions");
        }
        let mut rng = Rng::new(seed);
        let mut p: Vec<_> = (0..c.vocabulary * c.width)
            .map(|_| 0.1 * rng.normal())
            .collect();
        let mut blocks = Vec::new();
        for _ in 0..c.layers {
            let query = Dense::new(&mut p, c.width, c.width, &mut rng);
            let key = Dense::new(&mut p, c.width, c.width, &mut rng);
            let value = Dense::new(&mut p, c.width, c.width, &mut rng);
            let project = Dense::new(&mut p, c.width, c.width, &mut rng);
            let up = Dense::new(&mut p, c.width, c.feed_forward, &mut rng);
            let down = Dense::new(&mut p, c.feed_forward, c.width, &mut rng);
            let norm1 = p.len();
            p.extend(vec![1.0; c.width]);
            p.extend(vec![0.0; c.width]);
            let norm2 = p.len();
            p.extend(vec![1.0; c.width]);
            p.extend(vec![0.0; c.width]);
            blocks.push(Block {
                query,
                key,
                value,
                project,
                up,
                down,
                norm1,
                norm2,
            });
        }
        let readout = Dense::new(&mut p, c.width, c.vocabulary, &mut rng);
        Ok(Self {
            parameters: p,
            config,
            blocks,
            readout,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], tokens: &[usize]) -> Result<Vec<Vec<Var>>> {
        let c = self.config;
        if p.len() != self.parameters.len()
            || tokens.is_empty()
            || tokens.len() > c.context
            || tokens.iter().any(|id| *id >= c.vocabulary)
        {
            return Err("invalid transformer input");
        }
        let mut x: Vec<Vec<_>> = tokens
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                positional_encoding(i, c.width)
                    .iter()
                    .enumerate()
                    .map(|(j, position)| t.offset(p[id * c.width + j], *position))
                    .collect()
            })
            .collect();
        for block in &self.blocks {
            let queries: Vec<_> = x.iter().map(|r| block.query.forward(t, p, r)).collect();
            let keys: Vec<_> = x.iter().map(|r| block.key.forward(t, p, r)).collect();
            let values: Vec<_> = x.iter().map(|r| block.value.forward(t, p, r)).collect();
            let mut concatenated = vec![Vec::new(); x.len()];
            let head_width = c.width / c.heads;
            for head in 0..c.heads {
                let range = head * head_width..(head + 1) * head_width;
                let q: Vec<_> = queries.iter().map(|r| r[range.clone()].to_vec()).collect();
                let k: Vec<_> = keys.iter().map(|r| r[range.clone()].to_vec()).collect();
                let v: Vec<_> = values.iter().map(|r| r[range.clone()].to_vec()).collect();
                let attended = attention(t, &q, &k, &v, true)?;
                for (row, part) in concatenated.iter_mut().zip(attended) {
                    row.extend(part);
                }
            }
            for (row, attended) in x.iter_mut().zip(concatenated) {
                let projected = block.project.forward(t, p, &attended);
                let residual: Vec<_> = row
                    .iter()
                    .zip(projected)
                    .map(|(a, b)| t.add(*a, b))
                    .collect();
                let n = block.norm1;
                *row = layer_norm(
                    t,
                    &residual,
                    &p[n..n + c.width],
                    &p[n + c.width..n + 2 * c.width],
                )?;
                let hidden = block.up.forward(t, p, row);
                let hidden: Vec<_> = hidden.iter().map(|v| t.relu(*v)).collect();
                let projected = block.down.forward(t, p, &hidden);
                let residual: Vec<_> = row
                    .iter()
                    .zip(projected)
                    .map(|(a, b)| t.add(*a, b))
                    .collect();
                let n = block.norm2;
                *row = layer_norm(
                    t,
                    &residual,
                    &p[n..n + c.width],
                    &p[n + c.width..n + 2 * c.width],
                )?;
            }
        }
        Ok(x.iter()
            .map(|row| self.readout.forward(t, p, row))
            .collect())
    }
    pub fn loss(&self, tokens: &[usize], targets: &[usize]) -> Result<f64> {
        if tokens.len() != targets.len() {
            return Err("target length mismatch");
        }
        let mut t = Tape::new();
        let p = t.variables(&self.parameters);
        let logits = self.forward(&mut t, &p, tokens)?;
        let losses: Result<Vec<_>> = logits
            .iter()
            .zip(targets)
            .map(|(row, &target)| t.cross_entropy(row, target))
            .collect();
        Ok(losses?.iter().map(|v| t.value(*v)).sum::<f64>() / tokens.len() as f64)
    }
    pub fn train(
        &mut self,
        tokens: &[usize],
        steps: usize,
        rate: f64,
        seed: u64,
    ) -> Result<Vec<f64>> {
        let batches = token_batches(tokens, self.config.context)?;
        let mut rng = Rng::new(seed);
        let mut history = Vec::new();
        for _ in 0..steps {
            let (input, targets) = &batches[rng.index(batches.len()).expect("nonempty")];
            let mut t = Tape::new();
            let p = t.variables(&self.parameters);
            let logits = self.forward(&mut t, &p, input)?;
            let losses: Result<Vec<_>> = logits
                .iter()
                .zip(targets)
                .map(|(row, &target)| t.cross_entropy(row, target))
                .collect();
            let sum = t.sum(&losses?);
            let loss = t.scale(sum, 1.0 / targets.len() as f64);
            history.push(t.value(loss));
            t.backward(loss)?;
            t.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(history)
    }
    pub fn next_probabilities(&self, tokens: &[usize], temperature: f64) -> Result<Vec<f64>> {
        if !temperature.is_finite() || temperature <= 0.0 || tokens.is_empty() {
            return Err("invalid sampling input");
        }
        let context = &tokens[tokens.len().saturating_sub(self.config.context)..];
        let mut t = Tape::new();
        let p = t.variables(&self.parameters);
        let rows = self.forward(&mut t, &p, context)?;
        softmax(
            &rows[rows.len() - 1]
                .iter()
                .map(|v| t.value(*v) / temperature)
                .collect::<Vec<_>>(),
        )
    }
    pub fn generate(
        &self,
        prompt: &[usize],
        new_tokens: usize,
        temperature: f64,
        seed: u64,
    ) -> Result<Vec<usize>> {
        let mut rng = Rng::new(seed);
        let mut tokens = prompt.to_vec();
        for _ in 0..new_tokens {
            let p = self.next_probabilities(&tokens, temperature)?;
            tokens.push(rng.weighted(&p).ok_or("invalid probabilities")?);
        }
        Ok(tokens)
    }
    /// One-token REINFORCE policy update using a learned reward per response token.
    pub fn reinforce(
        &mut self,
        prompt: &[usize],
        rewards: &[f64],
        steps: usize,
        rate: f64,
        seed: u64,
    ) -> Result<()> {
        if rewards.len() != self.config.vocabulary || rewards.iter().any(|v| !v.is_finite()) {
            return Err("invalid rewards");
        }
        let mut rng = Rng::new(seed);
        for _ in 0..steps {
            let probs = self.next_probabilities(prompt, 1.0)?;
            let action = rng.weighted(&probs).ok_or("invalid policy")?;
            let baseline: f64 = probs.iter().zip(rewards).map(|(p, r)| p * r).sum();
            let mut t = Tape::new();
            let p = t.variables(&self.parameters);
            let logits = self.forward(&mut t, &p, prompt)?;
            let ce = t.cross_entropy(&logits[logits.len() - 1], action)?;
            let loss = t.scale(ce, rewards[action] - baseline);
            t.backward(loss)?;
            t.sgd(&mut self.parameters, &p, rate)?;
        }
        Ok(())
    }
}

pub struct MixtureOfExperts {
    pub parameters: Vec<f64>,
    gate: Dense,
    experts: Vec<Dense>,
    top_k: usize,
}
impl MixtureOfExperts {
    pub fn new(width: usize, experts: usize, top_k: usize, seed: u64) -> Result<Self> {
        if width == 0 || top_k == 0 || top_k > experts {
            return Err("invalid expert configuration");
        }
        let mut p = Vec::new();
        let mut rng = Rng::new(seed);
        let gate = Dense::new(&mut p, width, experts, &mut rng);
        let experts = (0..experts)
            .map(|_| Dense::new(&mut p, width, width, &mut rng))
            .collect();
        Ok(Self {
            parameters: p,
            gate,
            experts,
            top_k,
        })
    }
    pub fn forward(&self, t: &mut Tape, p: &[Var], x: &[Var]) -> Result<(Vec<Var>, Vec<usize>)> {
        if p.len() != self.parameters.len() || x.len() != self.gate.input {
            return Err("invalid expert input");
        }
        let scores = self.gate.forward(t, p, x);
        let mut order: Vec<_> = (0..scores.len()).collect();
        order.sort_by(|&a, &b| t.value(scores[b]).total_cmp(&t.value(scores[a])));
        order.truncate(self.top_k);
        let weights = t.softmax(&order.iter().map(|&i| scores[i]).collect::<Vec<_>>());
        let outputs: Vec<_> = order
            .iter()
            .map(|&i| self.experts[i].forward(t, p, x))
            .collect();
        let result = (0..x.len())
            .map(|j| {
                let column: Vec<_> = outputs.iter().map(|r| r[j]).collect();
                t.dot(&weights, &column)
            })
            .collect();
        Ok((result, order))
    }
}

/// Retrieval + prompt construction. Feed the result into the tokenizer and Transformer.
pub fn retrieval_prompt(
    documents: &[&str],
    question: &str,
    k: usize,
) -> Result<(Vec<usize>, String)> {
    if documents.is_empty() || k == 0 {
        return Err("empty retrieval corpus");
    }
    let words = |text: &str| {
        text.split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let query = words(question);
    let mut scored: Vec<_> = documents
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let terms = words(d);
            let score = query.iter().filter(|w| terms.contains(w)).count() as f64
                / ((query.len() * terms.len()).max(1) as f64).sqrt();
            (i, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let ids: Vec<_> = scored.iter().take(k).map(|v| v.0).collect();
    let context = ids
        .iter()
        .map(|&i| documents[i])
        .collect::<Vec<_>>()
        .join("\n");
    Ok((
        ids,
        format!("Context:\n{context}\nQuestion: {question}\nAnswer:"),
    ))
}

/// Bradley–Terry preference reward model shared by text and image examples.
pub fn fit_reward_model(
    chosen: &[Vec<f64>],
    rejected: &[Vec<f64>],
    epochs: usize,
    rate: f64,
) -> Result<Vec<f64>> {
    let d = crate::numeric::matrix_width(chosen)?;
    if crate::numeric::matrix_width(rejected)? != d
        || chosen.len() != rejected.len()
        || !rate.is_finite()
        || rate <= 0.0
    {
        return Err("invalid preference data");
    }
    let mut w = vec![0.0; d];
    for _ in 0..epochs {
        for (good, bad) in chosen.iter().zip(rejected) {
            let delta: Vec<_> = good.iter().zip(bad).map(|(a, b)| a - b).collect();
            let factor = crate::numeric::sigmoid(crate::numeric::dot(&w, &delta)) - 1.0;
            for (weight, difference) in w.iter_mut().zip(delta) {
                *weight -= rate * factor * difference / chosen.len() as f64;
            }
        }
    }
    Ok(w)
}
