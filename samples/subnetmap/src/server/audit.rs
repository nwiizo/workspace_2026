use leptos::prelude::*;
use uuid::Uuid;

use crate::models::audit::{AuditLog, Comment};

#[server]
pub async fn list_audit_logs(
    entity_type: Option<String>,
    entity_id: Option<Uuid>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AuditLog>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let limit = limit.unwrap_or(50);

    let rows = if let (Some(et), Some(eid)) = (&entity_type, entity_id) {
        sqlx::query_as!(
            AuditLog,
            r#"SELECT id, entity_type, entity_id, action,
               old_value, new_value, created_at
               FROM audit_logs
               WHERE entity_type = $1 AND entity_id = $2
               ORDER BY created_at DESC LIMIT $3"#,
            et,
            eid,
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    } else if let Some(ref cur) = cursor {
        let cursor_id =
            Uuid::parse_str(cur).map_err(|e| ServerFnError::new(format!("Invalid cursor: {e}")))?;
        sqlx::query_as!(
            AuditLog,
            r#"SELECT id, entity_type, entity_id, action,
               old_value, new_value, created_at
               FROM audit_logs
               WHERE id < $1
               ORDER BY created_at DESC LIMIT $2"#,
            cursor_id,
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    } else {
        sqlx::query_as!(
            AuditLog,
            r#"SELECT id, entity_type, entity_id, action,
               old_value, new_value, created_at
               FROM audit_logs
               ORDER BY created_at DESC LIMIT $1"#,
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?
    };

    Ok(rows)
}

#[server]
pub async fn list_comments(
    entity_type: String,
    entity_id: Uuid,
) -> Result<Vec<Comment>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = sqlx::query_as!(
        Comment,
        "SELECT id, entity_type, entity_id, content, created_at
         FROM comments
         WHERE entity_type = $1 AND entity_id = $2
         ORDER BY created_at DESC",
        entity_type,
        entity_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(rows)
}

#[server]
pub async fn create_comment(
    entity_type: String,
    entity_id: Uuid,
    content: String,
) -> Result<Comment, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    if content.trim().is_empty() {
        return Err(ServerFnError::new("Comment content cannot be empty"));
    }

    let id = Uuid::now_v7();
    let comment = sqlx::query_as!(
        Comment,
        "INSERT INTO comments (id, entity_type, entity_id, content)
         VALUES ($1, $2, $3, $4)
         RETURNING id, entity_type, entity_id, content, created_at",
        id,
        entity_type,
        entity_id,
        content
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(comment)
}
