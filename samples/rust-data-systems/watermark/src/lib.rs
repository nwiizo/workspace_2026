use std::collections::BTreeMap;

/// An event with both event time and processing time.
#[derive(Debug, Clone)]
pub struct Event {
    pub event_time: u64,      // when the event actually occurred (ms)
    pub processing_time: u64, // when the system received it (ms)
    pub value: i64,
}

/// Result of a window aggregation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowResult {
    pub window_start: u64,
    pub window_end: u64,
    pub sum: i64,
    pub count: usize,
}

/// Tumbling window aggregator based on event time.
pub struct EventTimeWindowing {
    window_size: u64,
    /// Buffered events per window: window_start -> events
    windows: BTreeMap<u64, Vec<Event>>,
    /// Current watermark: "no more events before this time"
    watermark: u64,
    /// Completed (emitted) window results
    results: Vec<WindowResult>,
    /// Allowed lateness beyond watermark
    allowed_lateness: u64,
    /// Late events that were dropped
    dropped_late: Vec<Event>,
}

impl EventTimeWindowing {
    pub fn new(window_size: u64, allowed_lateness: u64) -> Self {
        Self {
            window_size,
            windows: BTreeMap::new(),
            watermark: 0,
            results: Vec::new(),
            allowed_lateness,
            dropped_late: Vec::new(),
        }
    }

    /// Determine which window an event belongs to.
    fn window_for(&self, event_time: u64) -> u64 {
        (event_time / self.window_size) * self.window_size
    }

    /// Process an event. Returns true if accepted, false if dropped as late.
    pub fn process_event(&mut self, event: Event) -> bool {
        let window_start = self.window_for(event.event_time);
        let window_end = window_start + self.window_size;

        // Check if the event is too late (window already closed beyond allowed lateness)
        if window_end + self.allowed_lateness < self.watermark {
            self.dropped_late.push(event);
            return false;
        }

        self.windows.entry(window_start).or_default().push(event);
        true
    }

    /// Advance the watermark. Triggers emission of any completed windows.
    pub fn advance_watermark(&mut self, new_watermark: u64) {
        if new_watermark <= self.watermark {
            return;
        }
        self.watermark = new_watermark;

        // Emit windows whose end time + allowed_lateness <= watermark
        let mut to_emit = Vec::new();
        for &window_start in self.windows.keys() {
            let window_end = window_start + self.window_size;
            if window_end + self.allowed_lateness <= self.watermark {
                to_emit.push(window_start);
            }
        }

        for window_start in to_emit {
            if let Some(events) = self.windows.remove(&window_start) {
                let sum: i64 = events.iter().map(|e| e.value).sum();
                let count = events.len();
                self.results.push(WindowResult {
                    window_start,
                    window_end: window_start + self.window_size,
                    sum,
                    count,
                });
            }
        }
    }

    /// Force-emit all remaining windows (flush on shutdown).
    pub fn flush(&mut self) {
        let windows: BTreeMap<u64, Vec<Event>> = std::mem::take(&mut self.windows);
        for (window_start, events) in windows {
            let sum: i64 = events.iter().map(|e| e.value).sum::<i64>();
            let count = events.len();
            self.results.push(WindowResult {
                window_start,
                window_end: window_start + self.window_size,
                sum,
                count,
            });
        }
    }

    pub fn results(&self) -> &[WindowResult] {
        &self.results
    }

    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    pub fn dropped_late_count(&self) -> usize {
        self.dropped_late.len()
    }
}

/// Tumbling window aggregator based on processing time.
/// Demonstrates how backlog replay breaks processing-time windows.
pub struct ProcessingTimeWindowing {
    window_size: u64,
    windows: BTreeMap<u64, Vec<Event>>,
    results: Vec<WindowResult>,
}

impl ProcessingTimeWindowing {
    pub fn new(window_size: u64) -> Self {
        Self {
            window_size,
            windows: BTreeMap::new(),
            results: Vec::new(),
        }
    }

    fn window_for(&self, processing_time: u64) -> u64 {
        (processing_time / self.window_size) * self.window_size
    }

    pub fn process_event(&mut self, event: Event) {
        let window_start = self.window_for(event.processing_time);
        self.windows.entry(window_start).or_default().push(event);
    }

    /// Close windows before the given processing time.
    pub fn close_windows_before(&mut self, processing_time: u64) {
        let mut to_emit = Vec::new();
        for &window_start in self.windows.keys() {
            if window_start + self.window_size <= processing_time {
                to_emit.push(window_start);
            }
        }

        for window_start in to_emit {
            if let Some(events) = self.windows.remove(&window_start) {
                let sum: i64 = events.iter().map(|e| e.value).sum();
                let count = events.len();
                self.results.push(WindowResult {
                    window_start,
                    window_end: window_start + self.window_size,
                    sum,
                    count,
                });
            }
        }
    }

    pub fn flush(&mut self) {
        let windows: BTreeMap<u64, Vec<Event>> = std::mem::take(&mut self.windows);
        for (window_start, events) in windows {
            let sum: i64 = events.iter().map(|e| e.value).sum::<i64>();
            let count = events.len();
            self.results.push(WindowResult {
                window_start,
                window_end: window_start + self.window_size,
                sum,
                count,
            });
        }
    }

    pub fn results(&self) -> &[WindowResult] {
        &self.results
    }
}

/// Multi-partition watermark tracker.
/// The global watermark is the minimum across all partition watermarks.
pub struct WatermarkTracker {
    partitions: BTreeMap<String, u64>,
}

impl WatermarkTracker {
    pub fn new(partition_ids: &[&str]) -> Self {
        let mut partitions = BTreeMap::new();
        for &id in partition_ids {
            partitions.insert(id.to_string(), 0);
        }
        Self { partitions }
    }

    /// Update a partition's watermark.
    pub fn update(&mut self, partition_id: &str, watermark: u64) {
        if let Some(wm) = self.partitions.get_mut(partition_id)
            && watermark > *wm
        {
            *wm = watermark;
        }
    }

    /// Global watermark = min of all partition watermarks.
    pub fn global_watermark(&self) -> u64 {
        self.partitions.values().copied().min().unwrap_or(0)
    }

    pub fn partition_watermarks(&self) -> &BTreeMap<String, u64> {
        &self.partitions
    }

    /// Watermark alignment (Flink FLIP-217 相当).
    ///
    /// 最も遅いパーティションのwatermarkから `max_drift_ms` を超えて先行している
    /// パーティションのIDを返す。呼び出し側はそれらのパーティションからの読み出しを
    /// 一時停止することで、状態（バッファ）の肥大化と遅延パーティションのwatermark
    /// holdbackを抑える。
    pub fn aligned_pause_set(&self, max_drift_ms: u64) -> Vec<String> {
        let Some(&min_wm) = self.partitions.values().min() else {
            return Vec::new();
        };
        self.partitions
            .iter()
            .filter(|&(_, &wm)| wm > min_wm + max_drift_ms)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(event_time: u64, processing_time: u64, value: i64) -> Event {
        Event {
            event_time,
            processing_time,
            value,
        }
    }

    #[test]
    fn basic_event_time_windowing() {
        let mut w = EventTimeWindowing::new(1000, 0);

        // Events in window [0, 1000)
        w.process_event(make_event(100, 100, 1));
        w.process_event(make_event(500, 200, 2));
        w.process_event(make_event(900, 300, 3));

        // Events in window [1000, 2000)
        w.process_event(make_event(1100, 400, 10));
        w.process_event(make_event(1500, 500, 20));

        // Advance watermark past first window
        w.advance_watermark(1000);
        assert_eq!(w.results().len(), 1);
        assert_eq!(w.results()[0].sum, 6);
        assert_eq!(w.results()[0].count, 3);
        assert_eq!(w.results()[0].window_start, 0);

        // Advance watermark past second window
        w.advance_watermark(2000);
        assert_eq!(w.results().len(), 2);
        assert_eq!(w.results()[1].sum, 30);
        assert_eq!(w.results()[1].count, 2);
    }

    #[test]
    fn out_of_order_events_handled_correctly() {
        let mut w = EventTimeWindowing::new(1000, 0);

        // Events arrive out of order but all within the same window
        w.process_event(make_event(900, 100, 3));
        w.process_event(make_event(100, 200, 1));
        w.process_event(make_event(500, 300, 2));

        w.advance_watermark(1000);
        assert_eq!(w.results().len(), 1);
        assert_eq!(w.results()[0].sum, 6);
        assert_eq!(w.results()[0].count, 3);
    }

    #[test]
    fn late_events_within_allowed_lateness() {
        let mut w = EventTimeWindowing::new(1000, 500);

        // Window [0, 1000) events
        w.process_event(make_event(100, 100, 1));

        // Advance watermark to 1200
        w.advance_watermark(1200);

        // Late event for window [0, 1000) -- within allowed lateness (1000 + 500 > 1200)
        let accepted = w.process_event(make_event(200, 1300, 2));
        assert!(accepted, "event should be accepted within allowed lateness");

        // Advance past allowed lateness
        w.advance_watermark(1500);
        assert_eq!(w.results().len(), 1);
        assert_eq!(w.results()[0].sum, 3);
        assert_eq!(w.results()[0].count, 2);
    }

    #[test]
    fn late_events_beyond_allowed_lateness_dropped() {
        let mut w = EventTimeWindowing::new(1000, 200);

        w.process_event(make_event(100, 100, 1));
        w.advance_watermark(1300); // window [0,1000) closes at watermark >= 1000+200=1200

        // This event is too late: window [0,1000) already closed
        let accepted = w.process_event(make_event(200, 1400, 2));
        assert!(!accepted, "event should be dropped as too late");
        assert_eq!(w.dropped_late_count(), 1);
    }

    #[test]
    fn processing_time_window_correct_under_normal_flow() {
        let mut w = ProcessingTimeWindowing::new(1000);

        // Normal flow: processing time ~ event time
        for t in (0..3000).step_by(100) {
            w.process_event(make_event(t, t + 10, 1));
        }

        w.close_windows_before(3000);
        // Windows [0,1000), [1000,2000), [2000,3000) all close (start+1000 <= 3000)
        assert_eq!(w.results().len(), 3);
        assert_eq!(w.results()[0].count, 10); // processing_time 10..910
        assert_eq!(w.results()[1].count, 10); // processing_time 1010..1910
        assert_eq!(w.results()[2].count, 10); // processing_time 2010..2910
    }

    #[test]
    fn backlog_replay_breaks_processing_time_windows() {
        // Scenario: consumer restarts and replays a backlog
        // Event times span [0, 5000), but all arrive at processing time ~10000

        let mut et_windowing = EventTimeWindowing::new(1000, 0);
        let mut pt_windowing = ProcessingTimeWindowing::new(1000);

        // Original events: event_time 0..5000, processing_time matches
        // After restart: all replayed at processing_time = 10000..10050
        let replay_start = 10000;
        for i in 0..50 {
            let event_time = i * 100; // spread across 5 event-time windows
            let processing_time = replay_start + i; // all crammed into 1 processing-time window

            let event = make_event(event_time, processing_time, 1);
            et_windowing.process_event(event.clone());
            pt_windowing.process_event(event);
        }

        // Event-time windowing: advance watermark past all windows
        et_windowing.advance_watermark(6000);

        // Processing-time windowing: close windows
        pt_windowing.close_windows_before(11000);

        // Event-time: events correctly distributed across 5 windows
        assert_eq!(
            et_windowing.results().len(),
            5,
            "should have 5 event-time windows"
        );
        for result in et_windowing.results() {
            assert_eq!(result.count, 10, "each window should have 10 events");
        }

        // Processing-time: all events crammed into 1 window!
        assert_eq!(
            pt_windowing.results().len(),
            1,
            "all events collapsed into 1 processing-time window"
        );
        assert_eq!(
            pt_windowing.results()[0].count,
            50,
            "all 50 events in one window"
        );

        eprintln!(
            "Event-time windows: {} (10 events each)",
            et_windowing.results().len()
        );
        eprintln!(
            "Processing-time windows: {} ({} events crammed together)",
            pt_windowing.results().len(),
            pt_windowing.results()[0].count
        );
    }

    #[test]
    fn multi_partition_watermark() {
        let mut tracker = WatermarkTracker::new(&["p0", "p1", "p2"]);

        // All start at 0
        assert_eq!(tracker.global_watermark(), 0);

        // p0 advances fast, p1 slower, p2 stuck
        tracker.update("p0", 1000);
        tracker.update("p1", 500);
        tracker.update("p2", 100);

        // Global watermark is the minimum
        assert_eq!(tracker.global_watermark(), 100);

        // p2 catches up
        tracker.update("p2", 800);
        assert_eq!(tracker.global_watermark(), 500);

        // All advance
        tracker.update("p0", 2000);
        tracker.update("p1", 1500);
        tracker.update("p2", 1500);
        assert_eq!(tracker.global_watermark(), 1500);
    }

    #[test]
    fn watermark_lag_vs_completeness_tradeoff() {
        // Demonstrate: higher allowed_lateness = more complete but higher latency
        let events: Vec<Event> = vec![
            make_event(100, 100, 1),
            make_event(500, 200, 2),
            make_event(900, 300, 3),
            make_event(200, 1100, 4), // late event (event_time 200, arrives after window "should" close)
        ];

        // Scenario 1: no allowed lateness
        let mut w_strict = EventTimeWindowing::new(1000, 0);
        for e in &events {
            w_strict.process_event(e.clone());
        }
        w_strict.advance_watermark(1100);

        // Scenario 2: 200ms allowed lateness
        let mut w_lenient = EventTimeWindowing::new(1000, 200);
        for e in &events {
            w_lenient.process_event(e.clone());
        }
        w_lenient.advance_watermark(1100);

        // Strict: window closes at watermark 1000, late event at t=200 still got in
        // because watermark was 0 when it arrived (it arrived at processing_time 1100
        // but we advanced watermark after all events)
        assert_eq!(w_strict.results().len(), 1);
        let strict_sum = w_strict.results()[0].sum;

        // Lenient: same events, but window waits longer before closing
        // At watermark 1100, window [0,1000) needs watermark >= 1000+200=1200 to close
        assert_eq!(
            w_lenient.results().len(),
            0,
            "lenient window should not close yet"
        );

        w_lenient.advance_watermark(1200);
        assert_eq!(w_lenient.results().len(), 1);
        let lenient_sum = w_lenient.results()[0].sum;

        assert_eq!(
            strict_sum, lenient_sum,
            "both should include the late event"
        );
        eprintln!("Strict closes at watermark=1000, lenient at watermark=1200");
        eprintln!("Trade-off: 200ms more latency for same completeness in this case");
    }

    #[test]
    fn random_order_produces_correct_results() {
        // 100 events with event times 0..10000, shuffled arrival order
        let mut events: Vec<Event> = (0..100)
            .map(|i| {
                let event_time = i * 100;
                // Simulate random delays: processing_time = event_time + some jitter
                let jitter = (i * 37 + 13) % 200;
                make_event(event_time, event_time + jitter, 1)
            })
            .collect();

        // Shuffle by reversing chunks (deterministic "shuffle")
        events.chunks_mut(10).for_each(|chunk| chunk.reverse());

        let mut w = EventTimeWindowing::new(1000, 500);
        for e in &events {
            w.process_event(e.clone());
        }
        w.advance_watermark(15000); // well past all events

        // Should have 10 windows, each with 10 events
        let total_count: usize = w.results().iter().map(|r| r.count).sum();
        let total_sum: i64 = w.results().iter().map(|r| r.sum).sum();

        assert_eq!(total_count, 100, "all events should be counted");
        assert_eq!(total_sum, 100, "all events have value=1, sum should be 100");
        assert_eq!(w.dropped_late_count(), 0, "no events should be dropped");
    }

    #[test]
    fn flush_emits_incomplete_windows() {
        let mut w = EventTimeWindowing::new(1000, 0);

        w.process_event(make_event(100, 100, 1));
        w.process_event(make_event(1500, 200, 2));

        // Don't advance watermark -- simulate shutdown
        assert_eq!(w.results().len(), 0);

        w.flush();
        assert_eq!(
            w.results().len(),
            2,
            "flush should emit all pending windows"
        );
    }

    #[test]
    fn watermark_alignment_pauses_fast_partitions() {
        let mut tr = WatermarkTracker::new(&["p0", "p1", "p2"]);
        tr.update("p0", 10_000);
        tr.update("p1", 5_000);
        tr.update("p2", 1_000);

        // 差 9000ms。max_drift=3000なら、min(1000)+3000=4000 を超えるp0,p1を停止対象に
        let mut pause = tr.aligned_pause_set(3_000);
        pause.sort();
        assert_eq!(pause, vec!["p0".to_string(), "p1".to_string()]);
    }

    #[test]
    fn watermark_alignment_no_pause_when_aligned() {
        let mut tr = WatermarkTracker::new(&["p0", "p1", "p2"]);
        tr.update("p0", 3_000);
        tr.update("p1", 2_500);
        tr.update("p2", 2_000);

        // 差 1000ms < 3000ms drift → 停止不要
        let pause = tr.aligned_pause_set(3_000);
        assert!(pause.is_empty());
    }
}
