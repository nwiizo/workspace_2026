use crate::{
    Result,
    numeric::{distance2, dot, matrix_width, symmetric_eigen},
    random::Rng,
};

pub fn k_means(
    x: &[Vec<f64>],
    k: usize,
    iterations: usize,
    seed: u64,
) -> Result<(Vec<Vec<f64>>, Vec<usize>)> {
    let d = matrix_width(x)?;
    if k == 0 || k > x.len() || iterations == 0 {
        return Err("invalid k-means settings");
    }
    let mut rng = Rng::new(seed);
    let mut centers = vec![x[rng.index(x.len()).expect("positive")].clone()];
    // k-means++ initialization.
    while centers.len() < k {
        let weights: Vec<_> = x
            .iter()
            .map(|r| {
                centers
                    .iter()
                    .map(|c| distance2(r, c))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();
        centers.push(x[rng.weighted(&weights).ok_or("invalid distances")?].clone());
    }
    let mut labels = vec![0; x.len()];
    for _ in 0..iterations {
        for (i, row) in x.iter().enumerate() {
            labels[i] = (0..k)
                .min_by(|&a, &b| {
                    distance2(row, &centers[a]).total_cmp(&distance2(row, &centers[b]))
                })
                .expect("positive");
        }
        let mut sums = vec![vec![0.0; d]; k];
        let mut counts = vec![0; k];
        for (row, &label) in x.iter().zip(&labels) {
            counts[label] += 1;
            for (s, v) in sums[label].iter_mut().zip(row) {
                *s += v;
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for (v, sum) in centers[c].iter_mut().zip(&sums[c]) {
                    *v = sum / counts[c] as f64;
                }
            }
        }
    }
    for (i, row) in x.iter().enumerate() {
        labels[i] = (0..k)
            .min_by(|&a, &b| distance2(row, &centers[a]).total_cmp(&distance2(row, &centers[b])))
            .expect("positive");
    }
    Ok((centers, labels))
}
pub fn mean_shift(
    x: &[Vec<f64>],
    bandwidth: f64,
    iterations: usize,
) -> Result<(Vec<Vec<f64>>, Vec<usize>)> {
    let d = matrix_width(x)?;
    if !bandwidth.is_finite() || bandwidth <= 0.0 || iterations == 0 {
        return Err("invalid bandwidth/iterations");
    }
    let mut centers: Vec<Vec<f64>> = Vec::new();
    let mut labels = Vec::new();
    for row in x {
        let mut point = row.clone();
        for _ in 0..iterations {
            let neighbors: Vec<_> = x
                .iter()
                .filter(|p| distance2(p, &point) <= bandwidth * bandwidth)
                .collect();
            if neighbors.is_empty() {
                break;
            }
            let next: Vec<_> = (0..d)
                .map(|j| neighbors.iter().map(|p| p[j]).sum::<f64>() / neighbors.len() as f64)
                .collect();
            let change = distance2(&point, &next);
            point = next;
            if change < 1e-12 {
                break;
            }
        }
        let label = if let Some(i) = centers
            .iter()
            .position(|c| distance2(c, &point) < bandwidth * bandwidth * 0.25)
        {
            i
        } else {
            centers.push(point);
            centers.len() - 1
        };
        labels.push(label);
    }
    Ok((centers, labels))
}
/// None denotes noise; min_points includes the point itself.
pub fn dbscan(x: &[Vec<f64>], epsilon: f64, min_points: usize) -> Result<Vec<Option<usize>>> {
    matrix_width(x)?;
    if !epsilon.is_finite() || epsilon <= 0.0 || min_points == 0 {
        return Err("invalid DBSCAN settings");
    }
    let neighbors = |i: usize| {
        (0..x.len())
            .filter(|&j| distance2(&x[i], &x[j]) <= epsilon * epsilon)
            .collect::<Vec<_>>()
    };
    let mut visited = vec![false; x.len()];
    let mut labels = vec![None; x.len()];
    let mut cluster = 0;
    for i in 0..x.len() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let mut queue = neighbors(i);
        if queue.len() < min_points {
            continue;
        }
        labels[i] = Some(cluster);
        let mut cursor = 0;
        let mut queued = vec![false; x.len()];
        for &j in &queue {
            queued[j] = true;
        }
        while cursor < queue.len() {
            let j = queue[cursor];
            cursor += 1;
            if !visited[j] {
                visited[j] = true;
                let next = neighbors(j);
                if next.len() >= min_points {
                    for p in next {
                        if !queued[p] {
                            queue.push(p);
                            queued[p] = true;
                        }
                    }
                }
            }
            if labels[j].is_none() {
                labels[j] = Some(cluster);
            }
        }
        cluster += 1;
    }
    Ok(labels)
}
pub fn agglomerative(x: &[Vec<f64>], k: usize) -> Result<Vec<usize>> {
    matrix_width(x)?;
    if k == 0 || k > x.len() {
        return Err("invalid cluster count");
    }
    let mut groups: Vec<Vec<usize>> = (0..x.len()).map(|i| vec![i]).collect();
    while groups.len() > k {
        let mut best = (0, 1, f64::INFINITY);
        for a in 0..groups.len() {
            for b in a + 1..groups.len() {
                let distance = groups[a]
                    .iter()
                    .flat_map(|&i| groups[b].iter().map(move |&j| distance2(&x[i], &x[j])))
                    .fold(f64::INFINITY, f64::min);
                if distance < best.2 {
                    best = (a, b, distance);
                }
            }
        }
        let other = groups.remove(best.1);
        groups[best.0].extend(other);
    }
    let mut labels = vec![0; x.len()];
    for (label, group) in groups.iter().enumerate() {
        for &i in group {
            labels[i] = label;
        }
    }
    Ok(labels)
}
pub struct Pca {
    pub mean: Vec<f64>,
    pub components: Vec<Vec<f64>>,
    pub eigenvalues: Vec<f64>,
}
impl Pca {
    pub fn fit(x: &[Vec<f64>], dimensions: usize) -> Result<Self> {
        let d = matrix_width(x)?;
        if dimensions == 0 || dimensions > d || x.len() < 2 {
            return Err("invalid PCA dimensions");
        }
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
        let (values, vectors) = symmetric_eigen(&covariance)?;
        let mut order: Vec<_> = (0..d).collect();
        order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
        let components = order[..dimensions]
            .iter()
            .map(|&i| vectors.iter().map(|r| r[i]).collect())
            .collect();
        let eigenvalues = order[..dimensions].iter().map(|&i| values[i]).collect();
        Ok(Self {
            mean,
            components,
            eigenvalues,
        })
    }
    pub fn transform(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.len() != self.mean.len() || x.iter().any(|v| !v.is_finite()) {
            return Err("invalid feature vector");
        }
        let centered: Vec<_> = x.iter().zip(&self.mean).map(|(a, b)| a - b).collect();
        Ok(self.components.iter().map(|c| dot(c, &centered)).collect())
    }
}

pub fn tsne_probabilities(x: &[Vec<f64>], perplexity: f64) -> Result<Vec<Vec<f64>>> {
    matrix_width(x)?;
    let n = x.len();
    if n < 3 || !perplexity.is_finite() || perplexity < 1.0 || perplexity >= (n - 1) as f64 {
        return Err("perplexity must lie in [1,n-1)");
    }
    let mut conditional = vec![vec![0.0; n]; n];
    for i in 0..n {
        let distances: Vec<_> = x.iter().map(|row| distance2(&x[i], row)).collect();
        let min = distances
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, v)| *v)
            .fold(f64::INFINITY, f64::min);
        let (mut beta, mut low, mut high) = (1.0, 0.0, f64::INFINITY);
        for _ in 0..80 {
            let mut weights: Vec<_> = distances
                .iter()
                .enumerate()
                .map(|(j, v)| {
                    if i == j {
                        0.0
                    } else {
                        (-(v - min) * beta).exp()
                    }
                })
                .collect();
            let total: f64 = weights.iter().sum();
            for w in &mut weights {
                *w /= total;
            }
            let entropy = -weights
                .iter()
                .filter(|v| **v > 0.0)
                .map(|v| v * v.ln())
                .sum::<f64>();
            conditional[i] = weights;
            if (entropy - perplexity.ln()).abs() < 1e-8 {
                break;
            }
            if entropy > perplexity.ln() {
                low = beta;
                beta = if high.is_infinite() {
                    beta * 2.0
                } else {
                    (beta + high) / 2.0
                };
            } else {
                high = beta;
                beta = (beta + low) / 2.0;
            }
        }
    }
    Ok((0..n)
        .map(|i| {
            (0..n)
                .map(|j| (conditional[i][j] + conditional[j][i]) / (2 * n) as f64)
                .collect()
        })
        .collect())
}
/// Exact O(n²) Student-t objective. P must be symmetric, normalized, and zero-diagonal.
pub fn tsne_loss_gradient(p: &[Vec<f64>], y: &[Vec<f64>]) -> Result<(f64, Vec<Vec<f64>>)> {
    let d = matrix_width(y)?;
    let n = y.len();
    if n < 2 || matrix_width(p)? != n || p.len() != n || p.iter().flatten().any(|v| *v < 0.0) {
        return Err("invalid t-SNE probabilities");
    }
    if (p.iter().flatten().sum::<f64>() - 1.0).abs() > 1e-8
        || (0..n).any(|i| p[i][i] != 0.0 || (0..n).any(|j| (p[i][j] - p[j][i]).abs() > 1e-12))
    {
        return Err("t-SNE probabilities must be symmetric, normalized and zero-diagonal");
    }
    let weights: Vec<Vec<_>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        0.0
                    } else {
                        1.0 / (1.0 + distance2(&y[i], &y[j]))
                    }
                })
                .collect()
        })
        .collect();
    let total: f64 = weights.iter().flatten().sum();
    let mut loss = 0.0;
    let mut gradient = vec![vec![0.0; d]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let q = weights[i][j] / total;
                if p[i][j] > 0.0 {
                    loss += p[i][j] * (p[i][j] / q).ln();
                }
                for k in 0..d {
                    gradient[i][k] += 4.0 * (p[i][j] - q) * weights[i][j] * (y[i][k] - y[j][k]);
                }
            }
        }
    }
    Ok((loss, gradient))
}
pub fn tsne(
    x: &[Vec<f64>],
    perplexity: f64,
    steps: usize,
    rate: f64,
    seed: u64,
) -> Result<Vec<Vec<f64>>> {
    let p = tsne_probabilities(x, perplexity)?;
    if !rate.is_finite() || rate <= 0.0 {
        return Err("invalid rate");
    }
    let mut rng = Rng::new(seed);
    let mut y: Vec<Vec<_>> = (0..x.len())
        .map(|_| vec![rng.normal() * 0.01, rng.normal() * 0.01])
        .collect();
    for _ in 0..steps {
        let (_, gradient) = tsne_loss_gradient(&p, &y)?;
        for (row, g) in y.iter_mut().zip(gradient) {
            for (v, d) in row.iter_mut().zip(g) {
                *v -= rate * d;
            }
        }
        for k in 0..2 {
            let mean = y.iter().map(|r| r[k]).sum::<f64>() / y.len() as f64;
            for r in &mut y {
                r[k] -= mean;
            }
        }
    }
    Ok(y)
}
