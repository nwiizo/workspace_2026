use crate::Result;
pub(crate) fn matrix_width(rows: &[Vec<f64>]) -> Result<usize> {
    let width = rows.first().ok_or("empty matrix")?.len();
    if width == 0
        || rows
            .iter()
            .any(|r| r.len() != width || r.iter().any(|v| !v.is_finite()))
    {
        return Err("ragged or nonfinite matrix");
    }
    Ok(width)
}
pub(crate) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub(crate) fn distance2(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum()
}
pub fn softmax(values: &[f64]) -> Result<Vec<f64>> {
    if values.is_empty() || values.iter().any(|v| v.is_nan() || *v == f64::INFINITY) {
        return Err("invalid logits");
    }
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return Err("all logits masked");
    }
    let mut probs: Vec<_> = values.iter().map(|v| (v - max).exp()).collect();
    let sum: f64 = probs.iter().sum();
    for value in &mut probs {
        *value /= sum;
    }
    Ok(probs)
}
pub fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let e = x.exp();
        e / (1.0 + e)
    }
}
pub fn mean_squared_error(actual: &[f64], predicted: &[f64]) -> Result<f64> {
    if actual.is_empty()
        || actual.len() != predicted.len()
        || actual.iter().chain(predicted).any(|x| !x.is_finite())
    {
        return Err("invalid observations");
    }
    Ok(distance2(actual, predicted) / actual.len() as f64)
}
/// Jacobi eigen-decomposition for a real symmetric matrix; eigenvectors are columns.
pub fn symmetric_eigen(matrix: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
    let n = matrix_width(matrix)?;
    if matrix.len() != n
        || (0..n).any(|i| (0..n).any(|j| (matrix[i][j] - matrix[j][i]).abs() > 1e-10))
    {
        return Err("matrix is not symmetric");
    }
    let mut a = matrix.to_vec();
    let mut vectors = vec![vec![0.0; n]; n];
    for (i, row) in vectors.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _ in 0..100 * n * n {
        let mut pivot = (0, 0, 0.0_f64);
        for (i, row) in a.iter().enumerate() {
            for (j, &value) in row.iter().enumerate().skip(i + 1) {
                if value.abs() > pivot.2 {
                    pivot = (i, j, value.abs());
                }
            }
        }
        if pivot.2 < 1e-12 {
            break;
        }
        let (p, q, _) = pivot;
        let theta = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (s, c) = theta.sin_cos();
        let (app, aqq, apq) = (a[p][p], a[q][q], a[p][q]);
        for k in 0..n {
            if k != p && k != q {
                let (akp, akq) = (a[k][p], a[k][q]);
                a[k][p] = c * akp - s * akq;
                a[p][k] = a[k][p];
                a[k][q] = s * akp + c * akq;
                a[q][k] = a[k][q];
            }
            let (vp, vq) = (vectors[k][p], vectors[k][q]);
            vectors[k][p] = c * vp - s * vq;
            vectors[k][q] = s * vp + c * vq;
        }
        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
    }
    Ok(((0..n).map(|i| a[i][i]).collect(), vectors))
}
pub fn solve_linear(matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>> {
    let n = matrix_width(matrix)?;
    if matrix.len() != n || rhs.len() != n || rhs.iter().any(|v| !v.is_finite()) {
        return Err("invalid linear system");
    }
    let mut a: Vec<_> = matrix
        .iter()
        .zip(rhs)
        .map(|(r, b)| {
            let mut r = r.clone();
            r.push(*b);
            r
        })
        .collect();
    for p in 0..n {
        let pivot = (p..n)
            .max_by(|&i, &j| a[i][p].abs().total_cmp(&a[j][p].abs()))
            .expect("nonempty");
        if a[pivot][p].abs() < 1e-12 {
            return Err("singular linear system");
        }
        a.swap(p, pivot);
        let divisor = a[p][p];
        for value in &mut a[p][p..] {
            *value /= divisor;
        }
        let pivot_row = a[p].clone();
        for (i, row) in a.iter_mut().enumerate() {
            if i == p {
                continue;
            }
            let factor = row[p];
            for (value, pivot) in row[p..].iter_mut().zip(&pivot_row[p..]) {
                *value -= factor * pivot;
            }
        }
    }
    Ok(a.iter().map(|r| r[n]).collect())
}
