//! The compatibility boundary used while callers still speak the old API.

use crate::{
    CinemaError, LegacySnapshot, ShowApi,
    planning::ScheduledShow,
    sales::{SeatNumber, SeatingPlan},
};

pub struct MigratingShow {
    planning: ScheduledShow,
    sales: Option<SeatingPlan>,
}

impl MigratingShow {
    pub fn new(id: u32, title: String, capacity: u16) -> Result<Self, CinemaError> {
        Ok(Self {
            planning: ScheduledShow::new(id, title, capacity)?,
            sales: None,
        })
    }

    /// Builds a candidate without modifying the source. The caller switches only on success.
    pub fn from_snapshot(source: &LegacySnapshot) -> Result<Self, CinemaError> {
        let mut candidate = Self::new(source.id, source.title.clone(), source.capacity)?;
        if !source.published && !source.reserved_seats.is_empty() {
            return Err(CinemaError::ReservationsBeforePublication);
        }
        if source.published {
            candidate.publish()?;
            for &seat in &source.reserved_seats {
                candidate.reserve(seat)?;
            }
        }
        Ok(candidate)
    }
}

impl ShowApi for MigratingShow {
    fn publish(&mut self) -> Result<(), CinemaError> {
        let event = self.planning.publish()?;
        // In-process delivery is infallible here. Durable/asynchronous delivery
        // would need a transactional handoff, retries and reconciliation.
        self.sales = Some(SeatingPlan::open(event));
        Ok(())
    }

    fn reserve(&mut self, seat: u16) -> Result<(), CinemaError> {
        let sales = self.sales.as_mut().ok_or(CinemaError::NotPublished)?;
        sales.reserve(SeatNumber::try_from(seat)?)
    }

    fn snapshot(&self) -> LegacySnapshot {
        LegacySnapshot {
            id: self.planning.id().get(),
            title: self.planning.title().into(),
            capacity: self.planning.capacity(),
            published: self.planning.is_published(),
            reserved_seats: self
                .sales
                .as_ref()
                .map(SeatingPlan::reserved_seats)
                .unwrap_or_default(),
        }
    }
}
