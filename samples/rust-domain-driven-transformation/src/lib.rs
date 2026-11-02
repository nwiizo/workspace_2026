//! A single-show migration from a shared record to planning and sales models.

pub mod legacy;
pub mod migration;
pub mod planning;
pub mod sales;

use thiserror::Error;

/// The existing API's data shape; untrusted until validated by a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySnapshot {
    pub id: u32,
    pub title: String,
    pub capacity: u16,
    pub published: bool,
    pub reserved_seats: Vec<u16>,
}

/// The caller-facing operations kept during the migration.
pub trait ShowApi {
    fn publish(&mut self) -> Result<(), CinemaError>;
    fn reserve(&mut self, seat: u16) -> Result<(), CinemaError>;
    fn snapshot(&self) -> LegacySnapshot;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CinemaError {
    #[error("show ID must be positive")]
    InvalidShowId,
    #[error("title must not be blank")]
    EmptyTitle,
    #[error("capacity must be positive")]
    InvalidCapacity,
    #[error("show is already published")]
    AlreadyPublished,
    #[error("show is not published")]
    NotPublished,
    #[error("seat {0} is outside this show's seating plan")]
    InvalidSeat(u16),
    #[error("seat {0} is already reserved")]
    SeatTaken(u16),
    #[error("an unpublished show cannot have reservations")]
    ReservationsBeforePublication,
    #[error("publication differs from the existing seating plan")]
    ConflictingPublication,
}
