#![forbid(unsafe_code)]

//! Small experiments separating memory safety from application security.
//!
//! A reference cannot outlive the request-owned value it borrows:
//!
//! ```compile_fail,E0597
//! let response;
//! {
//!     let request = String::from("request-owned data");
//!     response = &request;
//! }
//! println!("{response}");
//! ```
//!
//! Moving ownership instead keeps the data alive:
//!
//! ```
//! let response = {
//!     let request = String::from("request-owned data");
//!     request
//! };
//! assert_eq!(response, "request-owned data");
//! ```
//!
//! An uninitialized value cannot be read through ordinary safe code:
//!
//! ```compile_fail,E0381
//! let response: String;
//! println!("{response}");
//! ```

#[derive(Debug, PartialEq, Eq)]
pub struct Invoice {
    pub id: u64,
    pub tenant_id: u64,
    pub description: &'static str,
}

/// Intentionally missing authorization. Used to demonstrate the leak.
pub fn find_invoice_by_id(invoices: &[Invoice], invoice_id: u64) -> Option<&Invoice> {
    invoices.iter().find(|invoice| invoice.id == invoice_id)
}

/// Looks up an invoice for a tenant obtained from an authenticated identity.
///
/// Authentication itself is outside this in-memory example. Accepting the
/// tenant ID directly from an untrusted request would invalidate this policy.
pub fn find_invoice_for_tenant(
    invoices: &[Invoice],
    tenant_id: u64,
    invoice_id: u64,
) -> Option<&Invoice> {
    invoices
        .iter()
        .find(|invoice| invoice.id == invoice_id && invoice.tenant_id == tenant_id)
}
