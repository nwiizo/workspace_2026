//! sealed trait と closed enum で API の拡張点を明示的に制御する。

use std::marker::PhantomData;

mod sealed {
    pub trait Sealed {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentMethod {
    Card,
    BankTransfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authorized;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Captured;

impl sealed::Sealed for Authorized {}
impl sealed::Sealed for Captured {}

pub trait PaymentState: sealed::Sealed {
    fn label() -> &'static str;
}

impl PaymentState for Authorized {
    fn label() -> &'static str {
        "authorized"
    }
}

impl PaymentState for Captured {
    fn label() -> &'static str {
        "captured"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment<S: PaymentState> {
    id: String,
    method: PaymentMethod,
    _state: PhantomData<S>,
}

impl Payment<Authorized> {
    pub fn authorize(id: impl Into<String>, method: PaymentMethod) -> Self {
        Self {
            id: id.into(),
            method,
            _state: PhantomData,
        }
    }

    pub fn capture(self) -> Payment<Captured> {
        Payment {
            id: self.id,
            method: self.method,
            _state: PhantomData,
        }
    }
}

impl<S: PaymentState> Payment<S> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn method(&self) -> PaymentMethod {
        self.method
    }

    pub fn state_label(&self) -> &'static str {
        S::label()
    }
}

pub fn audit_line<S: PaymentState>(payment: &Payment<S>) -> String {
    let method = match payment.method() {
        PaymentMethod::Card => "card",
        PaymentMethod::BankTransfer => "bank-transfer",
    };
    format!(
        "payment_id={} method={} state={}",
        payment.id(),
        method,
        payment.state_label()
    )
}
