//! ses — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "SendEmail",
    "SendRawEmail",
    "VerifyEmailIdentity",
    "ListIdentities",
    "DeleteIdentity",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "ses", "email", ACTIONS);
}
