use leptos::prelude::*;

/// Toggle like on a post
#[server]
pub async fn toggle_like(post_id: String) -> Result<bool, ServerFnError> {
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

    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM likes WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if existing.is_some() {
        sqlx::query("DELETE FROM likes WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        // Create unlike notification (skip)
        Ok(false)
    } else {
        sqlx::query("INSERT INTO likes (user_id, post_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(post_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        // Create notification for post author
        create_notification(&pool, user_id, post_id, "like").await?;

        Ok(true)
    }
}

/// Toggle rechirp on a post
#[server]
pub async fn toggle_rechirp(post_id: String) -> Result<bool, ServerFnError> {
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

    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM rechirps WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if existing.is_some() {
        sqlx::query("DELETE FROM rechirps WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
        Ok(false)
    } else {
        sqlx::query("INSERT INTO rechirps (user_id, post_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(post_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        create_notification(&pool, user_id, post_id, "rechirp").await?;
        Ok(true)
    }
}

/// Follow a user
#[server]
pub async fn follow_user(target_username: String) -> Result<bool, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let target: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(&target_username)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let (target_id,) = target.ok_or_else(|| ServerFnError::new("User not found"))?;

    if target_id == user_id {
        return Err(ServerFnError::new("Cannot follow yourself"));
    }

    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT follower_id FROM follows WHERE follower_id = $1 AND following_id = $2",
    )
    .bind(user_id)
    .bind(target_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if existing.is_some() {
        // Unfollow
        sqlx::query("DELETE FROM follows WHERE follower_id = $1 AND following_id = $2")
            .bind(user_id)
            .bind(target_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
        Ok(false)
    } else {
        // Follow
        sqlx::query("INSERT INTO follows (follower_id, following_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(target_id)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        // Create follow notification
        let notif_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO notifications (id, user_id, actor_id, event_type) VALUES ($1, $2, $3, 'follow'::notification_event)",
        )
        .bind(notif_id)
        .bind(target_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        Ok(true)
    }
}

/// Get user profile data
#[server]
pub async fn get_user_profile(
    username: String,
) -> Result<crate::models::user::UserProfile, ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let current_user_id = super::posts::get_current_user_id().await;

    #[derive(sqlx::FromRow)]
    struct ProfileRow {
        id: Uuid,
        username: String,
        display_name: String,
        bio: Option<String>,
        avatar_url: Option<String>,
        header_url: Option<String>,
        followers_count: i32,
        following_count: i32,
        posts_count: i32,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let row: Option<ProfileRow> = sqlx::query_as(
        "SELECT id, username, display_name, bio, avatar_url, header_url, \
         followers_count, following_count, posts_count, created_at \
         FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let row = row.ok_or_else(|| ServerFnError::new("User not found"))?;

    let (is_following, is_followed_by) = if let Some(uid) = current_user_id {
        let following: Option<(Uuid,)> = sqlx::query_as(
            "SELECT follower_id FROM follows WHERE follower_id = $1 AND following_id = $2",
        )
        .bind(uid)
        .bind(row.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        let followed_by: Option<(Uuid,)> = sqlx::query_as(
            "SELECT follower_id FROM follows WHERE follower_id = $1 AND following_id = $2",
        )
        .bind(row.id)
        .bind(uid)
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        (following.is_some(), followed_by.is_some())
    } else {
        (false, false)
    };

    Ok(crate::models::user::UserProfile {
        id: row.id,
        username: row.username,
        display_name: row.display_name,
        bio: row.bio,
        avatar_url: row.avatar_url,
        header_url: row.header_url,
        followers_count: row.followers_count,
        following_count: row.following_count,
        posts_count: row.posts_count,
        created_at: row.created_at,
        is_following,
        is_followed_by,
    })
}

#[cfg(feature = "ssr")]
async fn create_notification(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    post_id: uuid::Uuid,
    event_type: &str,
) -> Result<(), ServerFnError> {
    use uuid::Uuid;

    // Get post author
    let post_author: Option<(Uuid,)> = sqlx::query_as("SELECT user_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if let Some((author_id,)) = post_author {
        if author_id != actor_id {
            let notif_id = Uuid::now_v7();
            sqlx::query(
                "INSERT INTO notifications (id, user_id, actor_id, event_type, post_id) \
                 VALUES ($1, $2, $3, $4::notification_event, $5)",
            )
            .bind(notif_id)
            .bind(author_id)
            .bind(actor_id)
            .bind(event_type)
            .bind(post_id)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
        }
    }

    Ok(())
}
