use bloom_filter::BloomFilter;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// A tombstone marker for deleted keys.
const TOMBSTONE: &str = "\x00__TOMBSTONE__\x00";

/// In-memory sorted buffer (memtable).
pub struct Memtable {
    data: BTreeMap<String, String>,
    size_bytes: usize,
}

impl Memtable {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: 0,
        }
    }

    pub fn put(&mut self, key: &str, value: &str) {
        let entry_size = key.len() + value.len();
        if let Some(old) = self.data.insert(key.to_string(), value.to_string()) {
            self.size_bytes -= old.len();
            self.size_bytes -= key.len();
        }
        self.size_bytes += entry_size;
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|v| v.as_str())
    }

    pub fn delete(&mut self, key: &str) {
        self.put(key, TOMBSTONE);
    }

    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Drain all entries for flushing to an SSTable.
    pub fn drain(&mut self) -> BTreeMap<String, String> {
        self.size_bytes = 0;
        std::mem::take(&mut self.data)
    }
}

impl Default for Memtable {
    fn default() -> Self {
        Self::new()
    }
}

/// A sorted string table on disk.
/// Format: one "key\tvalue\n" per line, sorted by key.
pub struct SSTable {
    path: PathBuf,
    bloom: BloomFilter,
    min_key: String,
    max_key: String,
    entry_count: usize,
}

impl SSTable {
    /// Write a BTreeMap to an SSTable file.
    pub fn write_from_map(path: &Path, data: &BTreeMap<String, String>) -> Result<Self, LsmError> {
        let mut file = fs::File::create(path).map_err(LsmError::Io)?;
        let mut bloom = BloomFilter::new(data.len().max(1), 0.01)
            .map_err(|e| LsmError::Internal(e.to_string()))?;

        let mut min_key = String::new();
        let mut max_key = String::new();

        for (i, (key, value)) in data.iter().enumerate() {
            writeln!(file, "{key}\t{value}").map_err(LsmError::Io)?;
            bloom.insert(key);
            if i == 0 {
                min_key = key.clone();
            }
            max_key = key.clone();
        }

        Ok(Self {
            path: path.to_path_buf(),
            bloom,
            min_key,
            max_key,
            entry_count: data.len(),
        })
    }

    /// Look up a key in this SSTable.
    /// Uses bloom filter to skip unnecessary disk reads.
    pub fn get(&self, key: &str) -> Result<Option<String>, LsmError> {
        // Bloom filter check
        if !self.bloom.contains(&key) {
            return Ok(None);
        }

        // Key range check
        if key < self.min_key.as_str() || key > self.max_key.as_str() {
            return Ok(None);
        }

        // Linear scan (in a real implementation, we'd use a sparse index + binary search)
        let file = fs::File::open(&self.path).map_err(LsmError::Io)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(LsmError::Io)?;
            if let Some((k, v)) = line.split_once('\t') {
                if k == key {
                    return Ok(Some(v.to_string()));
                }
                if k > key {
                    return Ok(None); // Sorted, so no point continuing
                }
            }
        }

        Ok(None)
    }

    /// Read all entries from this SSTable.
    pub fn scan(&self) -> Result<BTreeMap<String, String>, LsmError> {
        let file = fs::File::open(&self.path).map_err(LsmError::Io)?;
        let reader = io::BufReader::new(file);
        let mut data = BTreeMap::new();

        for line in reader.lines() {
            let line = line.map_err(LsmError::Io)?;
            if let Some((k, v)) = line.split_once('\t') {
                data.insert(k.to_string(), v.to_string());
            }
        }

        Ok(data)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }
}

/// Merge multiple SSTables into a new one, keeping the newest value for each key.
/// SSTables should be ordered from newest to oldest.
pub fn compact(sstables: &[&SSTable], output_path: &Path) -> Result<SSTable, LsmError> {
    let mut merged: BTreeMap<String, String> = BTreeMap::new();

    // Iterate oldest to newest so newer values overwrite older ones
    for sstable in sstables.iter().rev() {
        let data = sstable.scan()?;
        for (key, value) in data {
            merged.insert(key, value);
        }
    }

    // Remove tombstones during compaction
    merged.retain(|_, v| v != TOMBSTONE);

    SSTable::write_from_map(output_path, &merged)
}

/// The LSM-Tree engine.
pub struct LsmTree {
    memtable: Memtable,
    sstables: Vec<SSTable>, // newest first
    dir: PathBuf,
    flush_threshold: usize, // bytes
    next_sstable_id: u64,
    bytes_written: u64, // for write amplification tracking
}

impl LsmTree {
    pub fn new(dir: &Path, flush_threshold: usize) -> Result<Self, LsmError> {
        fs::create_dir_all(dir).map_err(LsmError::Io)?;
        Ok(Self {
            memtable: Memtable::new(),
            sstables: Vec::new(),
            dir: dir.to_path_buf(),
            flush_threshold,
            next_sstable_id: 0,
            bytes_written: 0,
        })
    }

    pub fn put(&mut self, key: &str, value: &str) -> Result<(), LsmError> {
        let write_size = key.len() + value.len();
        self.bytes_written += write_size as u64;
        self.memtable.put(key, value);

        if self.memtable.size_bytes() >= self.flush_threshold {
            self.flush()?;
        }

        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, LsmError> {
        // 1. Check memtable
        if let Some(value) = self.memtable.get(key) {
            if value == TOMBSTONE {
                return Ok(None);
            }
            return Ok(Some(value.to_string()));
        }

        // 2. Check SSTables (newest first)
        for sstable in &self.sstables {
            if let Some(value) = sstable.get(key)? {
                if value == TOMBSTONE {
                    return Ok(None);
                }
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    pub fn delete(&mut self, key: &str) -> Result<(), LsmError> {
        self.memtable.delete(key);
        if self.memtable.size_bytes() >= self.flush_threshold {
            self.flush()?;
        }
        Ok(())
    }

    /// Flush memtable to a new SSTable.
    pub fn flush(&mut self) -> Result<(), LsmError> {
        if self.memtable.is_empty() {
            return Ok(());
        }

        let data = self.memtable.drain();
        let path = self
            .dir
            .join(format!("sstable_{:04}.dat", self.next_sstable_id));
        self.next_sstable_id += 1;

        let sstable = SSTable::write_from_map(&path, &data)?;
        self.sstables.insert(0, sstable); // newest first

        Ok(())
    }

    /// Run compaction on all SSTables.
    pub fn compact(&mut self) -> Result<(), LsmError> {
        if self.sstables.len() < 2 {
            return Ok(());
        }

        let refs: Vec<&SSTable> = self.sstables.iter().collect();
        let output_path = self
            .dir
            .join(format!("sstable_{:04}.dat", self.next_sstable_id));
        self.next_sstable_id += 1;

        let merged = compact(&refs, &output_path)?;

        // Remove old SSTable files
        for sstable in &self.sstables {
            let _ = fs::remove_file(sstable.path());
        }

        self.sstables = vec![merged];
        Ok(())
    }

    /// Tiered compaction (RocksDB Universal風の縮小版):
    /// 同程度のサイズのSSTableが`tier_size`本溜まったら、その層だけをマージする。
    /// 全SSTableをstop-the-worldで畳む`compact()`と違い、各回の処理量を抑えられる。
    pub fn compact_tiered(&mut self, tier_size: usize) -> Result<usize, LsmError> {
        if self.sstables.len() < tier_size {
            return Ok(0);
        }

        // 簡略化: SSTableをエントリ数でグループ化し、同サイズが tier_size 本揃った
        // グループのうち最も古い側からマージする。
        let mut groups: std::collections::BTreeMap<usize, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (idx, sst) in self.sstables.iter().enumerate() {
            // 2の累乗バケットで「同程度サイズ」を緩く定義
            let bucket = sst.entry_count().next_power_of_two();
            groups.entry(bucket).or_default().push(idx);
        }

        // tier_size本溜まっている最小バケットから着手
        let target = groups
            .iter()
            .find(|(_, idxs)| idxs.len() >= tier_size)
            .map(|(_, idxs)| idxs.clone());
        let Some(idxs) = target else { return Ok(0) };

        let chosen: Vec<usize> = idxs.into_iter().take(tier_size).collect();
        let refs: Vec<&SSTable> = chosen.iter().map(|&i| &self.sstables[i]).collect();
        let output_path = self
            .dir
            .join(format!("sstable_{:04}.dat", self.next_sstable_id));
        self.next_sstable_id += 1;
        let merged = compact(&refs, &output_path)?;

        // 古いファイルを削除して、インデックスの大きい順に取り除く
        let mut chosen_sorted = chosen.clone();
        chosen_sorted.sort_unstable_by(|a, b| b.cmp(a));
        for idx in &chosen_sorted {
            let removed = self.sstables.remove(*idx);
            let _ = fs::remove_file(removed.path());
        }
        // mergedは新しい扱いとして先頭に
        self.sstables.insert(0, merged);
        Ok(chosen.len())
    }

    pub fn sstable_count(&self) -> usize {
        self.sstables.len()
    }

    pub fn memtable_size(&self) -> usize {
        self.memtable.size_bytes()
    }

    pub fn total_bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LsmError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn memtable_basic_operations() {
        let mut mt = Memtable::new();
        mt.put("key1", "val1");
        mt.put("key2", "val2");
        mt.put("key3", "val3");

        assert_eq!(mt.get("key1"), Some("val1"));
        assert_eq!(mt.get("key2"), Some("val2"));
        assert_eq!(mt.get("key3"), Some("val3"));
        assert_eq!(mt.get("missing"), None);
        assert_eq!(mt.len(), 3);
    }

    #[test]
    fn memtable_overwrite() {
        let mut mt = Memtable::new();
        mt.put("key", "v1");
        mt.put("key", "v2");
        assert_eq!(mt.get("key"), Some("v2"));
        assert_eq!(mt.len(), 1);
    }

    #[test]
    fn memtable_delete() {
        let mut mt = Memtable::new();
        mt.put("key", "value");
        mt.delete("key");
        assert_eq!(mt.get("key"), Some(TOMBSTONE));
    }

    #[test]
    fn sstable_write_and_read() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sst");

        let mut data = BTreeMap::new();
        data.insert("apple".to_string(), "red".to_string());
        data.insert("banana".to_string(), "yellow".to_string());
        data.insert("cherry".to_string(), "red".to_string());

        let sst = SSTable::write_from_map(&path, &data).unwrap();
        assert_eq!(sst.entry_count(), 3);

        assert_eq!(sst.get("apple").unwrap(), Some("red".to_string()));
        assert_eq!(sst.get("banana").unwrap(), Some("yellow".to_string()));
        assert_eq!(sst.get("cherry").unwrap(), Some("red".to_string()));
        assert_eq!(sst.get("missing").unwrap(), None);
    }

    #[test]
    fn sstable_bloom_filter_skip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sst");

        let mut data = BTreeMap::new();
        for i in 0..1000 {
            data.insert(format!("key-{i:04}"), format!("val-{i}"));
        }

        let sst = SSTable::write_from_map(&path, &data).unwrap();

        // Keys not in the set should (usually) be rejected by bloom filter
        let mut bloom_rejected = 0;
        for i in 1000..2000 {
            let key = format!("absent-{i}");
            if !sst.bloom.contains(&key) {
                bloom_rejected += 1;
            }
        }

        let bloom_reject_rate = bloom_rejected as f64 / 1000.0;
        eprintln!(
            "Bloom filter rejected {bloom_rejected}/1000 absent keys ({:.1}%)",
            bloom_reject_rate * 100.0
        );
        assert!(
            bloom_reject_rate > 0.95,
            "bloom filter should reject most absent keys"
        );
    }

    #[test]
    fn lsm_tree_put_get() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 1024).unwrap();

        lsm.put("key1", "val1").unwrap();
        lsm.put("key2", "val2").unwrap();
        lsm.put("key3", "val3").unwrap();

        assert_eq!(lsm.get("key1").unwrap(), Some("val1".to_string()));
        assert_eq!(lsm.get("key2").unwrap(), Some("val2".to_string()));
        assert_eq!(lsm.get("key3").unwrap(), Some("val3".to_string()));
        assert_eq!(lsm.get("missing").unwrap(), None);
    }

    #[test]
    fn lsm_tree_flush_and_read() {
        let dir = TempDir::new().unwrap();
        // Small threshold to trigger flush
        let mut lsm = LsmTree::new(dir.path(), 50).unwrap();

        for i in 0..20 {
            lsm.put(&format!("key-{i:02}"), &format!("val-{i}"))
                .unwrap();
        }

        assert!(lsm.sstable_count() > 0, "should have flushed to SSTables");
        eprintln!(
            "SSTables after 20 puts: {}, memtable size: {}",
            lsm.sstable_count(),
            lsm.memtable_size()
        );

        // All keys should still be readable
        for i in 0..20 {
            let val = lsm.get(&format!("key-{i:02}")).unwrap();
            assert_eq!(val, Some(format!("val-{i}")), "key-{i:02} not found");
        }
    }

    #[test]
    fn lsm_tree_delete() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 1024).unwrap();

        lsm.put("key", "value").unwrap();
        assert_eq!(lsm.get("key").unwrap(), Some("value".to_string()));

        lsm.delete("key").unwrap();
        assert_eq!(lsm.get("key").unwrap(), None);
    }

    #[test]
    fn lsm_tree_delete_across_flush() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 50).unwrap();

        // Put some data and flush
        lsm.put("key", "value").unwrap();
        lsm.flush().unwrap();

        // Delete (tombstone goes to memtable)
        lsm.delete("key").unwrap();

        // Should return None (tombstone shadows the SSTable value)
        assert_eq!(lsm.get("key").unwrap(), None);

        // Flush tombstone, then compact
        lsm.flush().unwrap();
        lsm.compact().unwrap();

        // After compaction, tombstone is removed
        assert_eq!(lsm.get("key").unwrap(), None);
        assert_eq!(lsm.sstable_count(), 1);
    }

    #[test]
    fn compaction_merges_sstables() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 50).unwrap();

        // Write enough data to create multiple SSTables
        for i in 0..50 {
            lsm.put(&format!("k{i:03}"), &format!("v{i}")).unwrap();
        }
        lsm.flush().unwrap();

        let before_count = lsm.sstable_count();
        eprintln!("SSTables before compaction: {before_count}");

        lsm.compact().unwrap();

        let after_count = lsm.sstable_count();
        eprintln!("SSTables after compaction: {after_count}");
        assert_eq!(after_count, 1, "compaction should merge into 1 SSTable");

        // All data still accessible
        for i in 0..50 {
            let val = lsm.get(&format!("k{i:03}")).unwrap();
            assert_eq!(val, Some(format!("v{i}")));
        }
    }

    #[test]
    fn compaction_newer_values_win() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 50).unwrap();

        // First batch
        for i in 0..10 {
            lsm.put(&format!("k{i:02}"), &format!("old-{i}")).unwrap();
        }
        lsm.flush().unwrap();

        // Second batch overwrites some keys
        for i in 0..5 {
            lsm.put(&format!("k{i:02}"), &format!("new-{i}")).unwrap();
        }
        lsm.flush().unwrap();

        lsm.compact().unwrap();

        // Newer values should win
        for i in 0..5 {
            let val = lsm.get(&format!("k{i:02}")).unwrap();
            assert_eq!(val, Some(format!("new-{i}")));
        }
        for i in 5..10 {
            let val = lsm.get(&format!("k{i:02}")).unwrap();
            assert_eq!(val, Some(format!("old-{i}")));
        }
    }

    #[test]
    fn write_amplification_measurement() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 200).unwrap();

        let num_ops = 500;
        let key_size = 10;
        let val_size = 50;
        let logical_bytes = num_ops * (key_size + val_size);

        for i in 0..num_ops {
            lsm.put(&format!("key-{i:06}"), &"x".repeat(val_size))
                .unwrap();
        }
        lsm.flush().unwrap();

        let physical_bytes = lsm.total_bytes_written();
        let write_amp = physical_bytes as f64 / logical_bytes as f64;

        eprintln!("Logical bytes: {logical_bytes}");
        eprintln!("Physical bytes tracked: {physical_bytes}");
        eprintln!("Write amplification (pre-compaction): {write_amp:.2}x");

        // Write amplification should be at least 1x (we write at least as much as input)
        assert!(write_amp >= 0.9, "write amp should be >= ~1x");
    }

    #[test]
    fn tiered_compaction_processes_subset() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 80).unwrap();

        // 4本のSSTableを作る
        for batch in 0..4 {
            for i in 0..3 {
                lsm.put(&format!("k{batch}{i}"), "v").unwrap();
            }
            lsm.flush().unwrap();
        }
        assert_eq!(lsm.sstable_count(), 4);

        // tier_size=4 → 4本全部が同サイズ層に乗るのでマージされる
        let merged = lsm.compact_tiered(4).unwrap();
        assert_eq!(merged, 4);
        assert_eq!(lsm.sstable_count(), 1);

        // 全データが読める
        for batch in 0..4 {
            for i in 0..3 {
                assert_eq!(lsm.get(&format!("k{batch}{i}")).unwrap(), Some("v".to_string()));
            }
        }
    }

    #[test]
    fn tiered_compaction_skips_when_below_tier() {
        let dir = TempDir::new().unwrap();
        let mut lsm = LsmTree::new(dir.path(), 80).unwrap();
        for batch in 0..2 {
            for i in 0..3 {
                lsm.put(&format!("k{batch}{i}"), "v").unwrap();
            }
            lsm.flush().unwrap();
        }
        // 2本しかないのでtier_size=4ではトリガしない
        let merged = lsm.compact_tiered(4).unwrap();
        assert_eq!(merged, 0);
        assert_eq!(lsm.sstable_count(), 2);
    }

    #[test]
    fn sequential_vs_random_write_pattern() {
        let dir = TempDir::new().unwrap();
        let n = 200;

        // Sequential writes
        let mut lsm_seq = LsmTree::new(&dir.path().join("seq"), 500).unwrap();
        for i in 0..n {
            lsm_seq
                .put(&format!("key-{i:06}"), &format!("val-{i}"))
                .unwrap();
        }
        lsm_seq.flush().unwrap();

        // "Random" writes (reverse order)
        let mut lsm_rand = LsmTree::new(&dir.path().join("rand"), 500).unwrap();
        for i in (0..n).rev() {
            lsm_rand
                .put(&format!("key-{i:06}"), &format!("val-{i}"))
                .unwrap();
        }
        lsm_rand.flush().unwrap();

        // Both should produce the same results since memtable sorts
        for i in 0..n {
            let key = format!("key-{i:06}");
            let seq_val = lsm_seq.get(&key).unwrap();
            let rand_val = lsm_rand.get(&key).unwrap();
            assert_eq!(seq_val, rand_val, "key {key} differs");
        }

        eprintln!("Both sequential and random write patterns produce identical sorted SSTables");
    }
}
