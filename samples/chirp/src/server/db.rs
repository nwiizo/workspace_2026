use leptos::prelude::*;
use sqlx::PgPool;

/// Extract the PgPool from Leptos context (provided in main.rs)
pub fn pool() -> Result<PgPool, ServerFnError> {
    use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found in context"))
}
