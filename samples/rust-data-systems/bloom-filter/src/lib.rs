use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Bloom filter using double hashing technique.
///
/// Two independent hash values h1 and h2 are derived from a single hash,
/// then combined as: hash_i(x) = h1(x) + i * h2(x) for i in 0..k
pub struct BloomFilter {
    bits: Vec<u64>,
    num_bits: usize,
    num_hashes: u32,
}

impl BloomFilter {
    /// Create a new Bloom filter sized for `expected_items` with target false positive rate `fp_rate`.
    ///
    /// # Errors
    /// Returns `Err` if `expected_items` is 0 or `fp_rate` is not in (0, 1).
    pub fn new(expected_items: usize, fp_rate: f64) -> Result<Self, BloomFilterError> {
        if expected_items == 0 {
            return Err(BloomFilterError::ZeroItems);
        }
        if fp_rate <= 0.0 || fp_rate >= 1.0 {
            return Err(BloomFilterError::InvalidFpRate(fp_rate));
        }

        let num_bits = optimal_num_bits(expected_items, fp_rate);
        let num_hashes = optimal_num_hashes(num_bits, expected_items);

        let words = num_bits.div_ceil(64);
        Ok(Self {
            bits: vec![0u64; words],
            num_bits,
            num_hashes,
        })
    }

    /// Create a Bloom filter with explicit bit count and hash count.
    ///
    /// # Errors
    /// Returns `Err` if `num_bits` or `num_hashes` is 0.
    pub fn with_params(num_bits: usize, num_hashes: u32) -> Result<Self, BloomFilterError> {
        if num_bits == 0 {
            return Err(BloomFilterError::ZeroBits);
        }
        if num_hashes == 0 {
            return Err(BloomFilterError::ZeroHashes);
        }
        let words = num_bits.div_ceil(64);
        Ok(Self {
            bits: vec![0u64; words],
            num_bits,
            num_hashes,
        })
    }

    /// Insert an item into the filter.
    pub fn insert<T: Hash>(&mut self, item: &T) {
        let (h1, h2) = double_hash(item);
        for i in 0..self.num_hashes {
            let idx = self.get_index(h1, h2, i);
            let word = idx / 64;
            let bit = idx % 64;
            self.bits[word] |= 1u64 << bit;
        }
    }

    /// Check if an item might be in the filter.
    /// Returns `false` if definitely not present, `true` if possibly present.
    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let (h1, h2) = double_hash(item);
        for i in 0..self.num_hashes {
            let idx = self.get_index(h1, h2, i);
            let word = idx / 64;
            let bit = idx % 64;
            if self.bits[word] & (1u64 << bit) == 0 {
                return false;
            }
        }
        true
    }

    /// Number of bits in the filter.
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Number of hash functions.
    pub fn num_hashes(&self) -> u32 {
        self.num_hashes
    }

    /// Approximate memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.bits.len() * 8
    }

    /// Count of set bits (for diagnostics).
    pub fn count_ones(&self) -> usize {
        self.bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    fn get_index(&self, h1: u64, h2: u64, i: u32) -> usize {
        let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
        (combined % self.num_bits as u64) as usize
    }
}

/// Counting Bloom filter -- supports deletion by using counters instead of single bits.
pub struct CountingBloomFilter {
    counters: Vec<u8>,
    num_slots: usize,
    num_hashes: u32,
}

impl CountingBloomFilter {
    /// Create a new counting Bloom filter.
    ///
    /// # Errors
    /// Returns `Err` if `expected_items` is 0 or `fp_rate` is not in (0, 1).
    pub fn new(expected_items: usize, fp_rate: f64) -> Result<Self, BloomFilterError> {
        if expected_items == 0 {
            return Err(BloomFilterError::ZeroItems);
        }
        if fp_rate <= 0.0 || fp_rate >= 1.0 {
            return Err(BloomFilterError::InvalidFpRate(fp_rate));
        }

        let num_slots = optimal_num_bits(expected_items, fp_rate);
        let num_hashes = optimal_num_hashes(num_slots, expected_items);

        Ok(Self {
            counters: vec![0u8; num_slots],
            num_slots,
            num_hashes,
        })
    }

    pub fn insert<T: Hash>(&mut self, item: &T) {
        let (h1, h2) = double_hash(item);
        for i in 0..self.num_hashes {
            let idx = self.get_index(h1, h2, i);
            self.counters[idx] = self.counters[idx].saturating_add(1);
        }
    }

    /// Remove an item. Returns `true` if the item was possibly present.
    pub fn remove<T: Hash>(&mut self, item: &T) -> bool {
        let (h1, h2) = double_hash(item);
        let mut indices = Vec::with_capacity(self.num_hashes as usize);

        // First check all positions are non-zero
        for i in 0..self.num_hashes {
            let idx = self.get_index(h1, h2, i);
            if self.counters[idx] == 0 {
                return false;
            }
            indices.push(idx);
        }

        // Decrement all positions
        for idx in indices {
            self.counters[idx] = self.counters[idx].saturating_sub(1);
        }
        true
    }

    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let (h1, h2) = double_hash(item);
        for i in 0..self.num_hashes {
            let idx = self.get_index(h1, h2, i);
            if self.counters[idx] == 0 {
                return false;
            }
        }
        true
    }

    pub fn memory_bytes(&self) -> usize {
        self.counters.len()
    }

    fn get_index(&self, h1: u64, h2: u64, i: u32) -> usize {
        let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
        (combined % self.num_slots as u64) as usize
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BloomFilterError {
    #[error("expected_items must be > 0")]
    ZeroItems,
    #[error("num_bits must be > 0")]
    ZeroBits,
    #[error("num_hashes must be > 0")]
    ZeroHashes,
    #[error("fp_rate must be in (0, 1), got {0}")]
    InvalidFpRate(f64),
}

/// Optimal number of bits: m = -n * ln(p) / (ln2)^2
fn optimal_num_bits(n: usize, fp_rate: f64) -> usize {
    let m = -(n as f64) * fp_rate.ln() / (2.0_f64.ln().powi(2));
    m.ceil() as usize
}

/// Optimal number of hashes: k = (m/n) * ln2
fn optimal_num_hashes(m: usize, n: usize) -> u32 {
    let k = (m as f64 / n as f64) * 2.0_f64.ln();
    let k = k.round() as u32;
    k.max(1)
}

/// Theoretical false positive rate: p = (1 - e^(-kn/m))^k
pub fn theoretical_fp_rate(num_bits: usize, num_hashes: u32, num_items: usize) -> f64 {
    let m = num_bits as f64;
    let k = num_hashes as f64;
    let n = num_items as f64;
    (1.0 - (-k * n / m).exp()).powf(k)
}

/// Double hashing: derive two hash streams from a single item.
/// DefaultHasher is fine for this educational verifier, but its algorithm is not stable API.
fn double_hash<T: Hash>(item: &T) -> (u64, u64) {
    let mut hasher1 = DefaultHasher::new();
    item.hash(&mut hasher1);
    let h1 = hasher1.finish();

    // Second hash: feed the first hash as additional entropy
    let mut hasher2 = DefaultHasher::new();
    h1.hash(&mut hasher2);
    item.hash(&mut hasher2);
    let h2 = hasher2.finish();

    // Ensure h2 is odd for better distribution across bit positions
    (h1, h2 | 1)
}

/// Blocked Bloom Filter.
///
/// 通常のBloom filterはk個のハッシュごとに散らばったビットを触るので、
/// 大きなビット配列ではキャッシュミスが連続する。Blocked Bloom filterは
/// ビット配列を「ブロック」（64バイト = 1キャッシュライン）に分割し、
/// 1要素のk個のビットを同一ブロック内で完結させる。理論上の偽陽性率は
/// 通常のBloomよりわずかに悪化するが、ルックアップ速度は1キャッシュライン
/// アクセスで済むぶん大きく改善する（特に大きなフィルタで顕著）。
pub struct BlockedBloomFilter {
    blocks: Vec<[u64; 8]>, // 各ブロック 512bit = 64byte = 1 cache line
    num_blocks: usize,
    num_hashes: u32,
}

const BLOCK_BITS: usize = 512;

impl BlockedBloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Result<Self, BloomFilterError> {
        if expected_items == 0 {
            return Err(BloomFilterError::ZeroItems);
        }
        if fp_rate <= 0.0 || fp_rate >= 1.0 {
            return Err(BloomFilterError::InvalidFpRate(fp_rate));
        }
        let total_bits = optimal_num_bits(expected_items, fp_rate);
        // 多少多めにブロックを取り、ブロックあたりのload factorを抑える
        let num_blocks = (total_bits.div_ceil(BLOCK_BITS)).max(1);
        let num_hashes = optimal_num_hashes(total_bits, expected_items);
        Ok(Self {
            blocks: vec![[0u64; 8]; num_blocks],
            num_blocks,
            num_hashes,
        })
    }

    pub fn insert<T: Hash>(&mut self, item: &T) {
        let (h1, h2) = double_hash(item);
        let block_idx = (h1 as usize) % self.num_blocks;
        for i in 0..self.num_hashes {
            let bit = (h1.wrapping_add((i as u64).wrapping_mul(h2)) as usize) % BLOCK_BITS;
            let word = bit / 64;
            let off = bit % 64;
            self.blocks[block_idx][word] |= 1u64 << off;
        }
    }

    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let (h1, h2) = double_hash(item);
        let block_idx = (h1 as usize) % self.num_blocks;
        for i in 0..self.num_hashes {
            let bit = (h1.wrapping_add((i as u64).wrapping_mul(h2)) as usize) % BLOCK_BITS;
            let word = bit / 64;
            let off = bit % 64;
            if self.blocks[block_idx][word] & (1u64 << off) == 0 {
                return false;
            }
        }
        true
    }

    pub fn num_blocks(&self) -> usize {
        self.num_blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_false_negatives() {
        let mut bf = BloomFilter::new(1000, 0.01).unwrap();
        let items: Vec<String> = (0..1000).map(|i| format!("item-{i}")).collect();

        for item in &items {
            bf.insert(item);
        }

        for item in &items {
            assert!(bf.contains(item), "false negative for {item}");
        }
    }

    #[test]
    fn false_positive_rate_1000() {
        measure_fp_rate(1_000, 0.01, 0.025);
    }

    #[test]
    fn false_positive_rate_10000() {
        measure_fp_rate(10_000, 0.01, 0.02);
    }

    #[test]
    fn false_positive_rate_100000() {
        measure_fp_rate(100_000, 0.01, 0.015);
    }

    fn measure_fp_rate(n: usize, target_fp: f64, tolerance: f64) {
        let mut bf = BloomFilter::new(n, target_fp).unwrap();

        // Insert n items
        for i in 0..n {
            bf.insert(&format!("present-{i}"));
        }

        // Test with items that were NOT inserted
        let test_count = n * 10;
        let mut false_positives = 0;
        for i in 0..test_count {
            if bf.contains(&format!("absent-{i}")) {
                false_positives += 1;
            }
        }

        let measured_fp = false_positives as f64 / test_count as f64;
        let theoretical = theoretical_fp_rate(bf.num_bits(), bf.num_hashes(), n);

        eprintln!(
            "n={n}: measured FP={measured_fp:.6}, theoretical={theoretical:.6}, target={target_fp}"
        );
        eprintln!(
            "  bits={}, hashes={}, memory={}B, bits_set={}/{}",
            bf.num_bits(),
            bf.num_hashes(),
            bf.memory_bytes(),
            bf.count_ones(),
            bf.num_bits()
        );

        assert!(
            measured_fp < tolerance,
            "FP rate {measured_fp} exceeds tolerance {tolerance} (target was {target_fp})"
        );
    }

    #[test]
    fn memory_comparison_with_hashset() {
        let n = 10_000usize;
        let mut bf = BloomFilter::new(n, 0.01).unwrap();
        let mut hs: HashSet<String> = HashSet::new();

        for i in 0..n {
            let item = format!("item-{i}");
            bf.insert(&item);
            hs.insert(item);
        }

        let bf_bytes = bf.memory_bytes();
        // HashSet: rough estimate ~(key_size + overhead) per entry
        // Each String "item-XXXX" is ~10 bytes content + 24 bytes String struct + hash table overhead
        let hs_bytes_estimate = n * 72; // rough per-entry estimate

        eprintln!("BloomFilter: {bf_bytes} bytes");
        eprintln!("HashSet (estimate): {hs_bytes_estimate} bytes");
        eprintln!(
            "Ratio: {:.1}x smaller",
            hs_bytes_estimate as f64 / bf_bytes as f64
        );

        assert!(
            bf_bytes < hs_bytes_estimate / 5,
            "Bloom filter should be much smaller than HashSet"
        );
    }

    #[test]
    fn optimal_params_sanity() {
        // For n=1000, p=0.01: m ≈ 9585, k ≈ 7
        let m = optimal_num_bits(1000, 0.01);
        let k = optimal_num_hashes(m, 1000);
        assert!((9000..10000).contains(&m), "m={m} not in expected range");
        assert!((6..=8).contains(&k), "k={k} not in expected range");
    }

    #[test]
    fn counting_bloom_filter_insert_remove() {
        let mut cbf = CountingBloomFilter::new(1000, 0.01).unwrap();

        cbf.insert(&"hello");
        cbf.insert(&"world");
        assert!(cbf.contains(&"hello"));
        assert!(cbf.contains(&"world"));
        assert!(!cbf.contains(&"missing"));

        // Remove "hello"
        assert!(cbf.remove(&"hello"));
        assert!(!cbf.contains(&"hello"));
        assert!(cbf.contains(&"world"));

        // Removing non-existent item returns false
        assert!(!cbf.remove(&"never-inserted"));
    }

    #[test]
    fn counting_bloom_filter_multiple_inserts() {
        let mut cbf = CountingBloomFilter::new(1000, 0.01).unwrap();

        cbf.insert(&"item");
        cbf.insert(&"item");
        cbf.insert(&"item");

        // After 3 inserts, removing once should still leave it "present"
        assert!(cbf.remove(&"item"));
        assert!(cbf.contains(&"item"));

        assert!(cbf.remove(&"item"));
        assert!(cbf.contains(&"item"));

        assert!(cbf.remove(&"item"));
        assert!(!cbf.contains(&"item"));
    }

    #[test]
    fn error_cases() {
        assert!(BloomFilter::new(0, 0.01).is_err());
        assert!(BloomFilter::new(100, 0.0).is_err());
        assert!(BloomFilter::new(100, 1.0).is_err());
        assert!(BloomFilter::new(100, -0.5).is_err());
        assert!(BloomFilter::with_params(0, 7).is_err());
        assert!(BloomFilter::with_params(1000, 0).is_err());
    }

    #[test]
    fn theoretical_fp_rate_sanity() {
        // For m=9585, k=7, n=1000, theoretical FP ≈ 0.01
        let rate = theoretical_fp_rate(9585, 7, 1000);
        assert!(
            (0.005..0.02).contains(&rate),
            "theoretical rate {rate} not near 0.01"
        );
    }

    #[test]
    fn empty_filter_contains_nothing() {
        let bf = BloomFilter::new(1000, 0.01).unwrap();
        for i in 0..100 {
            assert!(!bf.contains(&format!("item-{i}")));
        }
    }

    #[test]
    fn blocked_bloom_no_false_negatives() {
        let mut bbf = BlockedBloomFilter::new(10_000, 0.01).unwrap();
        let items: Vec<String> = (0..10_000).map(|i| format!("item-{i}")).collect();
        for it in &items {
            bbf.insert(it);
        }
        for it in &items {
            assert!(bbf.contains(it), "false negative for {it}");
        }
    }

    #[test]
    fn blocked_bloom_false_positive_rate_in_bounds() {
        let n = 10_000;
        let mut bbf = BlockedBloomFilter::new(n, 0.01).unwrap();
        for i in 0..n {
            bbf.insert(&format!("present-{i}"));
        }
        let mut fp = 0usize;
        let probes = 50_000;
        for i in 0..probes {
            if bbf.contains(&format!("absent-{i}")) {
                fp += 1;
            }
        }
        let rate = fp as f64 / probes as f64;
        eprintln!("Blocked Bloom fp_rate ≈ {rate:.4}");
        // ブロック化により理論値より少し悪化することを許容（実用上3%以内）
        assert!(rate < 0.03, "fp_rate {rate} unexpectedly high");
    }
}
