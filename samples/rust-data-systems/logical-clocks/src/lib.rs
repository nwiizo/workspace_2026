use std::collections::BTreeMap;

/// Lamport Clock: simple scalar logical clock.
/// Guarantees total order but cannot distinguish causality from concurrency.
#[derive(Debug, Clone)]
pub struct LamportClock {
    pub time: u64,
    pub node_id: String,
}

impl LamportClock {
    pub fn new(node_id: &str) -> Self {
        Self {
            time: 0,
            node_id: node_id.to_string(),
        }
    }

    /// Local event: increment the clock.
    pub fn tick(&mut self) -> u64 {
        self.time += 1;
        self.time
    }

    /// Send event: increment and return timestamp for the message.
    pub fn send(&mut self) -> u64 {
        self.time += 1;
        self.time
    }

    /// Receive event: max(local, received) + 1.
    pub fn receive(&mut self, received_time: u64) -> u64 {
        self.time = self.time.max(received_time) + 1;
        self.time
    }
}

/// Ordering result for vector clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalOrder {
    Before,
    After,
    Concurrent,
    Equal,
}

/// Vector Clock: one counter per node, captures causality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    pub clocks: BTreeMap<String, u64>,
    pub node_id: String,
}

impl VectorClock {
    pub fn new(node_id: &str) -> Self {
        let mut clocks = BTreeMap::new();
        clocks.insert(node_id.to_string(), 0);
        Self {
            clocks,
            node_id: node_id.to_string(),
        }
    }

    pub fn tick(&mut self) -> &BTreeMap<String, u64> {
        *self.clocks.entry(self.node_id.clone()).or_default() += 1;
        &self.clocks
    }

    pub fn send(&mut self) -> BTreeMap<String, u64> {
        self.tick();
        self.clocks.clone()
    }

    pub fn receive(&mut self, received: &BTreeMap<String, u64>) {
        // Pointwise max
        for (node, &time) in received {
            let entry = self.clocks.entry(node.clone()).or_default();
            *entry = (*entry).max(time);
        }
        // Increment own counter
        *self.clocks.entry(self.node_id.clone()).or_default() += 1;
    }

    /// Compare two vector clocks.
    pub fn compare(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>) -> CausalOrder {
        let all_keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();

        let mut a_less = false;
        let mut b_less = false;

        for key in all_keys {
            let va = a.get(key).copied().unwrap_or(0);
            let vb = b.get(key).copied().unwrap_or(0);
            if va < vb {
                a_less = true;
            }
            if vb < va {
                b_less = true;
            }
        }

        match (a_less, b_less) {
            (false, false) => CausalOrder::Equal,
            (true, false) => CausalOrder::Before, // a happened-before b
            (false, true) => CausalOrder::After,  // a happened-after b
            (true, true) => CausalOrder::Concurrent,
        }
    }

    pub fn snapshot(&self) -> BTreeMap<String, u64> {
        self.clocks.clone()
    }
}

/// Hybrid Logical Clock (HLC).
/// Combines physical timestamp with logical counter.
/// Stays close to wall clock while preserving causality.
#[derive(Debug, Clone)]
pub struct HLC {
    pub physical: u64, // wall clock (milliseconds)
    pub logical: u32,  // tie-breaker counter
    pub node_id: String,
}

/// A timestamp from an HLC, comparable and orderable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HLCTimestamp {
    pub physical: u64,
    pub logical: u32,
}

impl HLC {
    pub fn new(node_id: &str, initial_time: u64) -> Self {
        Self {
            physical: initial_time,
            logical: 0,
            node_id: node_id.to_string(),
        }
    }

    /// Local or send event with current wall clock reading.
    pub fn now(&mut self, wall_clock: u64) -> HLCTimestamp {
        if wall_clock > self.physical {
            self.physical = wall_clock;
            self.logical = 0;
        } else {
            self.logical += 1;
        }
        HLCTimestamp {
            physical: self.physical,
            logical: self.logical,
        }
    }

    /// Receive event: merge with incoming timestamp and wall clock.
    pub fn receive(&mut self, wall_clock: u64, msg: HLCTimestamp) -> HLCTimestamp {
        if wall_clock > self.physical && wall_clock > msg.physical {
            self.physical = wall_clock;
            self.logical = 0;
        } else if msg.physical > self.physical {
            self.physical = msg.physical;
            self.logical = msg.logical + 1;
        } else if self.physical > msg.physical {
            self.logical += 1;
        } else {
            // msg.physical == self.physical
            self.logical = self.logical.max(msg.logical) + 1;
        }
        HLCTimestamp {
            physical: self.physical,
            logical: self.logical,
        }
    }

    pub fn timestamp(&self) -> HLCTimestamp {
        HLCTimestamp {
            physical: self.physical,
            logical: self.logical,
        }
    }

    /// Drift from wall clock in milliseconds.
    pub fn drift(&self, wall_clock: u64) -> i64 {
        self.physical as i64 - wall_clock as i64
    }

    /// CockroachDB の `--max-offset` に相当する境界チェック。
    /// 受信したタイムスタンプの物理部分が `max_offset_ms` 以上未来を指していたら、
    /// クロックスキューがSLAを超えたとして拒否する。安全側に倒すための仕組み。
    pub fn receive_bounded(
        &mut self,
        wall_clock: u64,
        msg: HLCTimestamp,
        max_offset_ms: u64,
    ) -> Result<HLCTimestamp, ClockSkewError> {
        if msg.physical > wall_clock && msg.physical - wall_clock > max_offset_ms {
            return Err(ClockSkewError {
                wall_clock,
                msg_physical: msg.physical,
                max_offset_ms,
            });
        }
        Ok(self.receive(wall_clock, msg))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClockSkewError {
    pub wall_clock: u64,
    pub msg_physical: u64,
    pub max_offset_ms: u64,
}

impl std::fmt::Display for ClockSkewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clock skew {}ms exceeds max_offset {}ms (msg_physical={}, wall_clock={})",
            self.msg_physical.saturating_sub(self.wall_clock),
            self.max_offset_ms,
            self.msg_physical,
            self.wall_clock,
        )
    }
}

impl std::error::Error for ClockSkewError {}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Lamport Clock tests ---

    #[test]
    fn lamport_local_events() {
        let mut c = LamportClock::new("A");
        assert_eq!(c.tick(), 1);
        assert_eq!(c.tick(), 2);
        assert_eq!(c.tick(), 3);
    }

    #[test]
    fn lamport_send_receive() {
        let mut a = LamportClock::new("A");
        let mut b = LamportClock::new("B");

        a.tick(); // A=1
        let msg_time = a.send(); // A=2
        b.receive(msg_time); // B=max(0,2)+1=3

        assert_eq!(a.time, 2);
        assert_eq!(b.time, 3);
    }

    #[test]
    fn lamport_respects_causal_order() {
        let mut a = LamportClock::new("A");
        let mut b = LamportClock::new("B");

        // A does some work
        a.tick(); // A=1
        a.tick(); // A=2
        let msg = a.send(); // A=3

        // B receives, so B's clock should be > A's send time
        let b_time = b.receive(msg);
        assert!(
            b_time > msg,
            "receiver should have higher time than message"
        );
    }

    #[test]
    fn lamport_total_order_with_tiebreak() {
        // Two concurrent events can have the same Lamport time
        let mut a = LamportClock::new("A");
        let mut b = LamportClock::new("B");

        a.tick(); // A=1
        b.tick(); // B=1

        // Same Lamport time! Need node_id for tiebreak.
        assert_eq!(a.time, b.time);
        // Total order: (time, node_id)
        let order_a = (a.time, &a.node_id);
        let order_b = (b.time, &b.node_id);
        assert_ne!(order_a, order_b);
    }

    // --- Vector Clock tests ---

    #[test]
    fn vector_clock_causality() {
        let mut a = VectorClock::new("A");
        let mut b = VectorClock::new("B");

        a.tick(); // A={A:1}
        let msg = a.send(); // A={A:2}
        b.receive(&msg); // B={A:2, B:1}

        let snap_a = a.snapshot();
        let snap_b = b.snapshot();

        // a's send happened-before b's state
        assert_eq!(VectorClock::compare(&snap_a, &snap_b), CausalOrder::Before);
    }

    #[test]
    fn vector_clock_concurrency() {
        let mut a = VectorClock::new("A");
        let mut b = VectorClock::new("B");

        a.tick(); // A={A:1}
        b.tick(); // B={B:1}

        let snap_a = a.snapshot();
        let snap_b = b.snapshot();

        // No message exchange -> concurrent
        assert_eq!(
            VectorClock::compare(&snap_a, &snap_b),
            CausalOrder::Concurrent
        );
    }

    #[test]
    fn vector_clock_three_nodes() {
        let mut a = VectorClock::new("A");
        let mut b = VectorClock::new("B");
        let mut c = VectorClock::new("C");

        // A sends to B
        let msg_ab = a.send();
        b.receive(&msg_ab);

        // B sends to C
        let msg_bc = b.send();
        c.receive(&msg_bc);

        // A's first event should be before C's state (transitive causality)
        let snap_a_initial = {
            let mut m = BTreeMap::new();
            m.insert("A".to_string(), 1);
            m
        };
        let snap_c = c.snapshot();

        assert_eq!(
            VectorClock::compare(&snap_a_initial, &snap_c),
            CausalOrder::Before
        );

        // A does a concurrent event (no communication)
        a.tick();
        let snap_a_now = a.snapshot();

        // A's latest state is concurrent with C's
        assert_eq!(
            VectorClock::compare(&snap_a_now, &snap_c),
            CausalOrder::Concurrent
        );
    }

    #[test]
    fn vector_clock_size_grows_with_nodes() {
        let mut clocks: Vec<VectorClock> = (0..10)
            .map(|i| VectorClock::new(&format!("node-{i}")))
            .collect();

        // Chain of messages: 0->1->2->...->9
        for i in 0..9 {
            let msg = clocks[i].send();
            clocks[i + 1].receive(&msg);
        }

        // Last node's vector clock has entries for all nodes
        let last_snap = clocks[9].snapshot();
        assert_eq!(last_snap.len(), 10, "vector clock should have 10 entries");
    }

    // --- HLC tests ---

    #[test]
    fn hlc_basic_progression() {
        let mut hlc = HLC::new("A", 0);

        let t1 = hlc.now(100); // wall clock advanced past initial
        assert_eq!(t1.physical, 100);
        assert_eq!(t1.logical, 0);

        let t2 = hlc.now(100); // same wall clock
        assert_eq!(t2.physical, 100);
        assert_eq!(t2.logical, 1);

        let t3 = hlc.now(101); // wall clock advanced
        assert_eq!(t3.physical, 101);
        assert_eq!(t3.logical, 0);

        assert!(t1 < t2);
        assert!(t2 < t3);
    }

    #[test]
    fn hlc_receive_from_future() {
        let mut a = HLC::new("A", 100);
        let mut b = HLC::new("B", 100);

        // B's clock is ahead (clock skew)
        let b_ts = b.now(200);

        // A receives from B, A's wall clock is still 100
        let a_ts = a.receive(100, b_ts);

        // A should adopt B's physical time
        assert_eq!(a_ts.physical, 200);
        assert!(
            a_ts > b_ts,
            "receiver timestamp should be > message timestamp"
        );
    }

    #[test]
    fn hlc_stays_close_to_wall_clock() {
        let mut hlc = HLC::new("A", 0);

        // First call advances physical to 1000
        hlc.now(1000);

        // 99 more events at the same wall clock
        for _ in 0..99 {
            hlc.now(1000);
        }

        // Physical should still be 1000, logical is 99
        assert_eq!(hlc.physical, 1000);
        assert_eq!(hlc.logical, 99);
        assert_eq!(hlc.drift(1000), 0);
    }

    #[test]
    fn hlc_three_node_skew_simulation() {
        // Node A: correct clock
        // Node B: clock is 50ms ahead
        // Node C: clock is 30ms behind
        let mut a = HLC::new("A", 1000);
        let mut b = HLC::new("B", 1050); // 50ms ahead
        let mut c = HLC::new("C", 970); // 30ms behind

        // Event on each node at their local time
        let a1 = a.now(1000);
        let b1 = b.now(1050);
        let c1 = c.now(970);

        // A sends to B (B's wall clock: 1051)
        let _a_send = a.now(1001);
        let b2 = b.receive(1051, a1);

        // B sends to C (C's wall clock: 971)
        let c2 = c.receive(971, b1);

        // Causally related events should be ordered correctly
        assert!(a1 < b2, "A's event should be before B's receive");
        assert!(b1 < c2, "B's event should be before C's receive");

        // C's initial event is concurrent with A and B (no communication)
        // But HLC timestamps still allow comparison
        eprintln!("A1: {:?}", a1);
        eprintln!("B1: {:?}", b1);
        eprintln!("C1: {:?}", c1);
        eprintln!("B2 (after recv A): {:?}", b2);
        eprintln!("C2 (after recv B): {:?}", c2);

        // Drift check
        eprintln!("A drift: {}ms", a.drift(1001));
        eprintln!("B drift: {}ms", b.drift(1051));
        eprintln!("C drift: {}ms", c.drift(971));
    }

    #[test]
    fn hlc_causal_events_always_ordered() {
        let mut a = HLC::new("A", 0);
        let mut b = HLC::new("B", 0);

        let mut prev = HLCTimestamp {
            physical: 0,
            logical: 0,
        };

        // Ping-pong 100 messages
        for i in 0..100 {
            let wall = i as u64;
            let ts_a = a.now(wall);
            assert!(ts_a > prev, "monotonicity broken at step {i} (a.now)");

            let msg_a = ts_a;
            let ts_b = b.receive(wall, msg_a);
            assert!(ts_b > ts_a, "causality broken at step {i} (b.receive)");

            let msg_b = ts_b;
            let ts_a2 = a.receive(wall, msg_b);
            assert!(ts_a2 > ts_b, "causality broken at step {i} (a.receive)");

            prev = ts_a2;
        }
    }

    #[test]
    fn hlc_bounded_offset_accepts_within_skew() {
        let mut a = HLC::new("A", 1000);
        let ts = HLCTimestamp { physical: 1500, logical: 0 };
        // wall_clock=1000, msg=1500, max_offset=600 → 差500 ≤ 600 で許容
        let result = a.receive_bounded(1000, ts, 600);
        assert!(result.is_ok());
    }

    #[test]
    fn hlc_bounded_offset_rejects_excess_skew() {
        let mut a = HLC::new("A", 1000);
        let ts = HLCTimestamp { physical: 5000, logical: 0 };
        // wall_clock=1000, msg=5000, 差4000ms → max_offset=500 を超える
        let result = a.receive_bounded(1000, ts, 500);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.max_offset_ms, 500);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn hlc_causal_order_preserved(
            wall_a in 0u64..10000,
            wall_b in 0u64..10000,
        ) {
            let mut a = HLC::new("A", wall_a);
            let mut b = HLC::new("B", wall_b);

            let ts_a = a.now(wall_a);
            let ts_b = b.receive(wall_b, ts_a);

            // b.receive must produce timestamp > ts_a (causality)
            prop_assert!(ts_b > ts_a, "causality violated: {:?} not > {:?}", ts_b, ts_a);
        }

        #[test]
        fn lamport_send_receive_preserves_order(
            a_ticks in 0u32..100,
            b_ticks in 0u32..100,
        ) {
            let mut a = LamportClock::new("A");
            let mut b = LamportClock::new("B");

            for _ in 0..a_ticks {
                a.tick();
            }
            for _ in 0..b_ticks {
                b.tick();
            }

            let send_time = a.send();
            let recv_time = b.receive(send_time);

            prop_assert!(recv_time > send_time, "receive time must exceed send time");
        }

        #[test]
        fn vector_clock_send_receive_is_before(
            a_ticks in 1u32..20,
            b_ticks in 0u32..20,
        ) {
            let mut a = VectorClock::new("A");
            let mut b = VectorClock::new("B");

            for _ in 0..a_ticks {
                a.tick();
            }
            for _ in 0..b_ticks {
                b.tick();
            }

            let snap_before_send = a.snapshot();
            let msg = a.send();
            b.receive(&msg);
            let snap_after_recv = b.snapshot();

            // sender's state at send time should be "before" receiver's state after receive
            prop_assert_eq!(
                VectorClock::compare(&snap_before_send, &snap_after_recv),
                CausalOrder::Before
            );
        }
    }
}
