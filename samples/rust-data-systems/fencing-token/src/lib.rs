use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A lease without fencing -- vulnerable to split-brain.
pub struct NaiveLease {
    holder: Option<String>,
    expires_at: Option<Instant>,
    ttl: Duration,
}

impl NaiveLease {
    pub fn new(ttl: Duration) -> Self {
        Self {
            holder: None,
            expires_at: None,
            ttl,
        }
    }

    /// Attempt to acquire the lease.
    pub fn acquire(&mut self, client_id: &str, now: Instant) -> bool {
        if let Some(expires) = self.expires_at
            && now < expires
            && self.holder.as_deref() != Some(client_id)
        {
            return false; // someone else holds it and it hasn't expired
        }
        self.holder = Some(client_id.to_string());
        self.expires_at = Some(now + self.ttl);
        true
    }

    pub fn holder(&self) -> Option<&str> {
        self.holder.as_deref()
    }
}

/// Storage that accepts writes from anyone -- no fencing.
pub struct UnfencedStorage {
    data: std::collections::HashMap<String, (String, String)>, // key -> (value, writer)
    write_log: Vec<(String, String, String)>,                  // (key, value, writer)
}

impl UnfencedStorage {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
            write_log: Vec::new(),
        }
    }

    pub fn write(&mut self, key: &str, value: &str, writer: &str) {
        self.data
            .insert(key.to_string(), (value.to_string(), writer.to_string()));
        self.write_log
            .push((key.to_string(), value.to_string(), writer.to_string()));
    }

    pub fn get(&self, key: &str) -> Option<(&str, &str)> {
        self.data.get(key).map(|(v, w)| (v.as_str(), w.as_str()))
    }

    pub fn write_log(&self) -> &[(String, String, String)] {
        &self.write_log
    }
}

impl Default for UnfencedStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// A fencing token issuer -- monotonically increasing epoch numbers.
pub struct FencingTokenIssuer {
    next_epoch: u64,
}

impl FencingTokenIssuer {
    pub fn new() -> Self {
        Self { next_epoch: 1 }
    }

    /// Issue a new fencing token (lease + epoch).
    pub fn issue(&mut self, client_id: &str) -> FencingToken {
        let epoch = self.next_epoch;
        self.next_epoch += 1;
        FencingToken {
            client_id: client_id.to_string(),
            epoch,
        }
    }

    pub fn current_epoch(&self) -> u64 {
        self.next_epoch - 1
    }
}

impl Default for FencingTokenIssuer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FencingToken {
    pub client_id: String,
    pub epoch: u64,
}

/// Storage that validates fencing tokens before accepting writes.
pub struct FencedStorage {
    data: std::collections::HashMap<String, (String, u64)>, // key -> (value, epoch)
    max_epoch_seen: u64,
    write_log: Vec<WriteResult>,
}

#[derive(Debug, Clone)]
pub struct WriteResult {
    pub key: String,
    pub value: String,
    pub epoch: u64,
    pub client_id: String,
    pub accepted: bool,
}

#[derive(Debug)]
pub enum FencedWriteError {
    StaleEpoch { provided: u64, current_max: u64 },
}

impl std::fmt::Display for FencedWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleEpoch {
                provided,
                current_max,
            } => {
                write!(
                    f,
                    "stale epoch: provided {provided}, current max {current_max}"
                )
            }
        }
    }
}

impl std::error::Error for FencedWriteError {}

impl FencedStorage {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
            max_epoch_seen: 0,
            write_log: Vec::new(),
        }
    }

    /// Write with fencing token validation.
    /// Rejects writes from tokens with epoch <= max epoch seen.
    pub fn write(
        &mut self,
        key: &str,
        value: &str,
        token: &FencingToken,
    ) -> Result<(), FencedWriteError> {
        if token.epoch <= self.max_epoch_seen {
            self.write_log.push(WriteResult {
                key: key.to_string(),
                value: value.to_string(),
                epoch: token.epoch,
                client_id: token.client_id.clone(),
                accepted: false,
            });
            Err(FencedWriteError::StaleEpoch {
                provided: token.epoch,
                current_max: self.max_epoch_seen,
            })
        } else {
            self.max_epoch_seen = token.epoch;
            self.data
                .insert(key.to_string(), (value.to_string(), token.epoch));
            self.write_log.push(WriteResult {
                key: key.to_string(),
                value: value.to_string(),
                epoch: token.epoch,
                client_id: token.client_id.clone(),
                accepted: true,
            });
            Ok(())
        }
    }

    pub fn get(&self, key: &str) -> Option<(&str, u64)> {
        self.data.get(key).map(|(v, e)| (v.as_str(), *e))
    }

    pub fn write_log(&self) -> &[WriteResult] {
        &self.write_log
    }

    pub fn max_epoch(&self) -> u64 {
        self.max_epoch_seen
    }
}

impl Default for FencedStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulate a frozen client scenario.
/// Returns the sequence of events and final storage state.
pub fn simulate_frozen_client() -> FrozenClientResult {
    let mut issuer = FencingTokenIssuer::new();
    let mut storage = FencedStorage::new();

    // Client A acquires lease with epoch 1
    let token_a = issuer.issue("client-A");

    // Client A writes successfully
    let w1 = storage.write("resource", "value-from-A-1", &token_a);

    // Client A freezes (GC pause, network partition, etc.)
    // Client B acquires lease with epoch 2
    let token_b = issuer.issue("client-B");

    // Client B writes successfully
    let w2 = storage.write("resource", "value-from-B", &token_b);

    // Client A unfreezes and tries to write with its old token (epoch 1)
    let w3 = storage.write("resource", "stale-value-from-A", &token_a);

    let final_value = storage.get("resource").map(|(v, _)| v.to_string());

    FrozenClientResult {
        write_1_accepted: w1.is_ok(),
        write_2_accepted: w2.is_ok(),
        stale_write_rejected: w3.is_err(),
        final_value,
        write_log: storage.write_log().to_vec(),
    }
}

pub struct FrozenClientResult {
    pub write_1_accepted: bool,
    pub write_2_accepted: bool,
    pub stale_write_rejected: bool,
    pub final_value: Option<String>,
    pub write_log: Vec<WriteResult>,
}

/// Simulate split-brain WITHOUT fencing.
pub fn simulate_split_brain_no_fencing() -> SplitBrainResult {
    let start = Instant::now();
    let mut lease = NaiveLease::new(Duration::from_secs(10));
    let mut storage = UnfencedStorage::new();

    // Client A acquires lease
    let acquired_a = lease.acquire("client-A", start);

    // Client A writes
    storage.write("resource", "value-A-1", "client-A");

    // Client A freezes. Lease expires after TTL.
    let after_expiry = start + Duration::from_secs(15);

    // Client B acquires lease (A's has expired)
    let acquired_b = lease.acquire("client-B", after_expiry);

    // Client B writes
    storage.write("resource", "value-B", "client-B");

    // Client A unfreezes and writes -- no fencing, so it succeeds!
    storage.write("resource", "stale-value-A", "client-A");

    let final_value = storage.get("resource").map(|(v, _)| v.to_string());

    SplitBrainResult {
        a_acquired: acquired_a,
        b_acquired: acquired_b,
        final_value,
        stale_write_accepted: true, // no way to prevent it without fencing
        write_log: storage
            .write_log()
            .iter()
            .map(|(k, v, w)| format!("{k}={v} by {w}"))
            .collect(),
    }
}

pub struct SplitBrainResult {
    pub a_acquired: bool,
    pub b_acquired: bool,
    pub final_value: Option<String>,
    pub stale_write_accepted: bool,
    pub write_log: Vec<String>,
}

/// ETag/version ベースの CAS ストレージ。S3 If-Match (2024) と同じ発想で、
/// 「書き込み時に *直近に読んだ* version を提示する」CASでガードする。
/// 真の fencing token と違い、リーダー世代ではなく「オブジェクトの世代」を守る。
pub struct CasStorage {
    data: HashMap<String, (String, u64)>, // value, version
}

#[derive(Debug, PartialEq, Eq)]
pub enum CasError {
    VersionMismatch { expected: u64, actual: u64 },
    NotFound,
}

impl CasStorage {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn create_if_absent(&mut self, key: &str, value: &str) -> Result<u64, CasError> {
        if self.data.contains_key(key) {
            return Err(CasError::VersionMismatch { expected: 0, actual: self.data[key].1 });
        }
        self.data.insert(key.to_string(), (value.to_string(), 1));
        Ok(1)
    }

    pub fn read(&self, key: &str) -> Option<(String, u64)> {
        self.data.get(key).map(|(v, ver)| (v.clone(), *ver))
    }

    /// 提示された `if_match_version` が現在のversionと一致する場合のみ更新。
    /// S3 PutObject の If-Match ヘッダと同じ semantics。
    pub fn cas_update(
        &mut self,
        key: &str,
        new_value: &str,
        if_match_version: u64,
    ) -> Result<u64, CasError> {
        let (_, current) = self.data.get(key).ok_or(CasError::NotFound)?;
        if *current != if_match_version {
            return Err(CasError::VersionMismatch {
                expected: if_match_version,
                actual: *current,
            });
        }
        let next = current + 1;
        self.data.insert(key.to_string(), (new_value.to_string(), next));
        Ok(next)
    }
}

impl Default for CasStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fencing_token_monotonic_increase() {
        let mut issuer = FencingTokenIssuer::new();
        let t1 = issuer.issue("a");
        let t2 = issuer.issue("b");
        let t3 = issuer.issue("c");
        assert_eq!(t1.epoch, 1);
        assert_eq!(t2.epoch, 2);
        assert_eq!(t3.epoch, 3);
    }

    #[test]
    fn fenced_storage_rejects_stale_writes() {
        let mut issuer = FencingTokenIssuer::new();
        let mut storage = FencedStorage::new();

        let old_token = issuer.issue("old-client");
        let new_token = issuer.issue("new-client");

        // New token writes first
        assert!(storage.write("key", "new-value", &new_token).is_ok());

        // Old token is rejected
        assert!(storage.write("key", "old-value", &old_token).is_err());

        // Value is still from new client
        let (val, epoch) = storage.get("key").expect("key should exist");
        assert_eq!(val, "new-value");
        assert_eq!(epoch, 2);
    }

    #[test]
    fn fenced_storage_accepts_newer_epochs() {
        let mut issuer = FencingTokenIssuer::new();
        let mut storage = FencedStorage::new();

        let t1 = issuer.issue("client-1");
        let t2 = issuer.issue("client-2");
        let t3 = issuer.issue("client-3");

        assert!(storage.write("key", "v1", &t1).is_ok());
        assert!(storage.write("key", "v2", &t2).is_ok());
        assert!(storage.write("key", "v3", &t3).is_ok());

        let (val, epoch) = storage.get("key").expect("key should exist");
        assert_eq!(val, "v3");
        assert_eq!(epoch, 3);
    }

    #[test]
    fn same_epoch_rejected_on_second_use() {
        let mut issuer = FencingTokenIssuer::new();
        let mut storage = FencedStorage::new();

        let token = issuer.issue("client");

        // First write succeeds
        assert!(storage.write("key", "v1", &token).is_ok());

        // Same epoch is now stale (epoch == max_epoch_seen, not >)
        assert!(storage.write("key", "v2", &token).is_err());
    }

    #[test]
    fn frozen_client_scenario() {
        let result = simulate_frozen_client();

        assert!(result.write_1_accepted, "initial write should succeed");
        assert!(result.write_2_accepted, "new leader write should succeed");
        assert!(
            result.stale_write_rejected,
            "stale write from frozen client must be rejected"
        );
        assert_eq!(
            result.final_value.as_deref(),
            Some("value-from-B"),
            "final value should be from client B"
        );

        // Verify write log
        assert_eq!(result.write_log.len(), 3);
        assert!(result.write_log[0].accepted);
        assert!(result.write_log[1].accepted);
        assert!(!result.write_log[2].accepted);
    }

    #[test]
    fn split_brain_without_fencing() {
        let result = simulate_split_brain_no_fencing();

        assert!(result.a_acquired, "client A should acquire lease");
        assert!(result.b_acquired, "client B should acquire expired lease");
        assert!(
            result.stale_write_accepted,
            "without fencing, stale writes are accepted"
        );
        assert_eq!(
            result.final_value.as_deref(),
            Some("stale-value-A"),
            "without fencing, stale value from A overwrites B"
        );

        eprintln!("Write log (no fencing):");
        for entry in &result.write_log {
            eprintln!("  {entry}");
        }
    }

    #[test]
    fn fencing_rejects_all_stale_writes_in_sequence() {
        let mut issuer = FencingTokenIssuer::new();
        let mut storage = FencedStorage::new();

        // Issue 10 tokens
        let tokens: Vec<_> = (0..10)
            .map(|i| issuer.issue(&format!("client-{i}")))
            .collect();

        // Only the latest token writes
        assert!(storage.write("key", "latest", &tokens[9]).is_ok());

        // All previous tokens should be rejected
        let mut rejected_count = 0;
        for token in &tokens[..9] {
            if storage.write("key", "stale", token).is_err() {
                rejected_count += 1;
            }
        }

        assert_eq!(rejected_count, 9, "all 9 stale tokens should be rejected");
    }

    #[test]
    fn unfenced_storage_accepts_everything() {
        let mut storage = UnfencedStorage::new();

        storage.write("key", "v1", "client-A");
        storage.write("key", "v2", "client-B");
        storage.write("key", "v3", "client-A"); // stale client, but accepted

        let (val, writer) = storage.get("key").expect("key should exist");
        assert_eq!(val, "v3");
        assert_eq!(writer, "client-A");
        assert_eq!(storage.write_log().len(), 3);
    }

    #[test]
    fn epoch_overhead_is_minimal() {
        let mut issuer = FencingTokenIssuer::new();
        let mut storage = FencedStorage::new();

        let iterations = 100_000;
        let token = issuer.issue("bench-client");

        // Measure write + epoch check overhead
        // (we issue a new token each time to avoid rejection)
        let start = Instant::now();
        for i in 0..iterations {
            let t = FencingToken {
                client_id: "bench".to_string(),
                epoch: token.epoch + i as u64 + 1,
            };
            let _ = storage.write("key", "val", &t);
        }
        let elapsed = start.elapsed();

        let per_op_ns = elapsed.as_nanos() / iterations as u128;
        eprintln!("Fenced write: {per_op_ns}ns per operation ({iterations} ops in {elapsed:?})");

        // Fencing check should add negligible overhead (< 1μs per op)
        assert!(
            per_op_ns < 1_000,
            "epoch check overhead too high: {per_op_ns}ns"
        );
    }

    #[test]
    fn cas_update_succeeds_on_match() {
        let mut s = CasStorage::new();
        let v1 = s.create_if_absent("k", "v1").unwrap();
        let v2 = s.cas_update("k", "v2", v1).unwrap();
        assert_eq!(v2, 2);
        assert_eq!(s.read("k"), Some(("v2".into(), 2)));
    }

    #[test]
    fn cas_update_rejects_stale_version() {
        // 2クライアントが同じ古いversionに基づいて書こうとする
        let mut s = CasStorage::new();
        s.create_if_absent("k", "v1").unwrap();

        // A, B どちらも version=1 を読んだとする
        let (_, ver_seen_by_a) = s.read("k").unwrap();
        let (_, ver_seen_by_b) = s.read("k").unwrap();

        // A が先に更新成功 → version=2
        s.cas_update("k", "v-from-A", ver_seen_by_a).unwrap();
        // B の更新は古い version で落ちる
        let err = s.cas_update("k", "v-from-B", ver_seen_by_b).unwrap_err();
        assert!(matches!(err, CasError::VersionMismatch { expected: 1, actual: 2 }));
        assert_eq!(s.read("k").unwrap().0, "v-from-A");
    }

    #[test]
    fn cas_does_not_protect_zombie_with_correct_version() {
        // CASは「正しいversionを持っている書き込み」は通してしまう。
        // fencing tokenとの本質的違い。
        let mut s = CasStorage::new();
        s.create_if_absent("k", "init").unwrap();
        let v = 1;

        // A が write しようとして frozen → 復帰したとき、まだ誰も更新していなければ通る
        let v2 = s.cas_update("k", "zombie-write-by-A", v).unwrap();
        assert_eq!(v2, 2);
        // 「A は実は古いリーダーだった」という情報がストレージ側にないので止められない
    }
}
