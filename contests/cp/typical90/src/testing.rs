//! テスト用ヘルパー
//!
//! 競プロのテストを効率化するユーティリティ

use std::fmt::Debug;
use std::time::Instant;

/// 関数の実行時間を計測
pub fn measure_time<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    eprintln!("{}: {:?}", name, elapsed);
    result
}

/// ストレステスト: 2つの解法を比較
///
/// # Arguments
/// * `generator` - テストケース生成関数
/// * `naive` - 愚直解（正しいことが保証された遅い解法）
/// * `fast` - 高速解（検証したい解法）
/// * `iterations` - テスト回数
pub fn stress_test<T, R, G, N, F>(generator: G, naive: N, fast: F, iterations: usize) -> bool
where
    T: Clone + Debug,
    R: PartialEq + Debug,
    G: Fn(usize) -> T,
    N: Fn(&T) -> R,
    F: Fn(&T) -> R,
{
    for i in 0..iterations {
        let input = generator(i);
        let expected = naive(&input);
        let actual = fast(&input);

        if expected != actual {
            eprintln!("Mismatch at iteration {}", i);
            eprintln!("Input: {:?}", input);
            eprintln!("Expected: {:?}", expected);
            eprintln!("Actual: {:?}", actual);
            return false;
        }
    }
    eprintln!("All {} tests passed!", iterations);
    true
}

/// ランダムな配列を生成
pub fn random_array(seed: u64, len: usize, min: i64, max: i64) -> Vec<i64> {
    let mut rng = SimpleRng::new(seed);
    (0..len).map(|_| rng.next_range(min, max)).collect()
}

/// ランダムな順列を生成
pub fn random_permutation(seed: u64, n: usize) -> Vec<usize> {
    let mut rng = SimpleRng::new(seed);
    let mut perm: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.next_range(0, i as i64 + 1) as usize;
        perm.swap(i, j);
    }
    perm
}

/// ランダムな木（辺リスト）を生成
pub fn random_tree(seed: u64, n: usize) -> Vec<(usize, usize)> {
    if n <= 1 {
        return vec![];
    }
    let mut rng = SimpleRng::new(seed);
    let mut edges = Vec::with_capacity(n - 1);
    for i in 1..n {
        let parent = rng.next_range(0, i as i64) as usize;
        edges.push((parent, i));
    }
    edges
}

/// 簡易乱数生成器 (xorshift64)
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_range(&mut self, min: i64, max: i64) -> i64 {
        let range = (max - min) as u64;
        if range == 0 {
            return min;
        }
        min + (self.next() % range) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_array() {
        let arr = random_array(42, 10, 1, 100);
        assert_eq!(arr.len(), 10);
        assert!(arr.iter().all(|&x| x >= 1 && x < 100));
    }

    #[test]
    fn test_random_permutation() {
        let perm = random_permutation(42, 10);
        assert_eq!(perm.len(), 10);
        let mut sorted = perm.clone();
        sorted.sort();
        assert_eq!(sorted, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_random_tree() {
        let edges = random_tree(42, 10);
        assert_eq!(edges.len(), 9);
        // 全ての辺が parent < child
        assert!(edges.iter().all(|&(p, c)| p < c));
    }

    #[test]
    fn test_stress_test() {
        // 配列の和を計算する2つの方法を比較
        let naive = |arr: &Vec<i64>| -> i64 { arr.iter().sum() };
        let fast = |arr: &Vec<i64>| -> i64 { arr.iter().fold(0, |acc, &x| acc + x) };
        let generator = |i| random_array(i as u64, 100, -1000, 1000);

        assert!(stress_test(generator, naive, fast, 100));
    }
}
