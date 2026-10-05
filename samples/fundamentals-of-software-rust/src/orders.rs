//! Chapters 4 and 5: model a workflow with explicit states and transitions.
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAction {
    Pay,
    Ship,
    Cancel,
}

#[derive(Debug, PartialEq, Eq, Error)]
#[error("cannot {action:?} an order in state {status:?}")]
pub struct TransitionError {
    pub status: OrderStatus,
    pub action: OrderAction,
}

#[derive(Debug)]
pub struct Order {
    status: OrderStatus,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            status: OrderStatus::Pending,
        }
    }
}

impl Order {
    pub fn status(&self) -> OrderStatus {
        self.status
    }

    /// Cancellation is allowed only before payment; refunds are outside this model.
    pub fn apply(&mut self, action: OrderAction) -> Result<(), TransitionError> {
        self.status = match (self.status, action) {
            (OrderStatus::Pending, OrderAction::Pay) => OrderStatus::Paid,
            (OrderStatus::Paid, OrderAction::Ship) => OrderStatus::Shipped,
            (OrderStatus::Pending, OrderAction::Cancel) => OrderStatus::Cancelled,
            (status, action) => return Err(TransitionError { status, action }),
        };
        Ok(())
    }
}
