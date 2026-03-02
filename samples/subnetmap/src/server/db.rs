use leptos::prelude::*;
use sqlx::PgPool;

pub fn pool() -> Result<PgPool, ServerFnError> {
    use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found in context"))
}
