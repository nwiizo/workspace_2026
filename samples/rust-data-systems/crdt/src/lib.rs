use std::collections::{BTreeMap, BTreeSet};

/// G-Counter: grow-only counter (state-based CRDT).
/// Each node maintains its own counter; the value is the sum of all counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GCounter {
    counters: BTreeMap<String, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str) {
        *self.counters.entry(node_id.to_string()).or_default() += 1;
    }

    pub fn increment_by(&mut self, node_id: &str, amount: u64) {
        *self.counters.entry(node_id.to_string()).or_default() += amount;
    }

    pub fn value(&self) -> u64 {
        self.counters.values().sum()
    }

    /// Merge another G-Counter into this one (pointwise maximum).
    pub fn merge(&mut self, other: &GCounter) {
        for (node, &count) in &other.counters {
            let entry = self.counters.entry(node.clone()).or_default();
            *entry = (*entry).max(count);
        }
    }
}

impl Default for GCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// PN-Counter: positive-negative counter.
/// Uses two G-Counters: one for increments, one for decrements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PNCounter {
    positive: GCounter,
    negative: GCounter,
}

impl PNCounter {
    pub fn new() -> Self {
        Self {
            positive: GCounter::new(),
            negative: GCounter::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str) {
        self.positive.increment(node_id);
    }

    pub fn decrement(&mut self, node_id: &str) {
        self.negative.increment(node_id);
    }

    pub fn value(&self) -> i64 {
        self.positive.value() as i64 - self.negative.value() as i64
    }

    pub fn merge(&mut self, other: &PNCounter) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

impl Default for PNCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique operation identifier for an OR-Set add.
///
/// A dot is explicit about the two pieces that make a tag unique:
/// the replica that created it and that replica's monotonic counter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dot {
    pub replica_id: String,
    pub counter: u64,
}

/// OR-Set (Observed-Remove Set): add-wins semantics.
/// Each element is tagged with a unique dot. Remove only removes observed dots.
#[derive(Debug, Clone)]
pub struct ORSet {
    elements: BTreeMap<String, BTreeSet<Dot>>, // element -> set of unique dots
    tombstones: BTreeSet<Dot>,
    counters: BTreeMap<String, u64>,
}

impl ORSet {
    pub fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
            tombstones: BTreeSet::new(),
            counters: BTreeMap::new(),
        }
    }

    /// Add an element, returning the assigned dot.
    pub fn add(&mut self, element: &str, replica_id: &str) -> Dot {
        let dot = self.generate_dot(replica_id);
        self.elements
            .entry(element.to_string())
            .or_default()
            .insert(dot.clone());
        dot
    }

    /// Remove an element (only removes currently observed tags).
    pub fn remove(&mut self, element: &str) {
        if let Some(observed) = self.elements.remove(element) {
            self.tombstones.extend(observed);
        }
    }

    pub fn contains(&self, element: &str) -> bool {
        self.elements
            .get(element)
            .is_some_and(|tags| !tags.is_empty())
    }

    pub fn elements(&self) -> Vec<&str> {
        self.elements
            .iter()
            .filter(|(_, tags)| !tags.is_empty())
            .map(|(e, _)| e.as_str())
            .collect()
    }

    /// Merge another OR-Set. Add-wins: if an element has tags in either set, it's present.
    pub fn merge(&mut self, other: &ORSet) {
        self.tombstones.extend(other.tombstones.iter().cloned());

        for (element, other_tags) in &other.elements {
            let entry = self.elements.entry(element.clone()).or_default();
            for dot in other_tags {
                if !self.tombstones.contains(dot) {
                    entry.insert(dot.clone());
                }
            }
        }
        self.prune_removed();

        for (replica_id, &counter) in &other.counters {
            let entry = self.counters.entry(replica_id.clone()).or_default();
            *entry = (*entry).max(counter);
        }
    }

    fn prune_removed(&mut self) {
        for tags in self.elements.values_mut() {
            tags.retain(|dot| !self.tombstones.contains(dot));
        }
        self.elements.retain(|_, tags| !tags.is_empty());
    }

    fn generate_dot(&mut self, replica_id: &str) -> Dot {
        let counter = self.counters.entry(replica_id.to_string()).or_default();
        *counter += 1;
        Dot {
            replica_id: replica_id.to_string(),
            counter: *counter,
        }
    }
}

impl Default for ORSet {
    fn default() -> Self {
        Self::new()
    }
}

/// LWW-Register: Last-Writer-Wins Register.
/// Each write carries a timestamp and replica ID.
/// The register uses replica ID as a deterministic tie-breaker when timestamps match.
#[derive(Debug, Clone)]
pub struct LWWRegister {
    value: Option<String>,
    timestamp: u64,
    replica_id: String,
}

impl LWWRegister {
    pub fn new() -> Self {
        Self {
            value: None,
            timestamp: 0,
            replica_id: String::new(),
        }
    }

    pub fn set(&mut self, value: &str, timestamp: u64, replica_id: &str) {
        if (timestamp, replica_id) > (self.timestamp, self.replica_id.as_str()) {
            self.value = Some(value.to_string());
            self.timestamp = timestamp;
            self.replica_id = replica_id.to_string();
        }
    }

    pub fn get(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn replica_id(&self) -> &str {
        &self.replica_id
    }

    pub fn merge(&mut self, other: &LWWRegister) {
        if (other.timestamp, other.replica_id.as_str()) > (self.timestamp, self.replica_id.as_str())
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.replica_id = other.replica_id.clone();
        }
    }
}

impl Default for LWWRegister {
    fn default() -> Self {
        Self::new()
    }
}

/// Delta-state G-Counter (Almeida et al., JPDC 2018).
///
/// State-basedの安全性（半束のmerge）はそのままに、毎回の同期で
/// 全状態ではなく「前回以降に変化したノードのスロットだけ」を delta として送る。
/// delta も同型の `GCounter` であり、受信側は通常の merge で取り込む。
/// `Loro`, `riak_dt` などの実用CRDTライブラリはこの設計を採る。
#[derive(Debug, Clone)]
pub struct DeltaGCounter {
    state: GCounter,
    /// 前回 take_delta() 以降にローカルで変更されたスロットだけを溜める buffer
    pending_delta: GCounter,
}

impl DeltaGCounter {
    pub fn new() -> Self {
        Self {
            state: GCounter::new(),
            pending_delta: GCounter::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str) {
        self.increment_by(node_id, 1);
    }

    pub fn increment_by(&mut self, node_id: &str, amount: u64) {
        self.state.increment_by(node_id, amount);
        // delta側にも反映: 自スロットの「現在値」を delta に複製する
        let current = self.state.counters.get(node_id).copied().unwrap_or(0);
        self.pending_delta.counters.insert(node_id.to_string(), current);
    }

    pub fn value(&self) -> u64 {
        self.state.value()
    }

    /// 蓄積された delta を取り出してbufferをリセット。
    /// 戻り値だけを他レプリカへ送ればよい。
    pub fn take_delta(&mut self) -> GCounter {
        std::mem::take(&mut self.pending_delta)
    }

    /// 他レプリカから受け取った delta（または全状態）をマージ。
    /// 受信deltaは自分の送信bufferにも伝播させる（anti-entropy的な forwarding）。
    pub fn merge_delta(&mut self, delta: &GCounter) {
        self.state.merge(delta);
        self.pending_delta.merge(delta);
    }

    pub fn state(&self) -> &GCounter {
        &self.state
    }
}

impl Default for DeltaGCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- G-Counter tests ---

    #[test]
    fn gcounter_basic() {
        let mut c = GCounter::new();
        c.increment("node-A");
        c.increment("node-A");
        c.increment("node-B");
        assert_eq!(c.value(), 3);
    }

    #[test]
    fn gcounter_merge_idempotent() {
        let mut c1 = GCounter::new();
        c1.increment("node-A");
        c1.increment("node-A");

        let mut c2 = GCounter::new();
        c2.increment("node-B");

        let mut merged = c1.clone();
        merged.merge(&c2);

        let mut merged_again = merged.clone();
        merged_again.merge(&c2);

        assert_eq!(merged.value(), merged_again.value());
        assert_eq!(merged, merged_again);
    }

    #[test]
    fn gcounter_merge_commutative() {
        let mut c1 = GCounter::new();
        c1.increment_by("A", 3);

        let mut c2 = GCounter::new();
        c2.increment_by("B", 5);

        let mut m1 = c1.clone();
        m1.merge(&c2);

        let mut m2 = c2.clone();
        m2.merge(&c1);

        assert_eq!(m1, m2);
    }

    #[test]
    fn gcounter_merge_associative() {
        let mut c1 = GCounter::new();
        c1.increment_by("A", 2);

        let mut c2 = GCounter::new();
        c2.increment_by("B", 3);

        let mut c3 = GCounter::new();
        c3.increment_by("C", 4);

        // (c1 merge c2) merge c3
        let mut left = c1.clone();
        left.merge(&c2);
        left.merge(&c3);

        // c1 merge (c2 merge c3)
        let mut right_inner = c2.clone();
        right_inner.merge(&c3);
        let mut right = c1.clone();
        right.merge(&right_inner);

        assert_eq!(left, right);
    }

    // --- PN-Counter tests ---

    #[test]
    fn pncounter_basic() {
        let mut c = PNCounter::new();
        c.increment("A");
        c.increment("A");
        c.decrement("B");
        assert_eq!(c.value(), 1);
    }

    #[test]
    fn pncounter_merge() {
        let mut c1 = PNCounter::new();
        c1.increment("A");
        c1.increment("A");

        let mut c2 = PNCounter::new();
        c2.decrement("B");

        c1.merge(&c2);
        assert_eq!(c1.value(), 1);
    }

    #[test]
    fn pncounter_negative() {
        let mut c = PNCounter::new();
        c.decrement("A");
        c.decrement("A");
        c.increment("B");
        assert_eq!(c.value(), -1);
    }

    // --- OR-Set tests ---

    #[test]
    fn orset_add_remove() {
        let mut s = ORSet::new();
        s.add("apple", "A");
        s.add("banana", "A");
        assert!(s.contains("apple"));
        assert!(s.contains("banana"));

        s.remove("apple");
        assert!(!s.contains("apple"));
        assert!(s.contains("banana"));
    }

    #[test]
    fn orset_concurrent_add_remove() {
        // Simulate concurrent add and remove (add-wins semantics)
        let mut replica1 = ORSet::new();
        let mut replica2 = ORSet::new();

        // Both replicas start with "apple"
        replica1.add("apple", "A");
        replica2.merge(&replica1);

        // Replica 1 removes "apple"
        replica1.remove("apple");

        // Replica 2 concurrently adds "apple" again (new tag)
        replica2.add("apple", "B");

        // Merge: add-wins, so "apple" should be present (replica2's add wins)
        replica1.merge(&replica2);
        assert!(
            replica1.contains("apple"),
            "concurrent add should win over remove"
        );
    }

    #[test]
    fn orset_observed_remove_survives_old_snapshot_merge() {
        let mut replica1 = ORSet::new();
        let mut replica2 = ORSet::new();

        replica1.add("apple", "A");
        replica2.merge(&replica1);

        replica1.remove("apple");
        replica2.merge(&replica1);

        assert!(!replica2.contains("apple"));

        replica1.merge(&replica2);
        assert!(!replica1.contains("apple"));
    }

    #[test]
    fn orset_merge_idempotent() {
        let mut s1 = ORSet::new();
        s1.add("x", "A");
        s1.add("y", "B");

        let snapshot = s1.clone();
        s1.merge(&snapshot);

        assert_eq!(s1.elements().len(), 2);
    }

    // --- LWW-Register tests ---

    #[test]
    fn lww_register_latest_wins() {
        let mut r = LWWRegister::new();
        r.set("old", 1, "A");
        r.set("new", 2, "A");
        assert_eq!(r.get(), Some("new"));
    }

    #[test]
    fn lww_register_ignores_older() {
        let mut r = LWWRegister::new();
        r.set("new", 5, "A");
        r.set("old", 3, "A");
        assert_eq!(r.get(), Some("new"));
    }

    #[test]
    fn lww_register_tie_breaks_by_replica_id() {
        let mut r = LWWRegister::new();
        r.set("from-a", 5, "A");
        r.set("from-b", 5, "B");
        assert_eq!(r.get(), Some("from-b"));
        assert_eq!(r.replica_id(), "B");
    }

    #[test]
    fn lww_register_merge() {
        let mut r1 = LWWRegister::new();
        r1.set("from-1", 10, "A");

        let mut r2 = LWWRegister::new();
        r2.set("from-2", 20, "B");

        r1.merge(&r2);
        assert_eq!(r1.get(), Some("from-2"));
    }

    // --- 3-node simulation ---

    #[test]
    fn three_node_gcounter_convergence() {
        let mut node_a = GCounter::new();
        let mut node_b = GCounter::new();
        let mut node_c = GCounter::new();

        // Each node increments locally
        node_a.increment_by("A", 10);
        node_b.increment_by("B", 20);
        node_c.increment_by("C", 30);

        // Pairwise merges (simulating gossip)
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);

        // One more round to fully converge
        node_a.merge(&node_c);
        node_b.merge(&node_a);

        assert_eq!(node_a.value(), 60);
        assert_eq!(node_b.value(), 60);
        assert_eq!(node_c.value(), 60);
        assert_eq!(node_a, node_b);
        assert_eq!(node_b, node_c);
    }

    // --- Delta G-Counter tests ---

    #[test]
    fn delta_gcounter_basic_merge() {
        let mut a = DeltaGCounter::new();
        let mut b = DeltaGCounter::new();

        a.increment_by("A", 5);
        a.increment_by("A", 3);
        let delta_a = a.take_delta();
        // A側で8回incrementしたぶんだけがdeltaに乗る
        assert_eq!(delta_a.value(), 8);

        b.merge_delta(&delta_a);
        assert_eq!(b.value(), 8);
    }

    #[test]
    fn delta_gcounter_subsequent_deltas_are_minimal() {
        // 同期後に少しだけインクリメントした場合、deltaも小さい
        let mut a = DeltaGCounter::new();
        let mut b = DeltaGCounter::new();

        a.increment_by("A", 100);
        b.merge_delta(&a.take_delta());

        // 追加で1回だけincrement
        a.increment("A");
        let small_delta = a.take_delta();
        // delta内のスロット数はAだけ（1つ）
        assert_eq!(small_delta.counters.len(), 1);
        assert_eq!(small_delta.value(), 101);

        b.merge_delta(&small_delta);
        assert_eq!(b.value(), 101);
    }

    #[test]
    fn delta_gcounter_three_node_gossip() {
        let mut a = DeltaGCounter::new();
        let mut b = DeltaGCounter::new();
        let mut c = DeltaGCounter::new();

        a.increment_by("A", 7);
        b.increment_by("B", 11);
        c.increment_by("C", 13);

        // gossip: A→B→C→A→B
        b.merge_delta(&a.take_delta());
        c.merge_delta(&b.take_delta());
        a.merge_delta(&c.take_delta());
        b.merge_delta(&a.take_delta());

        assert_eq!(a.value(), 7 + 11 + 13);
        assert_eq!(b.value(), 7 + 11 + 13);
        assert_eq!(c.value(), 7 + 11 + 13);
    }

    #[test]
    fn delta_gcounter_take_delta_resets_buffer() {
        let mut a = DeltaGCounter::new();
        a.increment("A");
        a.take_delta();
        let next = a.take_delta();
        // 2回目の take は空 delta
        assert_eq!(next.value(), 0);
        assert!(next.counters.is_empty());
    }

    #[test]
    fn three_node_pncounter_convergence() {
        let mut node_a = PNCounter::new();
        let mut node_b = PNCounter::new();
        let mut node_c = PNCounter::new();

        node_a.increment("A");
        node_a.increment("A");
        node_b.decrement("B");
        node_c.increment("C");
        node_c.increment("C");
        node_c.increment("C");

        // Gossip rounds
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        node_a.merge(&node_c);
        node_b.merge(&node_a);

        // Expected: +2 -1 +3 = 4
        assert_eq!(node_a.value(), 4);
        assert_eq!(node_b.value(), 4);
        assert_eq!(node_c.value(), 4);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn gcounter_merge_is_commutative(
            a_count in 0u64..100,
            b_count in 0u64..100,
        ) {
            let mut c1 = GCounter::new();
            c1.increment_by("A", a_count);

            let mut c2 = GCounter::new();
            c2.increment_by("B", b_count);

            let mut m1 = c1.clone();
            m1.merge(&c2);

            let mut m2 = c2.clone();
            m2.merge(&c1);

            prop_assert_eq!(m1, m2);
        }

        #[test]
        fn gcounter_merge_is_idempotent(
            a_count in 0u64..100,
            b_count in 0u64..100,
        ) {
            let mut c1 = GCounter::new();
            c1.increment_by("A", a_count);

            let mut c2 = GCounter::new();
            c2.increment_by("B", b_count);

            let mut merged = c1.clone();
            merged.merge(&c2);

            let mut merged_twice = merged.clone();
            merged_twice.merge(&c2);

            prop_assert_eq!(merged, merged_twice);
        }

        #[test]
        fn gcounter_merge_is_associative(
            a in 0u64..50,
            b in 0u64..50,
            c in 0u64..50,
        ) {
            let mut c1 = GCounter::new();
            c1.increment_by("A", a);

            let mut c2 = GCounter::new();
            c2.increment_by("B", b);

            let mut c3 = GCounter::new();
            c3.increment_by("C", c);

            let mut left = c1.clone();
            left.merge(&c2);
            left.merge(&c3);

            let mut right_inner = c2.clone();
            right_inner.merge(&c3);
            let mut right = c1.clone();
            right.merge(&right_inner);

            prop_assert_eq!(left, right);
        }

        #[test]
        fn pncounter_converges_with_random_ops(
            ops_a in prop::collection::vec(prop::bool::ANY, 1..20),
            ops_b in prop::collection::vec(prop::bool::ANY, 1..20),
        ) {
            let mut node_a = PNCounter::new();
            let mut node_b = PNCounter::new();

            for &is_inc in &ops_a {
                if is_inc {
                    node_a.increment("A");
                } else {
                    node_a.decrement("A");
                }
            }

            for &is_inc in &ops_b {
                if is_inc {
                    node_b.increment("B");
                } else {
                    node_b.decrement("B");
                }
            }

            let mut merged_ab = node_a.clone();
            merged_ab.merge(&node_b);

            let mut merged_ba = node_b.clone();
            merged_ba.merge(&node_a);

            // Commutativity
            prop_assert_eq!(merged_ab.value(), merged_ba.value());

            // Idempotency
            let mut merged_aba = merged_ab.clone();
            merged_aba.merge(&node_b);
            prop_assert_eq!(merged_ab.value(), merged_aba.value());
        }

        #[test]
        fn lww_register_merge_commutative(
            v1_ts in 1u64..1000,
            v2_ts in 1u64..1000,
        ) {
            let mut r1 = LWWRegister::new();
            r1.set("val-1", v1_ts, "A");

            let mut r2 = LWWRegister::new();
            r2.set("val-2", v2_ts, "B");

            let mut m1 = r1.clone();
            m1.merge(&r2);

            let mut m2 = r2.clone();
            m2.merge(&r1);

            prop_assert_eq!(m1.get(), m2.get());
        }
    }
}
