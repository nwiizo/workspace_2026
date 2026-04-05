use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::event::ProbeEvent;

const DEFAULT_CAPACITY: usize = 1 << 16;

/// A single-producer, single-consumer lock-free ring buffer for probe events.
pub struct RingBuffer {
    buffer: Box<[UnsafeCell<ProbeEvent>]>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    dropped: AtomicUsize,
}

// SAFETY: RingBuffer can be sent between threads. However, it is NOT Sync:
// SPSC semantics require a single producer and single consumer. Sharing &RingBuffer
// across threads would allow concurrent push() calls, violating the SPSC invariant.
unsafe impl Send for RingBuffer {}

impl RingBuffer {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// # Panics
    ///
    /// Panics if `capacity` is not a power of two or is zero.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(
            capacity > 0 && capacity.is_power_of_two(),
            "capacity must be a power of two"
        );

        let zeroed_event = ProbeEvent {
            timestamp_ns: 0,
            probe_id: 0,
            event_kind: crate::event::EventKind::FunctionEntry,
            thread_id: 0,
            payload: 0,
        };

        let mut storage = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            storage.push(UnsafeCell::new(zeroed_event));
        }

        Self {
            buffer: storage.into_boxed_slice(),
            capacity,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, event: ProbeEvent) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= self.capacity {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        let index = head & self.mask;
        unsafe {
            *self.buffer[index].get() = event;
        }

        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<ProbeEvent> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        let index = tail & self.mask;
        let event = unsafe { *self.buffer[index].get() };

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(event)
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn drain(&self) -> Vec<ProbeEvent> {
        let mut events = Vec::with_capacity(self.len());
        while let Some(event) = self.pop() {
            events.push(event);
        }
        events
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;

    fn make_event(probe_id: u32, kind: EventKind) -> ProbeEvent {
        ProbeEvent {
            timestamp_ns: 0,
            probe_id,
            event_kind: kind,
            thread_id: 0,
            payload: 0,
        }
    }

    #[test]
    fn push_and_pop() {
        let rb = RingBuffer::with_capacity(4);
        assert!(rb.is_empty());

        assert!(rb.push(make_event(1, EventKind::FunctionEntry)));
        assert!(rb.push(make_event(2, EventKind::Clone)));
        assert_eq!(rb.len(), 2);

        let e1 = rb.pop().expect("should have event");
        assert_eq!(e1.probe_id, 1);
        assert_eq!(e1.event_kind, EventKind::FunctionEntry);

        let e2 = rb.pop().expect("should have event");
        assert_eq!(e2.probe_id, 2);
        assert_eq!(e2.event_kind, EventKind::Clone);

        assert!(rb.pop().is_none());
        assert!(rb.is_empty());
    }

    #[test]
    fn full_buffer_drops_events() {
        let rb = RingBuffer::with_capacity(2);

        assert!(rb.push(make_event(1, EventKind::Move)));
        assert!(rb.push(make_event(2, EventKind::Drop)));
        assert!(!rb.push(make_event(3, EventKind::Alloc)));
        assert_eq!(rb.dropped_count(), 1);
    }

    #[test]
    fn wraparound() {
        let rb = RingBuffer::with_capacity(4);
        for round in 0..2 {
            for i in 0..4 {
                assert!(rb.push(make_event(round * 4 + i, EventKind::FunctionEntry)));
            }
            for i in 0..4 {
                let e = rb.pop().expect("should have event");
                assert_eq!(e.probe_id, round * 4 + i);
            }
            assert!(rb.is_empty());
        }
    }

    #[test]
    fn drain_all() {
        let rb = RingBuffer::with_capacity(8);
        for i in 0..5 {
            rb.push(make_event(i, EventKind::Clone));
        }
        let events = rb.drain();
        assert_eq!(events.len(), 5);
        assert!(rb.is_empty());
        for (i, e) in events.iter().enumerate() {
            assert_eq!(e.probe_id, i as u32);
        }
    }

    #[test]
    #[should_panic(expected = "capacity must be a power of two")]
    fn non_power_of_two_panics() {
        RingBuffer::with_capacity(3);
    }

    #[test]
    #[should_panic(expected = "capacity must be a power of two")]
    fn zero_capacity_panics() {
        RingBuffer::with_capacity(0);
    }
}
