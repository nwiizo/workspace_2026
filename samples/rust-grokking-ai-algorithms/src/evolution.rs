//! Chapters 4–5: encodings, selection, crossover, mutation and complete searches.
use crate::{Result, random::Rng};

#[derive(Clone, Copy)]
pub enum Selection {
    Roulette,
    Rank,
    Tournament(usize),
    Elite,
}

pub fn select(scores: &[f64], method: Selection, rng: &mut Rng) -> Result<usize> {
    if scores.is_empty() || scores.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err("selection needs nonnegative finite scores");
    }
    let best = || {
        (0..scores.len())
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .expect("nonempty")
    };
    match method {
        Selection::Elite => Ok(best()),
        Selection::Roulette => rng.weighted(scores).ok_or("invalid weights"),
        Selection::Rank => {
            let mut order: Vec<_> = (0..scores.len()).collect();
            order.sort_by(|&a, &b| scores[a].total_cmp(&scores[b]));
            let rank: Vec<_> = (1..=scores.len()).map(|v| v as f64).collect();
            Ok(order[rng.weighted(&rank).expect("positive ranks")])
        }
        Selection::Tournament(size) => {
            if size == 0 {
                return Err("empty tournament");
            }
            let mut winner = rng.index(scores.len()).expect("nonempty");
            for _ in 1..size {
                let challenger = rng.index(scores.len()).expect("nonempty");
                if scores[challenger] > scores[winner] {
                    winner = challenger;
                }
            }
            Ok(winner)
        }
    }
}

/// [start,end) comes from b; the rest comes from a. start=0 gives one-point crossover.
pub fn crossover<T: Clone>(a: &[T], b: &[T], start: usize, end: usize) -> Result<Vec<T>> {
    if a.len() != b.len() || start > end || end > a.len() {
        return Err("invalid crossover range");
    }
    Ok(a.iter()
        .enumerate()
        .map(|(i, x)| {
            if (start..end).contains(&i) {
                b[i].clone()
            } else {
                x.clone()
            }
        })
        .collect())
}
pub fn uniform_crossover<T: Clone>(a: &[T], b: &[T], rng: &mut Rng) -> Result<Vec<T>> {
    if a.len() != b.len() {
        return Err("parent sizes differ");
    }
    Ok(a.iter()
        .zip(b)
        .map(|(x, y)| {
            if rng.uniform() < 0.5 {
                x.clone()
            } else {
                y.clone()
            }
        })
        .collect())
}
pub fn arithmetic_crossover(a: &[f64], b: &[f64], alpha: f64) -> Result<Vec<f64>> {
    if a.len() != b.len()
        || !(0.0..=1.0).contains(&alpha)
        || a.iter().chain(b).any(|x| !x.is_finite())
    {
        return Err("invalid real parents");
    }
    Ok(a.iter()
        .zip(b)
        .map(|(x, y)| alpha * x + (1.0 - alpha) * y)
        .collect())
}
pub fn bit_string_mutation(bits: &mut [bool], rng: &mut Rng) -> Result<()> {
    let i = rng.index(bits.len()).ok_or("empty chromosome")?;
    bits[i] = !bits[i];
    Ok(())
}
/// Chapter 4's flip-bit variant complements every gene.
pub fn flip_bit_mutation(bits: &mut [bool]) {
    for x in bits {
        *x = !*x;
    }
}
pub fn order_mutation<T>(order: &mut [T], rng: &mut Rng) -> Result<()> {
    if order.len() < 2 {
        return Err("order needs two genes");
    }
    let a = rng.index(order.len()).expect("nonempty");
    let b = (a + 1 + rng.index(order.len() - 1).expect("nonempty")) % order.len();
    order.swap(a, b);
    Ok(())
}
pub fn real_mutation(
    values: &mut [f64],
    bounds: (f64, f64),
    boundary: bool,
    rng: &mut Rng,
) -> Result<()> {
    if !bounds.0.is_finite()
        || !bounds.1.is_finite()
        || bounds.0 >= bounds.1
        || values.iter().any(|v| !v.is_finite())
    {
        return Err("invalid real chromosome");
    }
    let i = rng.index(values.len()).ok_or("empty chromosome")?;
    values[i] = if boundary {
        if rng.uniform() < 0.5 {
            bounds.0
        } else {
            bounds.1
        }
    } else {
        (values[i] + 0.1 * (bounds.1 - bounds.0) * rng.normal()).clamp(bounds.0, bounds.1)
    };
    Ok(())
}

#[derive(Clone, Copy)]
pub enum PermutationCrossover {
    Order,
    PartiallyMapped,
    Cycle,
}

pub fn permutation_crossover(
    a: &[usize],
    b: &[usize],
    range: std::ops::Range<usize>,
    method: PermutationCrossover,
) -> Result<Vec<usize>> {
    let mut sa = a.to_vec();
    sa.sort_unstable();
    let mut sb = b.to_vec();
    sb.sort_unstable();
    if a.is_empty()
        || sa != sb
        || sa.windows(2).any(|v| v[0] == v[1])
        || range.start >= range.end
        || range.end > a.len()
    {
        return Err("invalid permutations or range");
    }
    let n = a.len();
    let mut child = vec![None; n];
    match method {
        PermutationCrossover::Order => {
            for i in range.clone() {
                child[i] = Some(a[i]);
            }
            let mut position = range.end % n;
            for offset in 0..n {
                let gene = b[(range.end + offset) % n];
                if child.contains(&Some(gene)) {
                    continue;
                }
                while child[position].is_some() {
                    position = (position + 1) % n;
                }
                child[position] = Some(gene);
            }
        }
        PermutationCrossover::PartiallyMapped => {
            for i in range.clone() {
                child[i] = Some(a[i]);
            }
            for i in range.clone() {
                if a[range.clone()].contains(&b[i]) {
                    continue;
                }
                let mut j = i;
                while range.contains(&j) {
                    j = b.iter().position(|v| *v == a[j]).expect("same genes");
                }
                child[j] = Some(b[i]);
            }
            for i in 0..n {
                if child[i].is_none() {
                    child[i] = Some(b[i]);
                }
            }
        }
        PermutationCrossover::Cycle => {
            let mut cycle = 0;
            for start in 0..n {
                if child[start].is_some() {
                    continue;
                }
                let mut j = start;
                loop {
                    child[j] = Some(if cycle % 2 == 0 { a[j] } else { b[j] });
                    j = a.iter().position(|v| *v == b[j]).expect("same genes");
                    if j == start {
                        break;
                    }
                }
                cycle += 1;
            }
        }
    }
    Ok(child
        .into_iter()
        .map(|v| v.expect("all permutation positions filled"))
        .collect())
}

#[derive(Clone, Copy)]
pub struct GeneticConfig {
    pub population: usize,
    pub generations: usize,
    pub mutation: f64,
}

pub fn binary_genetic(
    width: usize,
    config: GeneticConfig,
    seed: u64,
    fitness: impl Fn(&[bool]) -> f64,
) -> Result<(Vec<bool>, f64)> {
    binary_genetic_population(width, config, PopulationModel::Elitist(1), seed, fitness)
}

#[derive(Clone, Copy)]
pub enum PopulationModel {
    Generational,
    SteadyState(usize),
    Elitist(usize),
}

pub fn binary_genetic_population(
    width: usize,
    config: GeneticConfig,
    model: PopulationModel,
    seed: u64,
    fitness: impl Fn(&[bool]) -> f64,
) -> Result<(Vec<bool>, f64)> {
    if width == 0 || config.population < 2 || !(0.0..=1.0).contains(&config.mutation) {
        return Err("invalid genetic settings");
    }
    let keep = match model {
        PopulationModel::Generational => 0,
        PopulationModel::SteadyState(replacements)
            if replacements > 0 && replacements <= config.population =>
        {
            config.population - replacements
        }
        PopulationModel::Elitist(survivors) if survivors < config.population => survivors,
        _ => return Err("invalid population replacement count"),
    };
    let mut rng = Rng::new(seed);
    let mut population: Vec<Vec<bool>> = (0..config.population)
        .map(|_| (0..width).map(|_| rng.uniform() < 0.5).collect())
        .collect();
    population[0].fill(false);
    for _ in 0..config.generations {
        let scores: Vec<_> = population.iter().map(|x| fitness(x)).collect();
        select(&scores, Selection::Elite, &mut rng)?;
        let mut order: Vec<_> = (0..population.len()).collect();
        order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        let mut next: Vec<_> = order[..keep]
            .iter()
            .map(|&i| population[i].clone())
            .collect();
        while next.len() < config.population {
            let a = select(&scores, Selection::Tournament(3), &mut rng)?;
            let b = select(&scores, Selection::Tournament(3), &mut rng)?;
            let cut = rng.index(width + 1).expect("positive");
            let mut child = crossover(&population[a], &population[b], 0, cut)?;
            for gene in &mut child {
                if rng.uniform() < config.mutation {
                    *gene = !*gene;
                }
            }
            next.push(child);
        }
        population = next;
    }
    let scores: Vec<_> = population.iter().map(|x| fitness(x)).collect();
    let best = select(&scores, Selection::Elite, &mut rng)?;
    Ok((population[best].clone(), scores[best]))
}

pub fn knapsack(
    items: &[(u32, u32)],
    capacity: u64,
    config: GeneticConfig,
    seed: u64,
) -> Result<(Vec<bool>, f64)> {
    binary_genetic(items.len(), config, seed, |bits| {
        let (weight, value) = items
            .iter()
            .zip(bits)
            .filter(|(_, chosen)| **chosen)
            .fold((0u64, 0u64), |(w, v), ((iw, iv), _)| {
                (w + u64::from(*iw), v + u64::from(*iv))
            });
        if weight > capacity { 0.0 } else { value as f64 }
    })
}

/// Mutation-and-selection evolutionary programming; can optimize ANN weights too.
pub fn evolve_real(
    dimensions: usize,
    bounds: (f64, f64),
    generations: usize,
    seed: u64,
    loss: impl Fn(&[f64]) -> f64,
) -> Result<(Vec<f64>, f64)> {
    if dimensions == 0 || !bounds.0.is_finite() || !bounds.1.is_finite() || bounds.0 >= bounds.1 {
        return Err("invalid search bounds");
    }
    let mut rng = Rng::new(seed);
    let mut best: Vec<_> = (0..dimensions)
        .map(|_| bounds.0 + (bounds.1 - bounds.0) * rng.uniform())
        .collect();
    let mut score = loss(&best);
    if !score.is_finite() {
        return Err("nonfinite fitness");
    }
    for generation in 0..generations {
        let scale = (bounds.1 - bounds.0) * 0.2 / (1.0 + generation as f64 * 0.03);
        for _ in 0..24 {
            let candidate: Vec<_> = best
                .iter()
                .map(|v| (v + scale * rng.normal()).clamp(bounds.0, bounds.1))
                .collect();
            let value = loss(&candidate);
            if !value.is_finite() {
                return Err("nonfinite fitness");
            }
            if value < score {
                score = value;
                best = candidate;
            }
        }
    }
    Ok((best, score))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    X,
    Constant(f64),
    Add(Box<Expr>, Box<Expr>),
    Multiply(Box<Expr>, Box<Expr>),
}
impl Expr {
    pub fn depth(&self) -> usize {
        match self {
            Self::Add(a, b) | Self::Multiply(a, b) => 1 + a.depth().max(b.depth()),
            _ => 0,
        }
    }
    pub fn evaluate(&self, x: f64) -> f64 {
        match self {
            Self::X => x,
            Self::Constant(v) => *v,
            Self::Add(a, b) => a.evaluate(x) + b.evaluate(x),
            Self::Multiply(a, b) => a.evaluate(x) * b.evaluate(x),
        }
    }
    pub fn subtree(&self, path: &[bool]) -> Option<&Self> {
        if path.is_empty() {
            return Some(self);
        }
        match self {
            Self::Add(a, b) | Self::Multiply(a, b) => {
                if path[0] { b } else { a }.subtree(&path[1..])
            }
            _ => None,
        }
    }
    pub fn replace(&mut self, path: &[bool], replacement: Self) -> Result<()> {
        if path.is_empty() {
            *self = replacement;
            return Ok(());
        }
        match self {
            Self::Add(a, b) | Self::Multiply(a, b) => {
                if path[0] { b } else { a }.replace(&path[1..], replacement)
            }
            _ => Err("invalid tree path"),
        }
    }
    pub fn change_node(&mut self, path: &[bool]) -> Result<()> {
        let replacement = match self.subtree(path).ok_or("invalid tree path")? {
            Self::X => Self::Constant(1.0),
            Self::Constant(_) => Self::X,
            Self::Add(a, b) => Self::Multiply(a.clone(), b.clone()),
            Self::Multiply(a, b) => Self::Add(a.clone(), b.clone()),
        };
        self.replace(path, replacement)
    }
}
pub fn tree_crossover(a: &Expr, b: &Expr, path_a: &[bool], path_b: &[bool]) -> Result<Expr> {
    let mut child = a.clone();
    child.replace(
        path_a,
        b.subtree(path_b).ok_or("invalid tree path")?.clone(),
    )?;
    Ok(child)
}
pub fn genetic_programming(
    data: &[(f64, f64)],
    generations: usize,
    seed: u64,
) -> Result<(Expr, f64)> {
    if data.is_empty() || data.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err("invalid regression examples");
    }
    fn random_expr(rng: &mut Rng, depth: usize) -> Expr {
        if depth == 0 || rng.uniform() < 0.3 {
            return if rng.uniform() < 0.5 {
                Expr::X
            } else {
                Expr::Constant((rng.uniform() * 5.0).floor() - 2.0)
            };
        }
        let a = Box::new(random_expr(rng, depth - 1));
        let b = Box::new(random_expr(rng, depth - 1));
        if rng.uniform() < 0.5 {
            Expr::Add(a, b)
        } else {
            Expr::Multiply(a, b)
        }
    }
    let error = |e: &Expr| {
        data.iter()
            .map(|(x, y)| (e.evaluate(*x) - y).powi(2))
            .sum::<f64>()
            / data.len() as f64
    };
    let mut rng = Rng::new(seed);
    let mut population: Vec<_> = (0..64).map(|_| random_expr(&mut rng, 3)).collect();
    for _ in 0..generations {
        population.sort_by(|a, b| error(a).total_cmp(&error(b)));
        let mut next = population[..8].to_vec();
        while next.len() < 64 {
            let a = &population[rng.index(16).expect("positive")];
            let mut child = a.clone();
            let path = if matches!(child, Expr::Add(..) | Expr::Multiply(..)) {
                vec![rng.uniform() < 0.5]
            } else {
                vec![]
            };
            let donor = if rng.uniform() < 0.5 {
                random_expr(&mut rng, 2)
            } else {
                population[rng.index(16).expect("positive")].clone()
            };
            child.replace(&path, donor)?;
            if child.depth() <= 6 && error(&child).is_finite() {
                next.push(child);
            } else {
                next.push(random_expr(&mut rng, 2));
            }
        }
        population = next;
    }
    population.sort_by(|a, b| error(a).total_cmp(&error(b)));
    Ok((population[0].clone(), error(&population[0])))
}

pub fn genetic_order(
    length: usize,
    config: GeneticConfig,
    seed: u64,
    cost: impl Fn(&[usize]) -> f64,
) -> Result<(Vec<usize>, f64)> {
    if length < 2 || config.population < 2 || !(0.0..=1.0).contains(&config.mutation) {
        return Err("invalid permutation search settings");
    }
    let mut rng = Rng::new(seed);
    let mut population: Vec<Vec<_>> = (0..config.population)
        .map(|_| {
            let mut route = (0..length).collect::<Vec<_>>();
            rng.shuffle(&mut route);
            route
        })
        .collect();
    for _ in 0..config.generations {
        let costs: Vec<_> = population.iter().map(|r| cost(r)).collect();
        if costs.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err("invalid route cost");
        }
        let scores: Vec<_> = costs.iter().map(|v| 1.0 / (1.0 + v)).collect();
        let elite = select(&scores, Selection::Elite, &mut rng)?;
        let mut next = vec![population[elite].clone()];
        while next.len() < config.population {
            let a = select(&scores, Selection::Tournament(3), &mut rng)?;
            let b = select(&scores, Selection::Tournament(3), &mut rng)?;
            let start = rng.index(length).expect("positive");
            let end = start + 1 + rng.index(length - start).expect("positive");
            let mut child = permutation_crossover(
                &population[a],
                &population[b],
                start..end,
                PermutationCrossover::Order,
            )?;
            if rng.uniform() < config.mutation {
                order_mutation(&mut child, &mut rng)?;
            }
            next.push(child);
        }
        population = next;
    }
    let mut scored: Vec<_> = population
        .into_iter()
        .map(|r| {
            let value = cost(&r);
            (r, value)
        })
        .collect();
    if scored.iter().any(|r| !r.1.is_finite() || r.1 < 0.0) {
        return Err("invalid route cost");
    }
    scored.sort_by(|a, b| a.1.total_cmp(&b.1));
    Ok(scored.remove(0))
}
