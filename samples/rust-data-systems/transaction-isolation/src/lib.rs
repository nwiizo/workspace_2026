use std::collections::BTreeMap;

/// Transaction ID (monotonically increasing).
type TxId = u64;

/// A versioned value in the MVCC store.
#[derive(Debug, Clone)]
struct Version {
    value: Option<String>, // None = deleted
    created_by: TxId,
    #[allow(dead_code)]
    deleted_by: Option<TxId>,
}

/// Transaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxState {
    Active,
    Committed,
    Aborted,
}

/// A simple MVCC key-value store.
/// Supports Read Committed and Snapshot Isolation levels.
pub struct MvccStore {
    /// key -> list of versions (newest last)
    data: BTreeMap<String, Vec<Version>>,
    /// Transaction states
    tx_states: BTreeMap<TxId, TxState>,
    /// Snapshot for each active transaction (set of committed TxIds at start time)
    tx_snapshots: BTreeMap<TxId, Vec<TxId>>,
    /// Read set for SSI: (tx_id -> keys read)
    tx_reads: BTreeMap<TxId, Vec<String>>,
    /// Write set for SSI: (tx_id -> keys written)
    tx_writes: BTreeMap<TxId, Vec<String>>,
    next_tx_id: TxId,
}

#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("transaction {0} not found or not active")]
    NotActive(TxId),
    #[error("key '{0}' not found")]
    KeyNotFound(String),
    #[error("write conflict: key '{key}' modified by tx {other_tx}")]
    WriteConflict { key: String, other_tx: TxId },
    #[error("serialization failure: tx {0} has rw-dependency conflict")]
    SerializationFailure(TxId),
}

/// Isolation level for read operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Each read sees the latest committed value at the time of that read.
    ReadCommitted,
    /// All reads see a consistent snapshot taken at transaction start.
    SnapshotIsolation,
}

impl MvccStore {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            tx_states: BTreeMap::new(),
            tx_snapshots: BTreeMap::new(),
            tx_reads: BTreeMap::new(),
            tx_writes: BTreeMap::new(),
            next_tx_id: 1,
        }
    }

    /// Begin a new transaction. Returns the transaction ID.
    pub fn begin(&mut self) -> TxId {
        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;
        self.tx_states.insert(tx_id, TxState::Active);

        // Snapshot: all currently committed transactions
        let committed: Vec<TxId> = self
            .tx_states
            .iter()
            .filter(|(_, state)| **state == TxState::Committed)
            .map(|(&id, _)| id)
            .collect();
        self.tx_snapshots.insert(tx_id, committed);
        self.tx_reads.insert(tx_id, Vec::new());
        self.tx_writes.insert(tx_id, Vec::new());

        tx_id
    }

    /// Read a key under the given isolation level.
    pub fn read(
        &mut self,
        tx_id: TxId,
        key: &str,
        isolation: IsolationLevel,
    ) -> Result<Option<String>, TxError> {
        if self.tx_states.get(&tx_id) != Some(&TxState::Active) {
            return Err(TxError::NotActive(tx_id));
        }

        // Record the read for SSI
        if let Some(reads) = self.tx_reads.get_mut(&tx_id) {
            reads.push(key.to_string());
        }

        let versions = match self.data.get(key) {
            Some(v) => v,
            None => return Ok(None),
        };

        match isolation {
            IsolationLevel::ReadCommitted => {
                // See the latest committed version
                for version in versions.iter().rev() {
                    if self.tx_states.get(&version.created_by) == Some(&TxState::Committed) {
                        return Ok(version.value.clone());
                    }
                    // Also see own writes
                    if version.created_by == tx_id {
                        return Ok(version.value.clone());
                    }
                }
                Ok(None)
            }
            IsolationLevel::SnapshotIsolation => {
                let snapshot = self.tx_snapshots.get(&tx_id).cloned().unwrap_or_default();
                // See the latest version created by a tx in our snapshot, or by ourselves
                for version in versions.iter().rev() {
                    if version.created_by == tx_id {
                        return Ok(version.value.clone());
                    }
                    if snapshot.contains(&version.created_by) {
                        return Ok(version.value.clone());
                    }
                }
                Ok(None)
            }
        }
    }

    /// Write a key-value pair.
    pub fn write(&mut self, tx_id: TxId, key: &str, value: &str) -> Result<(), TxError> {
        if self.tx_states.get(&tx_id) != Some(&TxState::Active) {
            return Err(TxError::NotActive(tx_id));
        }

        let versions = self.data.entry(key.to_string()).or_default();
        versions.push(Version {
            value: Some(value.to_string()),
            created_by: tx_id,
            deleted_by: None,
        });

        if let Some(writes) = self.tx_writes.get_mut(&tx_id) {
            writes.push(key.to_string());
        }

        Ok(())
    }

    /// Commit a transaction. Under SI, checks for write-write conflicts.
    /// Under SSI, additionally checks for rw-dependency cycles.
    pub fn commit(&mut self, tx_id: TxId) -> Result<(), TxError> {
        if self.tx_states.get(&tx_id) != Some(&TxState::Active) {
            return Err(TxError::NotActive(tx_id));
        }

        // Check for write-write conflicts (first-committer-wins)
        let my_writes = self.tx_writes.get(&tx_id).cloned().unwrap_or_default();
        let my_snapshot = self.tx_snapshots.get(&tx_id).cloned().unwrap_or_default();

        for key in &my_writes {
            if let Some(versions) = self.data.get(key) {
                for version in versions {
                    // If another tx wrote this key after our snapshot and committed
                    if version.created_by != tx_id
                        && !my_snapshot.contains(&version.created_by)
                        && self.tx_states.get(&version.created_by) == Some(&TxState::Committed)
                    {
                        self.tx_states.insert(tx_id, TxState::Aborted);
                        return Err(TxError::WriteConflict {
                            key: key.clone(),
                            other_tx: version.created_by,
                        });
                    }
                }
            }
        }

        self.tx_states.insert(tx_id, TxState::Committed);
        Ok(())
    }

    /// Commit with SSI validation: detects rw-dependency conflicts.
    pub fn commit_ssi(&mut self, tx_id: TxId) -> Result<(), TxError> {
        if self.tx_states.get(&tx_id) != Some(&TxState::Active) {
            return Err(TxError::NotActive(tx_id));
        }

        let my_reads = self.tx_reads.get(&tx_id).cloned().unwrap_or_default();
        let my_snapshot = self.tx_snapshots.get(&tx_id).cloned().unwrap_or_default();

        // Check: did any concurrent tx write to keys we read?
        for key in &my_reads {
            if let Some(versions) = self.data.get(key) {
                for version in versions {
                    if version.created_by != tx_id
                        && !my_snapshot.contains(&version.created_by)
                        && self.tx_states.get(&version.created_by) == Some(&TxState::Committed)
                    {
                        self.tx_states.insert(tx_id, TxState::Aborted);
                        return Err(TxError::SerializationFailure(tx_id));
                    }
                }
            }
        }

        // Also do write-write check
        let my_writes = self.tx_writes.get(&tx_id).cloned().unwrap_or_default();
        for key in &my_writes {
            if let Some(versions) = self.data.get(key) {
                for version in versions {
                    if version.created_by != tx_id
                        && !my_snapshot.contains(&version.created_by)
                        && self.tx_states.get(&version.created_by) == Some(&TxState::Committed)
                    {
                        self.tx_states.insert(tx_id, TxState::Aborted);
                        return Err(TxError::WriteConflict {
                            key: key.clone(),
                            other_tx: version.created_by,
                        });
                    }
                }
            }
        }

        self.tx_states.insert(tx_id, TxState::Committed);
        Ok(())
    }

    pub fn abort(&mut self, tx_id: TxId) {
        self.tx_states.insert(tx_id, TxState::Aborted);
    }

    /// Cahillに準じた SSI バリアント: rw-antidependencyを in/out の両方向で追跡し、
    /// 「pivot」（in と out の両方を持つトランザクション）でのみアボートする。
    /// `commit_ssi` は out 方向だけを見るので read-only でも余計にアボートしてしまうが、
    /// この実装は read-only を安全に通せる。
    pub fn commit_ssi_cahill(&mut self, tx_id: TxId) -> Result<(), TxError> {
        if self.tx_states.get(&tx_id) != Some(&TxState::Active) {
            return Err(TxError::NotActive(tx_id));
        }

        let my_reads = self.tx_reads.get(&tx_id).cloned().unwrap_or_default();
        let my_writes = self.tx_writes.get(&tx_id).cloned().unwrap_or_default();
        let my_snapshot = self.tx_snapshots.get(&tx_id).cloned().unwrap_or_default();

        // 自分から出ていく rw-antidependency: 読んだキーに、自分のスナップショット後に
        // コミット済みの他トランザクションが書いていないか
        let mut out_conflict = false;
        for key in &my_reads {
            if let Some(versions) = self.data.get(key) {
                for v in versions {
                    if v.created_by != tx_id
                        && !my_snapshot.contains(&v.created_by)
                        && self.tx_states.get(&v.created_by) == Some(&TxState::Committed)
                    {
                        out_conflict = true;
                        break;
                    }
                }
            }
            if out_conflict {
                break;
            }
        }

        // 自分に入ってくる rw-antidependency: 書いたキーを、自分のスナップショット後に
        // コミット済みの他トランザクションが読んでいたか
        let mut in_conflict = false;
        for key in &my_writes {
            for (&other_tx, reads) in &self.tx_reads {
                if other_tx == tx_id {
                    continue;
                }
                if self.tx_states.get(&other_tx) != Some(&TxState::Committed) {
                    continue;
                }
                if my_snapshot.contains(&other_tx) {
                    continue;
                }
                if reads.iter().any(|k| k == key) {
                    in_conflict = true;
                    break;
                }
            }
            if in_conflict {
                break;
            }
        }

        // Cahillの危険構造: pivot は in と out を両方持つ
        if in_conflict && out_conflict {
            self.tx_states.insert(tx_id, TxState::Aborted);
            return Err(TxError::SerializationFailure(tx_id));
        }

        // write-writeチェックは普通に実施
        for key in &my_writes {
            if let Some(versions) = self.data.get(key) {
                for v in versions {
                    if v.created_by != tx_id
                        && !my_snapshot.contains(&v.created_by)
                        && self.tx_states.get(&v.created_by) == Some(&TxState::Committed)
                    {
                        self.tx_states.insert(tx_id, TxState::Aborted);
                        return Err(TxError::WriteConflict {
                            key: key.clone(),
                            other_tx: v.created_by,
                        });
                    }
                }
            }
        }

        self.tx_states.insert(tx_id, TxState::Committed);
        Ok(())
    }

    /// Directly set a committed value (for test setup).
    pub fn setup_value(&mut self, key: &str, value: &str) {
        let tx = self.begin();
        self.write(tx, key, value)
            .expect("setup write should succeed");
        self.commit(tx).expect("setup commit should succeed");
    }
}

impl Default for MvccStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Dirty Read tests ---

    #[test]
    fn read_committed_prevents_dirty_read() {
        let mut store = MvccStore::new();
        store.setup_value("x", "original");

        let tx1 = store.begin();
        let tx2 = store.begin();

        // tx1 writes but hasn't committed
        store.write(tx1, "x", "dirty-value").unwrap();

        // tx2 should NOT see tx1's uncommitted write
        let val = store.read(tx2, "x", IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(val.as_deref(), Some("original"), "dirty read occurred!");

        store.commit(tx1).unwrap();

        // Now tx2 CAN see it (Read Committed sees latest committed)
        let val = store.read(tx2, "x", IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(val.as_deref(), Some("dirty-value"));
    }

    // --- Read Skew (Non-repeatable Read) tests ---

    #[test]
    fn read_committed_allows_read_skew() {
        let mut store = MvccStore::new();
        store.setup_value("x", "1");
        store.setup_value("y", "1");

        let tx1 = store.begin();
        let tx2 = store.begin();

        // tx1 reads x
        let x = store.read(tx1, "x", IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(x.as_deref(), Some("1"));

        // tx2 updates both x and y, then commits
        store.write(tx2, "x", "2").unwrap();
        store.write(tx2, "y", "2").unwrap();
        store.commit(tx2).unwrap();

        // tx1 reads y -- sees updated value (inconsistent with x read)
        let y = store.read(tx1, "y", IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(y.as_deref(), Some("2"), "Read Committed allows read skew");
    }

    #[test]
    fn snapshot_isolation_prevents_read_skew() {
        let mut store = MvccStore::new();
        store.setup_value("x", "1");
        store.setup_value("y", "1");

        let tx1 = store.begin();
        let tx2 = store.begin();

        // tx1 reads x (snapshot from begin time)
        let x = store
            .read(tx1, "x", IsolationLevel::SnapshotIsolation)
            .unwrap();
        assert_eq!(x.as_deref(), Some("1"));

        // tx2 updates both and commits
        store.write(tx2, "x", "2").unwrap();
        store.write(tx2, "y", "2").unwrap();
        store.commit(tx2).unwrap();

        // tx1 reads y -- still sees snapshot value
        let y = store
            .read(tx1, "y", IsolationLevel::SnapshotIsolation)
            .unwrap();
        assert_eq!(
            y.as_deref(),
            Some("1"),
            "Snapshot Isolation should prevent read skew"
        );
    }

    // --- Write Skew tests ---

    #[test]
    fn snapshot_isolation_allows_write_skew() {
        // Scenario: Two doctors on call. Policy: at least one must be on call.
        // Both read "2 doctors on call" and each decides to go off call.
        let mut store = MvccStore::new();
        store.setup_value("alice_oncall", "true");
        store.setup_value("bob_oncall", "true");

        let tx_alice = store.begin();
        let tx_bob = store.begin();

        // Alice checks: both on call? yes (2 >= 1, safe to leave)
        let alice_on = store
            .read(tx_alice, "alice_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        let bob_on = store
            .read(tx_alice, "bob_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        assert_eq!(alice_on.as_deref(), Some("true"));
        assert_eq!(bob_on.as_deref(), Some("true"));

        // Bob checks: both on call? yes
        let alice_on = store
            .read(tx_bob, "alice_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        let bob_on = store
            .read(tx_bob, "bob_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        assert_eq!(alice_on.as_deref(), Some("true"));
        assert_eq!(bob_on.as_deref(), Some("true"));

        // Alice goes off call
        store.write(tx_alice, "alice_oncall", "false").unwrap();
        // Bob goes off call
        store.write(tx_bob, "bob_oncall", "false").unwrap();

        // Both commit -- no write-write conflict (different keys!)
        assert!(store.commit(tx_alice).is_ok());
        assert!(store.commit(tx_bob).is_ok());

        // Result: nobody on call -- write skew!
        let check_tx = store.begin();
        let alice = store
            .read(check_tx, "alice_oncall", IsolationLevel::ReadCommitted)
            .unwrap();
        let bob = store
            .read(check_tx, "bob_oncall", IsolationLevel::ReadCommitted)
            .unwrap();
        assert_eq!(alice.as_deref(), Some("false"));
        assert_eq!(bob.as_deref(), Some("false"));
        eprintln!("Write skew: both doctors off call!");
    }

    #[test]
    fn ssi_prevents_write_skew() {
        let mut store = MvccStore::new();
        store.setup_value("alice_oncall", "true");
        store.setup_value("bob_oncall", "true");

        let tx_alice = store.begin();
        let tx_bob = store.begin();

        // Both read both keys
        store
            .read(tx_alice, "alice_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        store
            .read(tx_alice, "bob_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        store
            .read(tx_bob, "alice_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();
        store
            .read(tx_bob, "bob_oncall", IsolationLevel::SnapshotIsolation)
            .unwrap();

        // Alice goes off call, Bob goes off call
        store.write(tx_alice, "alice_oncall", "false").unwrap();
        store.write(tx_bob, "bob_oncall", "false").unwrap();

        // Alice commits first with SSI
        assert!(store.commit_ssi(tx_alice).is_ok());

        // Bob's commit should fail: Alice wrote alice_oncall which Bob read,
        // and Alice committed after Bob's snapshot
        let bob_result = store.commit_ssi(tx_bob);
        assert!(
            bob_result.is_err(),
            "SSI should detect write skew and abort Bob"
        );

        eprintln!("SSI correctly prevented write skew: {:?}", bob_result.err());
    }

    #[test]
    fn cahill_ssi_prevents_write_skew() {
        // Cahill版SSIも書き込み歪曲を検出すること
        let mut store = MvccStore::new();
        store.setup_value("alice_oncall", "true");
        store.setup_value("bob_oncall", "true");

        let tx_alice = store.begin();
        let tx_bob = store.begin();

        for tx in [tx_alice, tx_bob] {
            store.read(tx, "alice_oncall", IsolationLevel::SnapshotIsolation).unwrap();
            store.read(tx, "bob_oncall", IsolationLevel::SnapshotIsolation).unwrap();
        }
        store.write(tx_alice, "alice_oncall", "false").unwrap();
        store.write(tx_bob, "bob_oncall", "false").unwrap();

        assert!(store.commit_ssi_cahill(tx_alice).is_ok());
        // Bobはpivot（読んだalice_oncallが書かれ、書いたbob_oncallがAliceに読まれた）
        let bob_result = store.commit_ssi_cahill(tx_bob);
        assert!(bob_result.is_err(), "Cahill SSI should detect write skew");
    }

    #[test]
    fn cahill_ssi_allows_readonly_under_concurrent_writes() {
        // out_conflictだけ持つread-onlyトランザクションは、
        // 単純なcommit_ssiでは誤アボートするが、Cahill版では通る
        let mut store = MvccStore::new();
        store.setup_value("x", "1");
        store.setup_value("y", "1");

        let tx_reader = store.begin();
        let tx_writer = store.begin();

        // Readerはx, yを読むだけ。Writerはxを書く。
        store.read(tx_reader, "x", IsolationLevel::SnapshotIsolation).unwrap();
        store.read(tx_reader, "y", IsolationLevel::SnapshotIsolation).unwrap();
        store.write(tx_writer, "x", "2").unwrap();
        assert!(store.commit(tx_writer).is_ok());

        // 単純なSSIではReaderの読みxがWriterに書かれているのでアボートする
        let simple = {
            // 別ストアでcommit_ssiの挙動を確認
            let mut s2 = MvccStore::new();
            s2.setup_value("x", "1");
            s2.setup_value("y", "1");
            let r = s2.begin();
            let w = s2.begin();
            s2.read(r, "x", IsolationLevel::SnapshotIsolation).unwrap();
            s2.read(r, "y", IsolationLevel::SnapshotIsolation).unwrap();
            s2.write(w, "x", "2").unwrap();
            s2.commit(w).unwrap();
            s2.commit_ssi(r)
        };
        assert!(simple.is_err(), "naive SSI aborts read-only (false positive)");

        // Cahill版はReader.writesが空なのでin_conflict=false → 通る
        let cahill = store.commit_ssi_cahill(tx_reader);
        assert!(cahill.is_ok(), "Cahill SSI should allow read-only tx");
    }

    // --- Write-Write Conflict ---

    #[test]
    fn si_detects_write_write_conflict() {
        let mut store = MvccStore::new();
        store.setup_value("counter", "0");

        let tx1 = store.begin();
        let tx2 = store.begin();

        // Both update the same key
        store.write(tx1, "counter", "1").unwrap();
        store.write(tx2, "counter", "2").unwrap();

        // First committer wins
        assert!(store.commit(tx1).is_ok());
        assert!(
            store.commit(tx2).is_err(),
            "second writer should be rejected"
        );
    }

    // --- Phantom Read ---

    #[test]
    fn demonstrate_phantom_like_behavior() {
        // Simulate: meeting room booking
        // tx1 checks if room is free 10-11, then books it
        // tx2 does the same concurrently
        let mut store = MvccStore::new();
        store.setup_value("room_bookings_count", "0");

        let tx1 = store.begin();
        let tx2 = store.begin();

        // Both check room availability (read the same key)
        let count1 = store
            .read(
                tx1,
                "room_bookings_count",
                IsolationLevel::SnapshotIsolation,
            )
            .unwrap();
        let count2 = store
            .read(
                tx2,
                "room_bookings_count",
                IsolationLevel::SnapshotIsolation,
            )
            .unwrap();
        assert_eq!(count1.as_deref(), Some("0"));
        assert_eq!(count2.as_deref(), Some("0"));

        // Both book the room (different booking keys, but update count)
        store.write(tx1, "booking_1", "10:00-11:00").unwrap();
        store.write(tx1, "room_bookings_count", "1").unwrap();

        store.write(tx2, "booking_2", "10:00-11:00").unwrap();
        store.write(tx2, "room_bookings_count", "1").unwrap();

        // First committer wins (write-write conflict on count)
        assert!(store.commit(tx1).is_ok());
        assert!(
            store.commit(tx2).is_err(),
            "phantom-like: concurrent booking should conflict"
        );
    }

    // --- MVCC Version Chain ---

    #[test]
    fn mvcc_maintains_version_history() {
        let mut store = MvccStore::new();

        // Create 3 versions of the same key
        for i in 1..=3 {
            let tx = store.begin();
            store.write(tx, "key", &format!("v{i}")).unwrap();
            store.commit(tx).unwrap();
        }

        // A snapshot from before all writes should see nothing
        // (we can't test this easily without time travel, but we can verify
        //  the latest reader sees the latest committed version)
        let reader = store.begin();
        let val = store
            .read(reader, "key", IsolationLevel::SnapshotIsolation)
            .unwrap();
        assert_eq!(val.as_deref(), Some("v3"));
    }

    // --- Isolation Level Comparison ---

    #[test]
    fn isolation_level_comparison_table() {
        eprintln!("\nIsolation Level Comparison:");
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Anomaly", "Read Committed", "Snapshot", "SSI"
        );
        eprintln!("{}", "-".repeat(79));
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Dirty Read", "Prevented", "Prevented", "Prevented"
        );
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Read Skew", "POSSIBLE", "Prevented", "Prevented"
        );
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Write-Write Conflict", "N/A (no MVCC)", "Detected", "Detected"
        );
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Write Skew", "POSSIBLE", "POSSIBLE", "Prevented"
        );
        eprintln!(
            "{:<25} {:<18} {:<18} {:<18}",
            "Phantom", "POSSIBLE", "Partial", "Prevented"
        );
    }

    #[test]
    fn ssi_abort_rate_under_low_contention() {
        let mut store = MvccStore::new();

        // Setup 100 keys
        for i in 0..100 {
            store.setup_value(&format!("key-{i}"), "0");
        }

        let num_transactions = 1000;
        let mut committed = 0;
        let mut aborted = 0;

        // Each transaction reads and writes 2 random-ish keys (low contention)
        for i in 0..num_transactions {
            let tx = store.begin();
            let k1 = format!("key-{}", i % 100);
            let k2 = format!("key-{}", (i * 7 + 13) % 100);

            let _ = store.read(tx, &k1, IsolationLevel::SnapshotIsolation);
            let _ = store.read(tx, &k2, IsolationLevel::SnapshotIsolation);
            let _ = store.write(tx, &k1, &format!("{i}"));

            match store.commit_ssi(tx) {
                Ok(()) => committed += 1,
                Err(_) => aborted += 1,
            }
        }

        let abort_rate = aborted as f64 / num_transactions as f64 * 100.0;
        eprintln!(
            "SSI abort rate: {aborted}/{num_transactions} = {abort_rate:.1}% \
             (committed={committed})"
        );

        // With sequential execution and low overlap, abort rate should be manageable
        // (our implementation runs "sequentially" in tests, so conflicts come from
        // overlapping key sets between consecutive transactions)
        assert!(abort_rate < 50.0, "SSI abort rate too high: {abort_rate}%");
    }
}
