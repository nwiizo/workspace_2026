use leptos::prelude::*;

use crate::models::post::PostWithMeta;
use crate::models::user::UserSummary;

/// Search for posts by content
#[server]
pub async fn search_posts(
    query: String,
    limit: Option<i64>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    let pool = super::db::pool()?;
    let current_user_id = super::posts::get_current_user_id().await;
    let limit = limit.unwrap_or(20).min(50);

    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let pattern = format!("%{}%", query.trim());

    let rows = sqlx::query_as::<_, super::posts::PostWithMetaRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.reply_to_id,
               p.reply_count, p.like_count, p.rechirp_count, p.created_at,
               u.id as author_id, u.username as author_username,
               u.display_name as author_display_name, u.avatar_url as author_avatar_url,
               COALESCE(EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $1), FALSE) as liked_by_me,
               COALESCE(EXISTS(SELECT 1 FROM rechirps WHERE post_id = p.id AND user_id = $1), FALSE) as rechirped_by_me
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.is_deleted = FALSE
          AND p.content ILIKE $2
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(current_user_id)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Search for users by username or display name
#[server]
pub async fn search_users(
    query: String,
    limit: Option<i64>,
) -> Result<Vec<UserSummary>, ServerFnError> {
    let pool = super::db::pool()?;
    let limit = limit.unwrap_or(20).min(50);

    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let pattern = format!("%{}%", query.trim());

    let users: Vec<UserSummary> = sqlx::query_as(
        r#"
        SELECT id, username, display_name, avatar_url
        FROM users
        WHERE username ILIKE $1 OR display_name ILIKE $1
        ORDER BY followers_count DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(users)
}
