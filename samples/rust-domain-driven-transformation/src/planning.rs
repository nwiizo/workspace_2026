//! Publication facts are owned by the planning context.

use std::num::{NonZeroU16, NonZeroU32};

use crate::CinemaError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowId(NonZeroU32);

impl TryFrom<u32> for ShowId {
    type Error = CinemaError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(CinemaError::InvalidShowId)
    }
}

impl ShowId {
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

/// Immutable publication facts; contains neither a movie title nor reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowPublished {
    show_id: ShowId,
    capacity: NonZeroU16,
}

impl ShowPublished {
    pub fn show_id(self) -> ShowId {
        self.show_id
    }

    pub fn capacity(self) -> u16 {
        self.capacity.get()
    }
}

pub struct ScheduledShow {
    id: ShowId,
    title: String,
    capacity: NonZeroU16,
    published: bool,
}

impl ScheduledShow {
    pub fn new(id: u32, title: String, capacity: u16) -> Result<Self, CinemaError> {
        let id = ShowId::try_from(id)?;
        if title.trim().is_empty() {
            return Err(CinemaError::EmptyTitle);
        }
        let capacity = NonZeroU16::new(capacity).ok_or(CinemaError::InvalidCapacity)?;
        Ok(Self {
            id,
            title,
            capacity,
            published: false,
        })
    }

    pub fn publish(&mut self) -> Result<ShowPublished, CinemaError> {
        if self.published {
            return Err(CinemaError::AlreadyPublished);
        }
        self.published = true;
        Ok(ShowPublished {
            show_id: self.id,
            capacity: self.capacity,
        })
    }

    pub fn id(&self) -> ShowId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn capacity(&self) -> u16 {
        self.capacity.get()
    }

    pub fn is_published(&self) -> bool {
        self.published
    }
}
