//! Reproducible, non-cryptographic random numbers for the small experiments.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn index(&mut self, length: usize) -> Option<usize> {
        if length == 0 {
            return None;
        }
        let bound = u64::try_from(length).ok()?;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return usize::try_from(value % bound).ok();
            }
        }
    }
    pub fn normal(&mut self) -> f64 {
        (-2.0 * (1.0 - self.uniform()).ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
    }
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.index(i + 1).expect("positive range");
            values.swap(i, j);
        }
    }
    pub fn weighted(&mut self, weights: &[f64]) -> Option<usize> {
        if weights.is_empty() || weights.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return None;
        }
        let scale = weights.iter().copied().fold(0.0, f64::max);
        if scale == 0.0 {
            return self.index(weights.len());
        }
        let total: f64 = weights.iter().map(|v| v / scale).sum();
        let mut target = self.uniform() * total;
        for (i, weight) in weights.iter().enumerate() {
            target -= weight / scale;
            if target < 0.0 {
                return Some(i);
            }
        }
        weights.iter().rposition(|v| *v > 0.0)
    }
}
