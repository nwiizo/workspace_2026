//! Runtime event collection for RustProbe.
//!
//! Phase 1: types and infrastructure only (not yet linked to the driver).
//! Phase 2 will inject calls to this crate via MIR instrumentation.

pub mod collector;
pub mod event;
pub mod ringbuf;

pub use collector::Collector;
pub use event::{EventKind, ProbeEvent};
pub use ringbuf::RingBuffer;
