use leptos::prelude::*;
use uuid::Uuid;

use crate::models::tag::Tag;

#[server]
pub async fn list_tags() -> Result<Vec<Tag>, ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let rows = sqlx::query_as!(
        Tag,
        "SELECT id, name, color, created_at FROM tags ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(rows)
}

#[server]
pub async fn create_tag(name: String, color: String) -> Result<Tag, ServerFnError> {
    use super::db::pool;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("tag name cannot be empty"));
    }

    let color = color.trim().to_string();
    if !color.is_empty() && !color.starts_with('#') {
        return Err(ServerFnError::new(
            "color must be a hex color code (e.g. #ff0000)",
        ));
    }

    let pool = pool()?;
    let id = Uuid::now_v7();

    let tag = sqlx::query_as!(
        Tag,
        "INSERT INTO tags (id, name, color) VALUES ($1, $2, $3)
         RETURNING id, name, color, created_at",
        id,
        name,
        color
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(tag)
}

#[server]
pub async fn delete_tag(id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    let result = sqlx::query!("DELETE FROM tags WHERE id = $1", id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("Tag not found"));
    }
    Ok(())
}

#[server]
pub async fn add_tag_to_ip(ip_address_id: Uuid, tag_id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    sqlx::query!(
        "INSERT INTO ip_tags (ip_address_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        ip_address_id,
        tag_id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(())
}

#[server]
pub async fn remove_tag_from_ip(ip_address_id: Uuid, tag_id: Uuid) -> Result<(), ServerFnError> {
    use super::db::pool;

    let pool = pool()?;
    sqlx::query!(
        "DELETE FROM ip_tags WHERE ip_address_id = $1 AND tag_id = $2",
        ip_address_id,
        tag_id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("database error: {e}")))?;

    Ok(())
}
