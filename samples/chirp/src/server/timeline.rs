use leptos::prelude::*;

use crate::models::post::PostWithMeta;

/// Get the home timeline (posts from followed users + own posts)
#[server]
pub async fn get_home_timeline(
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let limit = limit.unwrap_or(20).min(50);
    let cursor_id: Option<Uuid> = cursor
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ServerFnError::new("Invalid cursor"))?;

    let rows = sqlx::query_as::<_, super::posts::PostWithMetaRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.reply_to_id,
               p.reply_count, p.like_count, p.rechirp_count, p.created_at,
               u.id as author_id, u.username as author_username,
               u.display_name as author_display_name, u.avatar_url as author_avatar_url,
               EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $1) as liked_by_me,
               EXISTS(SELECT 1 FROM rechirps WHERE post_id = p.id AND user_id = $1) as rechirped_by_me
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.is_deleted = FALSE
          AND p.reply_to_id IS NULL
          AND (p.user_id = $1 OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $1))
          AND ($2::UUID IS NULL OR p.id < $2)
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Get the public timeline (all posts, for explore/non-logged-in)
#[server]
pub async fn get_public_timeline(
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let current_user_id = super::posts::get_current_user_id().await;

    let limit = limit.unwrap_or(20).min(50);
    let cursor_id: Option<Uuid> = cursor
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ServerFnError::new("Invalid cursor"))?;

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
          AND p.reply_to_id IS NULL
          AND ($2::UUID IS NULL OR p.id < $2)
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(current_user_id)
    .bind(cursor_id)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Get a specific user's posts
#[server]
pub async fn get_user_timeline(
    username: String,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let current_user_id = super::posts::get_current_user_id().await;

    let limit = limit.unwrap_or(20).min(50);
    let cursor_id: Option<Uuid> = cursor
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ServerFnError::new("Invalid cursor"))?;

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
          AND u.username = $4
          AND ($2::UUID IS NULL OR p.id < $2)
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(current_user_id)
    .bind(cursor_id)
    .bind(limit)
    .bind(&username)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows.into_iter().map(Into::into).collect())
}
