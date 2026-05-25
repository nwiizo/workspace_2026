use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A consistent hash ring with virtual nodes.
pub struct HashRing {
    ring: BTreeMap<u64, String>,
    virtual_nodes: u32,
    nodes: Vec<String>,
}

impl HashRing {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
            nodes: Vec::new(),
        }
    }

    /// Add a node to the ring. Returns the number of virtual nodes added.
    pub fn add_node(&mut self, node: &str) -> u32 {
        self.nodes.push(node.to_string());
        let mut added = 0;
        for i in 0..self.virtual_nodes {
            let hash = hash_with_seed(node, i);
            self.ring.insert(hash, node.to_string());
            added += 1;
        }
        added
    }

    /// Remove a node from the ring. Returns the number of virtual nodes removed.
    pub fn remove_node(&mut self, node: &str) -> u32 {
        self.nodes.retain(|n| n != node);
        let mut removed = 0;
        for i in 0..self.virtual_nodes {
            let hash = hash_with_seed(node, i);
            if self.ring.remove(&hash).is_some() {
                removed += 1;
            }
        }
        removed
    }

    /// Get the node responsible for a key.
    /// Returns `None` if the ring is empty.
    pub fn get_node<T: Hash>(&self, key: &T) -> Option<&str> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = compute_hash(key);
        // Find the first node with hash >= key hash (clockwise search)
        self.ring
            .range(hash..)
            .next()
            .or_else(|| self.ring.iter().next())
            .map(|(_, n)| n.as_str())
    }

    /// Number of physical nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of points on the ring.
    pub fn ring_size(&self) -> usize {
        self.ring.len()
    }
}

/// Rendezvous Hashing (Highest Random Weight).
/// For each key, compute a weight for every node and pick the highest.
pub struct RendezvousHashing {
    nodes: Vec<String>,
}

impl RendezvousHashing {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: &str) {
        self.nodes.push(node.to_string());
    }

    pub fn remove_node(&mut self, node: &str) {
        self.nodes.retain(|n| n != node);
    }

    /// Get the node responsible for a key.
    /// Returns `None` if there are no nodes.
    pub fn get_node<T: Hash>(&self, key: &T) -> Option<&str> {
        self.nodes
            .iter()
            .map(|node| {
                let weight = compute_weight(key, node);
                (weight, node.as_str())
            })
            .max_by_key(|(w, _)| *w)
            .map(|(_, n)| n)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for RendezvousHashing {
    fn default() -> Self {
        Self::new()
    }
}

/// Jump Consistent Hash (Google, 2014).
/// Maps a key to one of `num_buckets` buckets with minimal disruption.
/// Only supports adding/removing the last bucket.
pub fn jump_consistent_hash(key: u64, num_buckets: u32) -> u32 {
    let mut b: i64 = -1;
    let mut j: i64 = 0;
    let mut k = key;

    while j < num_buckets as i64 {
        b = j;
        k = k.wrapping_mul(2862933555777941757).wrapping_add(1);
        j = ((b + 1) as f64 * ((1i64 << 31) as f64 / ((k >> 33) + 1) as f64)) as i64;
    }

    b as u32
}

/// MementoHash (Coluzzi et al., IEEE Access 2024).
///
/// Lifts Jumpの「末尾削除のみ」制約。削除済みバケットの"墓標"テーブル
/// （`replacements`）を持ち、ルックアップでJumpの結果が削除済みなら
/// `h mod working_set_size` で再ハッシュして生存バケットへ転送する。
/// メモリは削除バケット数に比例（AnchorHash/DxHashのようなO(N)ではない）。
pub struct MementoHash {
    /// Highest-ever bucket id + 1（縮退すれば減るが、削除穴は維持）
    b_array_size: u32,
    /// 直近に削除されたバケット。誰も削除されていなければ b_array_size と等しい。
    last_removed: u32,
    /// 削除バケット → (置換指標 replacer, その時点の last_removed)
    replacements: std::collections::HashMap<u32, (u32, u32)>,
}

impl MementoHash {
    pub fn new(initial_buckets: u32) -> Self {
        Self {
            b_array_size: initial_buckets,
            last_removed: initial_buckets,
            replacements: std::collections::HashMap::new(),
        }
    }

    /// 現在生存中のバケット数
    pub fn working_set_size(&self) -> u32 {
        self.b_array_size - self.replacements.len() as u32
    }

    /// バケットを追加。末尾削除→末尾追加のJump的拡張も含めて扱える。
    pub fn add_bucket(&mut self) -> u32 {
        if self.replacements.is_empty() {
            let b = self.b_array_size;
            self.b_array_size += 1;
            self.last_removed = self.b_array_size;
            return b;
        }
        let bucket = self.last_removed;
        // last_removedは「ひとつ前の削除」へ巻き戻る
        let (_, prev_removed) = self
            .replacements
            .remove(&bucket)
            .expect("last_removed must be in replacements when restoring");
        self.last_removed = prev_removed;
        if bucket + 1 > self.b_array_size {
            self.b_array_size = bucket + 1;
        }
        bucket
    }

    /// 任意のバケットを削除。Jumpと違って末尾以外でも可能。
    pub fn remove_bucket(&mut self, bucket: u32) {
        // Tail shrink fast path: 末尾連続削除をb_array_sizeで畳む
        if self.last_removed == self.b_array_size && bucket == self.b_array_size - 1 {
            self.b_array_size = bucket;
            self.last_removed = bucket;
            return;
        }
        let replacer = self.working_set_size() - 1;
        self.replacements
            .insert(bucket, (replacer, self.last_removed));
        self.last_removed = bucket;
    }

    /// 鍵を生存バケットへ写像する。
    pub fn get_bucket<T: Hash>(&self, key: &T) -> u32 {
        let base = compute_hash(key);
        let mut b = jump_consistent_hash(base, self.b_array_size);
        while let Some(&(replacer, _)) = self.replacements.get(&b) {
            // バケットbが削除されていれば、削除時の working_set_size-1 を modulus に再ハッシュ
            let h = rehash_for_bucket(base, b);
            b = (h % replacer as u64) as u32;
            // 再ハッシュ先 d も削除済みかつ「より新しい削除」なら、置換チェーンを登る
            while let Some(&(r, _)) = self.replacements.get(&b) {
                if r < replacer {
                    break;
                }
                b = r;
            }
        }
        b
    }
}

fn rehash_for_bucket(base: u64, bucket: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    base.hash(&mut hasher);
    bucket.hash(&mut hasher);
    hasher.finish()
}

fn compute_hash<T: Hash>(item: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);
    hasher.finish()
}

fn hash_with_seed(node: &str, seed: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    node.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish()
}

fn compute_weight<T: Hash>(key: &T, node: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    node.hash(&mut hasher);
    hasher.finish()
}

/// Measure the standard deviation of key distribution across nodes.
pub fn measure_distribution_stddev(ring: &HashRing, num_keys: usize) -> f64 {
    if ring.node_count() == 0 {
        return 0.0;
    }

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for i in 0..num_keys {
        if let Some(node) = ring.get_node(&format!("key-{i}")) {
            *counts.entry(node.to_string()).or_default() += 1;
        }
    }

    let n = ring.node_count() as f64;
    let mean = num_keys as f64 / n;
    let variance = counts
        .values()
        .map(|&c| {
            let diff = c as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;

    variance.sqrt()
}

/// Measure key movement ratio when a node is added.
/// Returns (keys_moved, total_keys, ratio).
pub fn measure_key_movement(
    virtual_nodes: u32,
    initial_nodes: &[&str],
    new_node: &str,
    num_keys: usize,
) -> (usize, usize, f64) {
    // Build ring without new node
    let mut ring_before = HashRing::new(virtual_nodes);
    for node in initial_nodes {
        ring_before.add_node(node);
    }

    // Build ring with new node
    let mut ring_after = HashRing::new(virtual_nodes);
    for node in initial_nodes {
        ring_after.add_node(node);
    }
    ring_after.add_node(new_node);

    let mut moved = 0;
    for i in 0..num_keys {
        let key = format!("key-{i}");
        let before = ring_before.get_node(&key);
        let after = ring_after.get_node(&key);
        if before != after {
            moved += 1;
        }
    }

    let ratio = moved as f64 / num_keys as f64;
    (moved, num_keys, ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lookup() {
        let mut ring = HashRing::new(100);
        ring.add_node("node-A");
        ring.add_node("node-B");
        ring.add_node("node-C");

        for i in 0..100 {
            assert!(ring.get_node(&format!("key-{i}")).is_some());
        }
    }

    #[test]
    fn deterministic_lookup() {
        let mut ring = HashRing::new(100);
        ring.add_node("node-A");
        ring.add_node("node-B");

        let result1 = ring.get_node(&"test-key");
        let result2 = ring.get_node(&"test-key");
        assert_eq!(result1, result2, "same key should always map to same node");
    }

    #[test]
    fn empty_ring() {
        let ring = HashRing::new(100);
        assert!(ring.get_node(&"key").is_none());
    }

    #[test]
    fn distribution_improves_with_virtual_nodes() {
        let nodes = ["node-A", "node-B", "node-C", "node-D", "node-E"];
        let num_keys = 100_000;

        let mut results = Vec::new();

        for &vnodes in &[1, 10, 50, 100, 500] {
            let mut ring = HashRing::new(vnodes);
            for node in &nodes {
                ring.add_node(node);
            }
            let stddev = measure_distribution_stddev(&ring, num_keys);
            let mean = num_keys as f64 / nodes.len() as f64;
            let cv = stddev / mean; // coefficient of variation
            results.push((vnodes, stddev, cv));
            eprintln!("vnodes={vnodes:>3}: stddev={stddev:>8.1}, CV={cv:.4}");
        }

        // More virtual nodes should give better distribution (lower CV)
        assert!(
            results.last().unwrap().2 < results.first().unwrap().2,
            "500 virtual nodes should have lower CV than 1"
        );
    }

    #[test]
    fn key_movement_minimality() {
        let initial_nodes = vec!["node-A", "node-B", "node-C", "node-D"];
        let num_keys = 100_000;
        let virtual_nodes = 100;

        let (moved, total, ratio) =
            measure_key_movement(virtual_nodes, &initial_nodes, "node-E", num_keys);

        let ideal_ratio = 1.0 / (initial_nodes.len() + 1) as f64;

        eprintln!("Moved: {moved}/{total} = {ratio:.4} (ideal: {ideal_ratio:.4})");

        assert!(
            ratio < ideal_ratio * 1.5,
            "too many keys moved: {ratio} >> {ideal_ratio}"
        );
        assert!(
            ratio > ideal_ratio * 0.5,
            "too few keys moved: {ratio} << {ideal_ratio}"
        );
    }

    #[test]
    fn naive_mod_n_movement() {
        let num_keys = 100_000;
        let n_before = 4u64;
        let n_after = 5u64;

        let mut moved = 0;
        for i in 0..num_keys {
            let hash = compute_hash(&format!("key-{i}"));
            let before = hash % n_before;
            let after = hash % n_after;
            if before != after {
                moved += 1;
            }
        }

        let ratio = moved as f64 / num_keys as f64;
        eprintln!("Naive mod N: {moved}/{num_keys} moved = {ratio:.4}");

        assert!(
            ratio > 0.7,
            "expected most keys to move with mod N, got {ratio}"
        );
    }

    #[test]
    fn rendezvous_hashing_basic() {
        let mut rh = RendezvousHashing::new();
        rh.add_node("node-A");
        rh.add_node("node-B");
        rh.add_node("node-C");

        let n1 = rh.get_node(&"test-key");
        let n2 = rh.get_node(&"test-key");
        assert_eq!(n1, n2);

        for i in 0..100 {
            assert!(rh.get_node(&format!("key-{i}")).is_some());
        }
    }

    #[test]
    fn rendezvous_hashing_minimal_movement() {
        let mut rh_before = RendezvousHashing::new();
        for name in &["A", "B", "C", "D"] {
            rh_before.add_node(name);
        }

        let mut rh_after = RendezvousHashing::new();
        for name in &["A", "B", "C", "D", "E"] {
            rh_after.add_node(name);
        }

        let num_keys = 100_000;
        let mut moved = 0;
        for i in 0..num_keys {
            let key = format!("key-{i}");
            if rh_before.get_node(&key) != rh_after.get_node(&key) {
                moved += 1;
            }
        }

        let ratio = moved as f64 / num_keys as f64;
        let ideal = 1.0 / 5.0;
        eprintln!("Rendezvous: moved {moved}/{num_keys} = {ratio:.4} (ideal: {ideal:.4})");

        assert!(
            (ratio - ideal).abs() < 0.02,
            "ratio {ratio} too far from ideal {ideal}"
        );
    }

    #[test]
    fn jump_consistent_hash_basic() {
        let b1 = jump_consistent_hash(42, 10);
        let b2 = jump_consistent_hash(42, 10);
        assert_eq!(b1, b2);

        for key in 0..10_000u64 {
            let bucket = jump_consistent_hash(key, 10);
            assert!(bucket < 10, "bucket {bucket} out of range");
        }
    }

    #[test]
    fn jump_consistent_hash_minimal_disruption() {
        let num_keys = 100_000u64;
        let buckets_before = 10u32;
        let buckets_after = 11u32;

        let mut moved = 0;
        for key in 0..num_keys {
            let before = jump_consistent_hash(key, buckets_before);
            let after = jump_consistent_hash(key, buckets_after);
            if before != after {
                moved += 1;
            }
        }

        let ratio = moved as f64 / num_keys as f64;
        let ideal = 1.0 / buckets_after as f64;
        eprintln!("Jump: moved {moved}/{num_keys} = {ratio:.4} (ideal: {ideal:.4})");

        assert!(
            (ratio - ideal).abs() < 0.01,
            "jump hash movement {ratio} too far from ideal {ideal}"
        );
    }

    #[test]
    fn jump_consistent_hash_distribution() {
        let num_buckets = 10u32;
        let num_keys = 100_000u64;
        let mut counts = vec![0usize; num_buckets as usize];

        for key in 0..num_keys {
            let bucket = jump_consistent_hash(key, num_buckets);
            counts[bucket as usize] += 1;
        }

        let mean = num_keys as f64 / num_buckets as f64;
        let max_deviation = counts
            .iter()
            .map(|&c| (c as f64 - mean).abs() / mean)
            .fold(0.0f64, f64::max);

        eprintln!("Jump hash distribution: {counts:?}");
        eprintln!("Max relative deviation: {max_deviation:.4}");

        assert!(
            max_deviation < 0.03,
            "distribution too uneven: max deviation {max_deviation}"
        );
    }

    #[test]
    fn memento_hash_basic() {
        let mut m = MementoHash::new(10);
        assert_eq!(m.working_set_size(), 10);

        // 任意のキーが常に範囲内のバケットへ写る
        for key in 0..1000u64 {
            let b = m.get_bucket(&key);
            assert!(b < 10, "bucket {b} out of range");
        }

        // 同じキーは同じバケットへ
        let b1 = m.get_bucket(&"hello");
        let b2 = m.get_bucket(&"hello");
        assert_eq!(b1, b2);
    }

    #[test]
    fn memento_hash_arbitrary_removal() {
        // Jumpでは不可能だった「末尾以外」の削除
        let mut m = MementoHash::new(10);
        let num_keys = 10_000u64;

        // 削除前のマッピング
        let before: Vec<u32> = (0..num_keys).map(|k| m.get_bucket(&k)).collect();

        // 中央のバケット3を削除
        m.remove_bucket(3);
        assert_eq!(m.working_set_size(), 9);

        let mut redirected = 0;
        let mut stable = 0;
        for (i, &was) in before.iter().enumerate() {
            let now = m.get_bucket(&(i as u64));
            assert_ne!(now, 3, "key {i} still maps to removed bucket 3");
            if was == 3 {
                redirected += 1;
            } else {
                assert_eq!(was, now, "key {i} moved unnecessarily: {was} → {now}");
                stable += 1;
            }
        }
        eprintln!("Memento: redirected {redirected}, stable {stable}");
        // 削除されたバケットの担当キーは約1/10
        assert!(redirected > 0);
        assert!(stable > num_keys as usize * 8 / 10);
    }

    #[test]
    fn memento_hash_add_back() {
        // 削除したバケットを復元すると、元のマッピングに戻る
        let mut m = MementoHash::new(8);
        let num_keys = 5_000u64;
        let before: Vec<u32> = (0..num_keys).map(|k| m.get_bucket(&k)).collect();

        m.remove_bucket(5);
        let _added = m.add_bucket();
        // 復元後、現在のworking_set_sizeは元通り
        assert_eq!(m.working_set_size(), 8);

        for (i, &was) in before.iter().enumerate() {
            let now = m.get_bucket(&(i as u64));
            assert_eq!(was, now, "key {i} did not return to original after restore");
        }
    }

    #[test]
    fn remove_node_redistributes() {
        let mut ring = HashRing::new(100);
        ring.add_node("node-A");
        ring.add_node("node-B");
        ring.add_node("node-C");

        let num_keys = 10_000;
        let before: Vec<Option<String>> = (0..num_keys)
            .map(|i| ring.get_node(&format!("key-{i}")).map(String::from))
            .collect();

        ring.remove_node("node-B");

        let mut reassigned_correctly = 0;
        for (i, was) in before.iter().enumerate() {
            let key = format!("key-{i}");
            let after = ring.get_node(&key).map(String::from);

            match (was, &after) {
                (Some(w), Some(a)) => {
                    if w == "node-B" {
                        assert!(a == "node-A" || a == "node-C");
                        reassigned_correctly += 1;
                    } else {
                        assert_eq!(w, a, "key-{i} moved from {w} to {a} unnecessarily");
                    }
                }
                _ => panic!("unexpected None for key-{i}"),
            }
        }

        eprintln!("Reassigned from removed node: {reassigned_correctly}");
        assert!(reassigned_correctly > 0);
    }
}
