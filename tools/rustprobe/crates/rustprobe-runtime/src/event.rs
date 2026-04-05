use core::fmt;

/// A single probe event recorded during program execution.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ProbeEvent {
    pub timestamp_ns: u64,
    pub probe_id: u32,
    pub event_kind: EventKind,
    pub thread_id: u32,
    pub payload: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EventKind {
    FunctionEntry = 0,
    FunctionExit = 1,
    Move = 2,
    Clone = 3,
    Drop = 4,
    Alloc = 5,
    Dealloc = 6,
    BorrowStart = 7,
    BorrowEnd = 8,
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionEntry => write!(f, "fn_enter"),
            Self::FunctionExit => write!(f, "fn_exit"),
            Self::Move => write!(f, "move"),
            Self::Clone => write!(f, "clone"),
            Self::Drop => write!(f, "drop"),
            Self::Alloc => write!(f, "alloc"),
            Self::Dealloc => write!(f, "dealloc"),
            Self::BorrowStart => write!(f, "borrow_start"),
            Self::BorrowEnd => write!(f, "borrow_end"),
        }
    }
}

impl ProbeEvent {
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_size_is_stable() {
        assert!(ProbeEvent::SIZE <= 32, "Event size should be compact");
    }

    #[test]
    fn event_kind_display() {
        assert_eq!(EventKind::Clone.to_string(), "clone");
        assert_eq!(EventKind::Drop.to_string(), "drop");
        assert_eq!(EventKind::Move.to_string(), "move");
    }
}
