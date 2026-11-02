//! Seat allocation is owned by the sales context.

use std::{collections::BTreeSet, num::NonZeroU16};

use crate::{CinemaError, planning::ShowPublished};

/// A positive seat number, validated separately from a show's capacity.
/// Callers outside this module cannot bypass validation by constructing the field.
///
/// ```compile_fail
/// use std::num::NonZeroU16;
/// use rust_domain_driven_transformation::sales::SeatNumber;
/// let seat = SeatNumber(NonZeroU16::MIN);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeatNumber(NonZeroU16);

impl TryFrom<u16> for SeatNumber {
    type Error = CinemaError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        NonZeroU16::new(value)
            .map(Self)
            .ok_or(CinemaError::InvalidSeat(value))
    }
}

impl SeatNumber {
    pub fn get(self) -> u16 {
        self.0.get()
    }
}

pub struct SeatingPlan {
    publication: ShowPublished,
    reserved: BTreeSet<SeatNumber>,
}

impl SeatingPlan {
    pub fn open(publication: ShowPublished) -> Self {
        Self {
            publication,
            reserved: BTreeSet::new(),
        }
    }

    /// Replay is a no-op. A changed ID or capacity needs a separate business flow.
    pub fn apply_publication(&mut self, event: ShowPublished) -> Result<(), CinemaError> {
        if self.publication != event {
            return Err(CinemaError::ConflictingPublication);
        }
        Ok(())
    }

    /// `&mut self` serializes changes to this value, not to copies in other processes.
    pub fn reserve(&mut self, seat: SeatNumber) -> Result<(), CinemaError> {
        if seat.get() > self.publication.capacity() {
            return Err(CinemaError::InvalidSeat(seat.get()));
        }
        if !self.reserved.insert(seat) {
            return Err(CinemaError::SeatTaken(seat.get()));
        }
        Ok(())
    }

    pub fn reserved_seats(&self) -> Vec<u16> {
        self.reserved.iter().map(|seat| seat.get()).collect()
    }
}
