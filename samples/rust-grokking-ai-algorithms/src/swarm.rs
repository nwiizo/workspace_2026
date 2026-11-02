use crate::{Result, random::Rng};

pub fn route_length(distances: &[Vec<f64>], route: &[usize]) -> Result<f64> {
    if route.is_empty()
        || distances
            .iter()
            .any(|r| r.len() != distances.len() || r.iter().any(|v| !v.is_finite() || *v < 0.0))
        || route.iter().any(|i| *i >= distances.len())
    {
        return Err("invalid route");
    }
    Ok((0..route.len())
        .map(|i| distances[route[i]][route[(i + 1) % route.len()]])
        .sum())
}
#[derive(Clone, Copy)]
pub struct AntConfig {
    pub ants: usize,
    pub iterations: usize,
    pub evaporation: f64,
    pub alpha: f64,
    pub beta: f64,
}
pub fn ant_colony(
    distances: &[Vec<f64>],
    config: AntConfig,
    seed: u64,
) -> Result<(Vec<usize>, f64)> {
    let n = distances.len();
    if n < 2
        || config.ants == 0
        || config.iterations == 0
        || !(0.0..1.0).contains(&config.evaporation)
        || !config.alpha.is_finite()
        || config.alpha < 0.0
        || !config.beta.is_finite()
        || config.beta < 0.0
        || distances.iter().enumerate().any(|(i, row)| {
            row.len() != n
                || row
                    .iter()
                    .enumerate()
                    .any(|(j, v)| !v.is_finite() || (i != j && *v <= 0.0))
        })
    {
        return Err("invalid ant settings/distances");
    }
    let mut rng = Rng::new(seed);
    let mut trails = vec![vec![1.0_f64; n]; n];
    let mut best = (vec![], f64::INFINITY);
    for _ in 0..config.iterations {
        let mut tours = Vec::new();
        for _ in 0..config.ants {
            let mut tour = vec![rng.index(n).expect("positive")];
            while tour.len() < n {
                let last = tour[tour.len() - 1];
                let candidates: Vec<_> = (0..n).filter(|j| !tour.contains(j)).collect();
                // Log-space probabilities avoid overflow of pheromone^alpha / distance^beta.
                let logits: Vec<_> = candidates
                    .iter()
                    .map(|&j| {
                        config.alpha * trails[last][j].max(1e-200).ln()
                            - config.beta * distances[last][j].ln()
                    })
                    .collect();
                let max = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let weights: Vec<_> = logits.iter().map(|v| (v - max).exp()).collect();
                tour.push(
                    candidates[rng
                        .weighted(&weights)
                        .ok_or("invalid transition probabilities")?],
                );
            }
            let length = route_length(distances, &tour)?;
            if length < best.1 {
                best = (tour.clone(), length);
            }
            tours.push((tour, length));
        }
        for row in &mut trails {
            for value in row {
                *value *= 1.0 - config.evaporation;
            }
        }
        for (tour, length) in tours {
            for i in 0..n {
                trails[tour[i]][tour[(i + 1) % n]] += 1.0 / length;
            }
        }
    }
    Ok(best)
}

#[derive(Clone, Copy)]
pub struct ParticleConfig {
    pub particles: usize,
    pub iterations: usize,
    pub inertia: f64,
    pub cognitive: f64,
    pub social: f64,
}
pub fn particle_swarm(
    bounds: &[(f64, f64)],
    config: ParticleConfig,
    seed: u64,
    objective: impl Fn(&[f64]) -> f64,
) -> Result<(Vec<f64>, f64)> {
    if bounds.is_empty()
        || config.particles == 0
        || config.iterations == 0
        || [config.inertia, config.cognitive, config.social]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        || bounds
            .iter()
            .any(|(a, b)| !a.is_finite() || !b.is_finite() || a >= b || !(b - a).is_finite())
    {
        return Err("invalid particle settings");
    }
    let mut rng = Rng::new(seed);
    let mut positions: Vec<Vec<f64>> = (0..config.particles)
        .map(|_| {
            bounds
                .iter()
                .map(|(a, b)| a + rng.uniform() * (b - a))
                .collect()
        })
        .collect();
    let mut velocities = vec![vec![0.0; bounds.len()]; config.particles];
    let mut personal = positions.clone();
    let mut scores: Vec<_> = positions.iter().map(|p| objective(p)).collect();
    if scores.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite objective");
    }
    let best_index = (0..scores.len())
        .min_by(|&a, &b| scores[a].total_cmp(&scores[b]))
        .expect("positive");
    let mut global = (personal[best_index].clone(), scores[best_index]);
    for _ in 0..config.iterations {
        for i in 0..config.particles {
            for (d, &(low, high)) in bounds.iter().enumerate() {
                let velocity = config.inertia * velocities[i][d]
                    + config.cognitive * rng.uniform() * (personal[i][d] - positions[i][d])
                    + config.social * rng.uniform() * (global.0[d] - positions[i][d]);
                velocities[i][d] = velocity.clamp(-(high - low), high - low);
                positions[i][d] = (positions[i][d] + velocities[i][d]).clamp(low, high);
            }
            let score = objective(&positions[i]);
            if !score.is_finite() {
                return Err("nonfinite objective");
            }
            if score < scores[i] {
                scores[i] = score;
                personal[i] = positions[i].clone();
            }
            if score < global.1 {
                global = (positions[i].clone(), score);
            }
        }
    }
    Ok(global)
}
pub fn booth(x: f64, y: f64) -> f64 {
    (x + 2.0 * y - 7.0).powi(2) + (2.0 * x + y - 5.0).powi(2)
}
