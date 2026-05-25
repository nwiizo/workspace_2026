use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Result of a payment operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentResult {
    Success { balance: i64 },
    InsufficientFunds { balance: i64 },
    DuplicateRequest { original_result: Box<PaymentResult> },
}

/// A naive payment service without idempotency protection.
/// Retries cause double-processing.
pub struct NaivePaymentService {
    balance: i64,
    operations: Vec<(String, i64)>, // (description, amount)
}

impl NaivePaymentService {
    pub fn new(initial_balance: i64) -> Self {
        Self {
            balance: initial_balance,
            operations: Vec::new(),
        }
    }

    /// Process a payment. No idempotency -- every call mutates state.
    pub fn charge(&mut self, amount: i64, description: &str) -> PaymentResult {
        if self.balance < amount {
            return PaymentResult::InsufficientFunds {
                balance: self.balance,
            };
        }
        self.balance -= amount;
        self.operations.push((description.to_string(), amount));
        PaymentResult::Success {
            balance: self.balance,
        }
    }

    pub fn balance(&self) -> i64 {
        self.balance
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

/// An idempotent payment service.
/// Uses idempotency keys to ensure exactly-once semantics.
pub struct IdempotentPaymentService {
    balance: i64,
    operations: Vec<(String, i64)>,
    /// Idempotency store: key -> cached result.
    /// In production, this would be in the same database transaction as the balance update.
    idempotency_store: HashMap<String, PaymentResult>,
}

impl IdempotentPaymentService {
    pub fn new(initial_balance: i64) -> Self {
        Self {
            balance: initial_balance,
            operations: Vec::new(),
            idempotency_store: HashMap::new(),
        }
    }

    /// Process a payment with idempotency key.
    /// If the key was seen before, return the cached result without mutation.
    pub fn charge(
        &mut self,
        idempotency_key: &str,
        amount: i64,
        description: &str,
    ) -> PaymentResult {
        // Check if we've already processed this request
        if let Some(cached) = self.idempotency_store.get(idempotency_key) {
            return PaymentResult::DuplicateRequest {
                original_result: Box::new(cached.clone()),
            };
        }

        // Process the payment
        let result = if self.balance < amount {
            PaymentResult::InsufficientFunds {
                balance: self.balance,
            }
        } else {
            self.balance -= amount;
            self.operations.push((description.to_string(), amount));
            PaymentResult::Success {
                balance: self.balance,
            }
        };

        // Store the result atomically with the business logic
        // (In production: same DB transaction)
        self.idempotency_store
            .insert(idempotency_key.to_string(), result.clone());

        result
    }

    pub fn balance(&self) -> i64 {
        self.balance
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

/// Demonstrates the TOCTOU problem when idempotency check and business logic
/// are in separate "transactions".
pub struct TocTouVulnerableService {
    balance: Arc<Mutex<i64>>,
    idempotency_store: Arc<Mutex<HashMap<String, PaymentResult>>>,
    operation_count: Arc<Mutex<usize>>,
}

impl TocTouVulnerableService {
    pub fn new(initial_balance: i64) -> Self {
        Self {
            balance: Arc::new(Mutex::new(initial_balance)),
            idempotency_store: Arc::new(Mutex::new(HashMap::new())),
            operation_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Check idempotency in one "transaction", process in another.
    /// The gap between check and insert is the TOCTOU window.
    pub fn charge_with_toctou_gap(&self, idempotency_key: &str, amount: i64) -> PaymentResult {
        // Step 1: Check idempotency store (separate "transaction")
        {
            let store = self.idempotency_store.lock().expect("lock poisoned");
            if let Some(cached) = store.get(idempotency_key) {
                return PaymentResult::DuplicateRequest {
                    original_result: Box::new(cached.clone()),
                };
            }
        }
        // <-- TOCTOU window: another thread could pass the check here

        // Step 2: Process payment (separate "transaction")
        let result = {
            let mut bal = self.balance.lock().expect("lock poisoned");
            if *bal < amount {
                PaymentResult::InsufficientFunds { balance: *bal }
            } else {
                *bal -= amount;
                let mut count = self.operation_count.lock().expect("lock poisoned");
                *count += 1;
                PaymentResult::Success { balance: *bal }
            }
        };

        // Step 3: Store idempotency record (separate "transaction")
        {
            let mut store = self.idempotency_store.lock().expect("lock poisoned");
            store.insert(idempotency_key.to_string(), result.clone());
        }

        result
    }

    pub fn balance(&self) -> i64 {
        *self.balance.lock().expect("lock poisoned")
    }

    pub fn operation_count(&self) -> usize {
        *self.operation_count.lock().expect("lock poisoned")
    }
}

impl Clone for TocTouVulnerableService {
    fn clone(&self) -> Self {
        Self {
            balance: Arc::clone(&self.balance),
            idempotency_store: Arc::clone(&self.idempotency_store),
            operation_count: Arc::clone(&self.operation_count),
        }
    }
}

/// Properly atomic idempotent service using a single lock.
pub struct AtomicIdempotentService {
    state: Arc<Mutex<ServiceState>>,
}

struct ServiceState {
    balance: i64,
    operation_count: usize,
    idempotency_store: HashMap<String, PaymentResult>,
}

impl AtomicIdempotentService {
    pub fn new(initial_balance: i64) -> Self {
        Self {
            state: Arc::new(Mutex::new(ServiceState {
                balance: initial_balance,
                operation_count: 0,
                idempotency_store: HashMap::new(),
            })),
        }
    }

    /// Idempotency check + business logic + result caching in one atomic operation.
    pub fn charge(&self, idempotency_key: &str, amount: i64) -> PaymentResult {
        let mut state = self.state.lock().expect("lock poisoned");

        // Check + process + store in one critical section
        if let Some(cached) = state.idempotency_store.get(idempotency_key) {
            return PaymentResult::DuplicateRequest {
                original_result: Box::new(cached.clone()),
            };
        }

        let result = if state.balance < amount {
            PaymentResult::InsufficientFunds {
                balance: state.balance,
            }
        } else {
            state.balance -= amount;
            state.operation_count += 1;
            PaymentResult::Success {
                balance: state.balance,
            }
        };

        state
            .idempotency_store
            .insert(idempotency_key.to_string(), result.clone());

        result
    }

    pub fn balance(&self) -> i64 {
        self.state.lock().expect("lock poisoned").balance
    }

    pub fn operation_count(&self) -> usize {
        self.state.lock().expect("lock poisoned").operation_count
    }
}

impl Clone for AtomicIdempotentService {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

/// Transactional Outbox pattern.
///
/// 「ビジネス状態の更新」と「外部に飛ばす副作用イベントの発行」を同一トランザクションで
/// 行うために、副作用を直接送らずouboxテーブルに書き込む。別プロセスがoutboxを読んで
/// 実際に送る。送信が冪等であれば、配送のat-least-onceを安全に吸収できる。
#[derive(Debug, Clone)]
pub struct OutboxEntry {
    pub id: u64,
    pub payload: String,
    pub published: bool,
}

pub struct OutboxService {
    state: Arc<Mutex<OutboxState>>,
}

struct OutboxState {
    balance: i64,
    idempotency_store: HashMap<String, OutboxResult>,
    outbox: Vec<OutboxEntry>,
    next_outbox_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxResult {
    Success { balance: i64, outbox_id: u64 },
    InsufficientFunds { balance: i64 },
    Duplicate { original_outbox_id: u64 },
}

impl OutboxService {
    pub fn new(initial_balance: i64) -> Self {
        Self {
            state: Arc::new(Mutex::new(OutboxState {
                balance: initial_balance,
                idempotency_store: HashMap::new(),
                outbox: Vec::new(),
                next_outbox_id: 1,
            })),
        }
    }

    /// 残高更新とoutbox追加を「同一トランザクション」（同一ロック）で行う。
    pub fn charge_and_emit(
        &self,
        idempotency_key: &str,
        amount: i64,
        event_payload: &str,
    ) -> OutboxResult {
        let mut state = self.state.lock().expect("lock poisoned");

        if let Some(cached) = state.idempotency_store.get(idempotency_key) {
            return match cached {
                OutboxResult::Success { outbox_id, .. } => OutboxResult::Duplicate {
                    original_outbox_id: *outbox_id,
                },
                other => other.clone(),
            };
        }

        if state.balance < amount {
            let r = OutboxResult::InsufficientFunds { balance: state.balance };
            state.idempotency_store.insert(idempotency_key.to_string(), r.clone());
            return r;
        }

        state.balance -= amount;
        let outbox_id = state.next_outbox_id;
        state.next_outbox_id += 1;
        state.outbox.push(OutboxEntry {
            id: outbox_id,
            payload: event_payload.to_string(),
            published: false,
        });

        let result = OutboxResult::Success {
            balance: state.balance,
            outbox_id,
        };
        state.idempotency_store.insert(idempotency_key.to_string(), result.clone());
        result
    }

    /// outbox publisher: 未発行のentryを返し、IDのリストでackする。
    pub fn drain_unpublished(&self) -> Vec<OutboxEntry> {
        let state = self.state.lock().expect("lock poisoned");
        state.outbox.iter().filter(|e| !e.published).cloned().collect()
    }

    pub fn mark_published(&self, ids: &[u64]) {
        let mut state = self.state.lock().expect("lock poisoned");
        for entry in state.outbox.iter_mut() {
            if ids.contains(&entry.id) {
                entry.published = true;
            }
        }
    }

    pub fn balance(&self) -> i64 {
        self.state.lock().expect("lock poisoned").balance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_service_double_charges_on_retry() {
        let mut svc = NaivePaymentService::new(1000);

        // First attempt: succeeds
        let r1 = svc.charge(100, "order-123");
        assert_eq!(r1, PaymentResult::Success { balance: 900 });

        // "Network timeout" -- client retries the same logical operation
        let r2 = svc.charge(100, "order-123");
        assert_eq!(r2, PaymentResult::Success { balance: 800 });

        // Balance was deducted twice!
        assert_eq!(svc.balance(), 800);
        assert_eq!(svc.operation_count(), 2);
    }

    #[test]
    fn idempotent_service_handles_retry() {
        let mut svc = IdempotentPaymentService::new(1000);

        // First attempt
        let r1 = svc.charge("idem-key-001", 100, "order-123");
        assert_eq!(r1, PaymentResult::Success { balance: 900 });

        // Retry with same idempotency key
        let r2 = svc.charge("idem-key-001", 100, "order-123");
        assert!(matches!(r2, PaymentResult::DuplicateRequest { .. }));

        // Balance deducted only once
        assert_eq!(svc.balance(), 900);
        assert_eq!(svc.operation_count(), 1);
    }

    #[test]
    fn different_idempotency_keys_are_independent() {
        let mut svc = IdempotentPaymentService::new(1000);

        svc.charge("key-1", 100, "order-1");
        svc.charge("key-2", 200, "order-2");
        svc.charge("key-3", 300, "order-3");

        assert_eq!(svc.balance(), 400);
        assert_eq!(svc.operation_count(), 3);
    }

    #[test]
    fn idempotent_service_caches_failure_too() {
        let mut svc = IdempotentPaymentService::new(50);

        // First attempt: insufficient funds
        let r1 = svc.charge("key-fail", 100, "big-order");
        assert!(matches!(r1, PaymentResult::InsufficientFunds { .. }));

        // Retry: should return cached failure, not re-check balance
        let r2 = svc.charge("key-fail", 100, "big-order");
        assert!(matches!(r2, PaymentResult::DuplicateRequest { .. }));
        if let PaymentResult::DuplicateRequest { original_result } = r2 {
            assert!(matches!(
                *original_result,
                PaymentResult::InsufficientFunds { .. }
            ));
        }
    }

    #[test]
    fn toctou_window_allows_double_processing() {
        // Simulate two "concurrent" requests with the same idempotency key
        // passing through the TOCTOU gap
        let svc = TocTouVulnerableService::new(1000);

        let svc1 = svc.clone();
        let svc2 = svc.clone();

        // Simulate: both threads pass the idempotency check before either stores result
        // We can't perfectly simulate the race in a single thread, but we can show
        // the vulnerability by calling charge_with_toctou_gap sequentially and noting
        // that the TOCTOU gap exists in the code structure.

        // In a real concurrent scenario, thread 1 checks (no key) -> thread 2 checks (no key)
        // -> thread 1 processes -> thread 2 processes = double charge

        // Sequential test: first call works, second is caught by stored result
        let r1 = svc1.charge_with_toctou_gap("key-race", 100);
        let r2 = svc2.charge_with_toctou_gap("key-race", 100);

        // Sequentially, the second call sees the stored result
        assert!(matches!(r1, PaymentResult::Success { .. }));
        assert!(matches!(r2, PaymentResult::DuplicateRequest { .. }));

        // But with true concurrency (threads hitting the gap), both could succeed
        // The code structure proves the vulnerability exists
        eprintln!("TOCTOU gap: between check and store, concurrent requests can slip through");
    }

    #[test]
    fn toctou_concurrent_double_charge() {
        use std::sync::Barrier;
        use std::thread;

        let svc = TocTouVulnerableService::new(1000);
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();

        for _ in 0..2 {
            let svc = svc.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                svc.charge_with_toctou_gap("concurrent-key", 100)
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let success_count = results
            .iter()
            .filter(|r| matches!(r, PaymentResult::Success { .. }))
            .count();

        eprintln!("Concurrent TOCTOU results: {results:?}");
        eprintln!("Success count: {success_count} (should be 1 for correct behavior)");

        // With TOCTOU, it's possible (but not guaranteed) that both succeed
        // This test documents the vulnerability rather than asserting it always happens
        // (thread scheduling is non-deterministic)
    }

    #[test]
    fn atomic_service_prevents_concurrent_double_charge() {
        use std::sync::Barrier;
        use std::thread;

        let svc = AtomicIdempotentService::new(1000);
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = Vec::new();

        // 10 threads all try to charge with the same key
        for _ in 0..10 {
            let svc = svc.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                svc.charge("same-key", 100)
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        let success_count = results
            .iter()
            .filter(|r| matches!(r, PaymentResult::Success { .. }))
            .count();
        let duplicate_count = results
            .iter()
            .filter(|r| matches!(r, PaymentResult::DuplicateRequest { .. }))
            .count();

        eprintln!("Atomic: {success_count} success, {duplicate_count} duplicate");

        assert_eq!(success_count, 1, "exactly one should succeed");
        assert_eq!(duplicate_count, 9, "rest should be duplicates");
        assert_eq!(
            svc.balance(),
            900,
            "balance should reflect exactly one charge"
        );
        assert_eq!(svc.operation_count(), 1);
    }

    #[test]
    fn many_retries_same_result() {
        let mut svc = IdempotentPaymentService::new(1000);

        let first = svc.charge("key-x", 250, "order-x");
        assert_eq!(first, PaymentResult::Success { balance: 750 });

        for _ in 0..100 {
            let retry = svc.charge("key-x", 250, "order-x");
            assert!(matches!(retry, PaymentResult::DuplicateRequest { .. }));
        }

        assert_eq!(svc.balance(), 750);
        assert_eq!(svc.operation_count(), 1);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn idempotent_retry_never_changes_balance(
            initial_balance in 1000i64..10000,
            amount in 1i64..500,
            num_retries in 1usize..50,
        ) {
            let mut svc = IdempotentPaymentService::new(initial_balance);

            let first = svc.charge("test-key", amount, "test");
            let expected_balance = match first {
                PaymentResult::Success { balance } => balance,
                PaymentResult::InsufficientFunds { balance } => balance,
                _ => panic!("first call should not be duplicate"),
            };

            for _ in 0..num_retries {
                svc.charge("test-key", amount, "test");
            }

            prop_assert_eq!(svc.balance(), expected_balance);
            prop_assert!(svc.operation_count() <= 1);
        }

        #[test]
        fn unique_keys_process_independently(
            num_operations in 1usize..20,
            amount in 1i64..100,
        ) {
            let initial = (num_operations as i64) * amount + 1000;
            let mut svc = IdempotentPaymentService::new(initial);

            for i in 0..num_operations {
                let key = format!("key-{i}");
                let result = svc.charge(&key, amount, "op");
                let is_success = matches!(result, PaymentResult::Success { .. });
                prop_assert!(is_success, "expected Success, got {:?}", result);
            }

            let expected = initial - (num_operations as i64) * amount;
            prop_assert_eq!(svc.balance(), expected);
            prop_assert_eq!(svc.operation_count(), num_operations);
        }
    }

    #[test]
    fn outbox_atomic_with_state_change() {
        let svc = OutboxService::new(1000);
        let result = svc.charge_and_emit("order-1", 100, "payment.captured");
        match result {
            OutboxResult::Success { balance, outbox_id } => {
                assert_eq!(balance, 900);
                assert_eq!(outbox_id, 1);
            }
            other => panic!("expected Success, got {other:?}"),
        }
        let pending = svc.drain_unpublished();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].payload, "payment.captured");
    }

    #[test]
    fn outbox_idempotent_retry_does_not_double_emit() {
        let svc = OutboxService::new(1000);
        svc.charge_and_emit("order-1", 100, "payment.captured");
        let r2 = svc.charge_and_emit("order-1", 100, "payment.captured");
        assert!(matches!(r2, OutboxResult::Duplicate { .. }));
        assert_eq!(svc.drain_unpublished().len(), 1);
    }

    #[test]
    fn outbox_publish_marks_published() {
        let svc = OutboxService::new(1000);
        svc.charge_and_emit("a", 100, "evt-a");
        svc.charge_and_emit("b", 200, "evt-b");
        let pending = svc.drain_unpublished();
        assert_eq!(pending.len(), 2);
        svc.mark_published(&[pending[0].id]);
        let remaining = svc.drain_unpublished();
        assert_eq!(remaining.len(), 1);
    }
}
