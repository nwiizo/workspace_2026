//! Baseline behavior. Deliberately keeps planning and sales in one record.

use crate::{CinemaError, LegacySnapshot, ShowApi};

pub struct LegacyShow {
    record: LegacySnapshot,
}

impl LegacyShow {
    pub fn new(id: u32, title: String, capacity: u16) -> Result<Self, CinemaError> {
        if id == 0 {
            return Err(CinemaError::InvalidShowId);
        }
        if title.trim().is_empty() {
            return Err(CinemaError::EmptyTitle);
        }
        if capacity == 0 {
            return Err(CinemaError::InvalidCapacity);
        }
        Ok(Self {
            record: LegacySnapshot {
                id,
                title,
                capacity,
                published: false,
                reserved_seats: Vec::new(),
            },
        })
    }
}

impl ShowApi for LegacyShow {
    fn publish(&mut self) -> Result<(), CinemaError> {
        if self.record.published {
            return Err(CinemaError::AlreadyPublished);
        }
        self.record.published = true;
        Ok(())
    }

    fn reserve(&mut self, seat: u16) -> Result<(), CinemaError> {
        if !self.record.published {
            return Err(CinemaError::NotPublished);
        }
        if seat == 0 || seat > self.record.capacity {
            return Err(CinemaError::InvalidSeat(seat));
        }
        if self.record.reserved_seats.contains(&seat) {
            return Err(CinemaError::SeatTaken(seat));
        }
        self.record.reserved_seats.push(seat);
        self.record.reserved_seats.sort_unstable();
        Ok(())
    }

    fn snapshot(&self) -> LegacySnapshot {
        self.record.clone()
    }
}
