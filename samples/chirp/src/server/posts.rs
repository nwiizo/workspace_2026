use leptos::prelude::*;

use crate::models::post::PostWithMeta;

/// Create a new post
#[server]
pub async fn create_post(
    content: String,
    reply_to_id: Option<String>,
) -> Result<PostWithMeta, ServerFnError> {
    use crate::models::user::UserSummary;
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    if content.trim().is_empty() || content.len() > 280 {
        return Err(ServerFnError::new(
            "Post must be between 1 and 280 characters",
        ));
    }

    let reply_to: Option<Uuid> = reply_to_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ServerFnError::new("Invalid reply_to_id"))?;

    let id = Uuid::now_v7();

    sqlx::query("INSERT INTO posts (id, user_id, content, reply_to_id) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(user_id)
        .bind(&content)
        .bind(reply_to)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let author: UserSummary =
        sqlx::query_as("SELECT id, username, display_name, avatar_url FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(PostWithMeta {
        id,
        user_id,
        content,
        reply_to_id: reply_to,
        reply_count: 0,
        like_count: 0,
        rechirp_count: 0,
        created_at: chrono::Utc::now(),
        author,
        liked_by_me: false,
        rechirped_by_me: false,
    })
}

/// Delete a post (soft delete)
#[server]
pub async fn delete_post(post_id: String) -> Result<(), ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let post_id: Uuid = post_id
        .parse()
        .map_err(|_| ServerFnError::new("Invalid post ID"))?;

    let result = sqlx::query(
        "UPDATE posts SET is_deleted = TRUE WHERE id = $1 AND user_id = $2 AND is_deleted = FALSE",
    )
    .bind(post_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("Post not found or already deleted"));
    }

    Ok(())
}

/// Get a single post with full detail (thread context)
#[server]
pub async fn get_post(post_id: String) -> Result<crate::models::post::PostDetail, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let current_user_id = get_current_user_id().await;

    let post_id: Uuid = post_id
        .parse()
        .map_err(|_| ServerFnError::new("Invalid post ID"))?;

    let post = fetch_post_with_meta(&pool, post_id, current_user_id)
        .await?
        .ok_or_else(|| ServerFnError::new("Post not found"))?;

    // Get parent if this is a reply
    let parent: Option<PostWithMeta> = if let Some(parent_id) = post.reply_to_id {
        fetch_post_with_meta(&pool, parent_id, current_user_id).await?
    } else {
        None
    };

    // Get replies
    let replies = fetch_replies(&pool, post_id, current_user_id).await?;

    Ok(crate::models::post::PostDetail {
        post,
        parent: parent.map(Box::new),
        replies,
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn get_current_user_id() -> Option<uuid::Uuid> {
    let session = super::auth::extract_session().await.ok()?;
    session.get("user_id").await.ok()?
}

#[cfg(feature = "ssr")]
async fn fetch_post_with_meta(
    pool: &sqlx::PgPool,
    post_id: uuid::Uuid,
    current_user_id: Option<uuid::Uuid>,
) -> Result<Option<PostWithMeta>, ServerFnError> {
    let row = sqlx::query_as::<_, PostWithMetaRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.reply_to_id,
               p.reply_count, p.like_count, p.rechirp_count, p.created_at,
               u.id as author_id, u.username as author_username,
               u.display_name as author_display_name, u.avatar_url as author_avatar_url,
               COALESCE(EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $2), FALSE) as liked_by_me,
               COALESCE(EXISTS(SELECT 1 FROM rechirps WHERE post_id = p.id AND user_id = $2), FALSE) as rechirped_by_me
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.id = $1 AND p.is_deleted = FALSE
        "#,
    )
    .bind(post_id)
    .bind(current_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(row.map(Into::into))
}

#[cfg(feature = "ssr")]
async fn fetch_replies(
    pool: &sqlx::PgPool,
    post_id: uuid::Uuid,
    current_user_id: Option<uuid::Uuid>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    let rows = sqlx::query_as::<_, PostWithMetaRow>(
        r#"
        SELECT p.id, p.user_id, p.content, p.reply_to_id,
               p.reply_count, p.like_count, p.rechirp_count, p.created_at,
               u.id as author_id, u.username as author_username,
               u.display_name as author_display_name, u.avatar_url as author_avatar_url,
               COALESCE(EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $2), FALSE) as liked_by_me,
               COALESCE(EXISTS(SELECT 1 FROM rechirps WHERE post_id = p.id AND user_id = $2), FALSE) as rechirped_by_me
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.reply_to_id = $1 AND p.is_deleted = FALSE
        ORDER BY p.created_at ASC
        LIMIT 50
        "#,
    )
    .bind(post_id)
    .bind(current_user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Row type for PostWithMeta queries
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
pub(crate) struct PostWithMetaRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    content: String,
    reply_to_id: Option<uuid::Uuid>,
    reply_count: i32,
    like_count: i32,
    rechirp_count: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    author_id: uuid::Uuid,
    author_username: String,
    author_display_name: String,
    author_avatar_url: Option<String>,
    liked_by_me: bool,
    rechirped_by_me: bool,
}

#[cfg(feature = "ssr")]
impl From<PostWithMetaRow> for PostWithMeta {
    fn from(row: PostWithMetaRow) -> Self {
        use crate::models::user::UserSummary;
        Self {
            id: row.id,
            user_id: row.user_id,
            content: row.content,
            reply_to_id: row.reply_to_id,
            reply_count: row.reply_count,
            like_count: row.like_count,
            rechirp_count: row.rechirp_count,
            created_at: row.created_at,
            author: UserSummary {
                id: row.author_id,
                username: row.author_username,
                display_name: row.author_display_name,
                avatar_url: row.author_avatar_url,
            },
            liked_by_me: row.liked_by_me,
            rechirped_by_me: row.rechirped_by_me,
        }
    }
}
