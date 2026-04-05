use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::event::{EventKind, ProbeEvent};
use crate::ringbuf::RingBuffer;

static COLLECTOR: OnceLock<Collector> = OnceLock::new();

// Thread-local ring buffer for zero-contention event recording.
thread_local! {
    static THREAD_BUFFER: RingBuffer = RingBuffer::new();
}

pub struct Collector {
    start: Instant,
    output_dir: Mutex<Option<String>>,
}

impl Collector {
    pub fn init(output_dir: &str) {
        COLLECTOR.get_or_init(|| Collector {
            start: Instant::now(),
            output_dir: Mutex::new(Some(output_dir.to_owned())),
        });
    }

    pub fn global() -> Option<&'static Collector> {
        COLLECTOR.get()
    }

    pub fn record(&self, probe_id: u32, event_kind: EventKind, payload: u64) {
        let timestamp_ns = self.start.elapsed().as_nanos() as u64;
        let thread_id = thread_id::get() as u32;

        let event = ProbeEvent {
            timestamp_ns,
            probe_id,
            event_kind,
            thread_id,
            payload,
        };

        THREAD_BUFFER.with(|buf| {
            buf.push(event);
        });
    }

    pub fn flush_current_thread(&self) {
        let output_dir = {
            let guard = self.output_dir.lock().unwrap_or_else(|e| e.into_inner());
            guard.clone()
        };

        let Some(dir) = output_dir else {
            return;
        };

        THREAD_BUFFER.with(|buf| {
            let events = buf.drain();
            if events.is_empty() {
                return;
            }

            let thread_id = thread_id::get();
            let path =
                std::path::PathBuf::from(&dir).join(format!("events_thread_{thread_id}.bin"));

            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                Ok(mut file) => {
                    for event in &events {
                        // Write fields individually to avoid reading padding bytes (UB).
                        let _ = file.write_all(&event.timestamp_ns.to_le_bytes());
                        let _ = file.write_all(&event.probe_id.to_le_bytes());
                        let _ = file.write_all(&[event.event_kind as u8]);
                        let _ = file.write_all(&event.thread_id.to_le_bytes());
                        let _ = file.write_all(&event.payload.to_le_bytes());
                    }
                }
                Err(e) => {
                    eprintln!("rustprobe: failed to open {}: {e}", path.display());
                }
            }
        });
    }
}

mod thread_id {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    thread_local! {
        static THREAD_ID: u64 = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get() -> u64 {
        THREAD_ID.with(|id| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_and_record() {
        let dir = std::env::temp_dir()
            .join("rustprobe_test")
            .to_string_lossy()
            .to_string();
        Collector::init(&dir);

        let collector = Collector::global().expect("should be initialized");
        collector.record(42, EventKind::Clone, 128);

        THREAD_BUFFER.with(|buf| {
            assert_eq!(buf.len(), 1);
            let event = buf.pop().expect("should have event");
            assert_eq!(event.probe_id, 42);
            assert_eq!(event.event_kind, EventKind::Clone);
            assert_eq!(event.payload, 128);
        });
    }

    #[test]
    fn thread_ids_are_unique() {
        let id1 = thread_id::get();
        let id2 = std::thread::spawn(|| thread_id::get())
            .join()
            .expect("thread should not panic");
        assert_ne!(id1, id2);
    }
}
