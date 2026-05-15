//! iam — AWS Query protocol stub.

use super::stub;
use crate::registry::Registry;
use std::sync::Arc;

static ACTIONS: &[&str] = &[
    "CreateUser",
    "DeleteUser",
    "GetUser",
    "ListUsers",
    "CreateRole",
    "GetRole",
    "ListRoles",
    "AttachRolePolicy",
    "CreatePolicy",
    "GetPolicy",
    "ListPolicies",
    "CreateAccessKey",
    "ListAccessKeys",
];

pub fn register(registry: &Arc<Registry>) {
    stub::register_query(registry, "iam", "iam", ACTIONS);
}
