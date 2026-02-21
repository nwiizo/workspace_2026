use leptos::prelude::*;

use crate::models::notification::Notification;

/// Get notifications for the current user
#[server]
pub async fn get_notifications(
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Notification>, ServerFnError> {
    use crate::models::notification::NotificationEvent;
    use crate::models::user::UserSummary;
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

    #[derive(sqlx::FromRow)]
    struct NotifRow {
        id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
        actor_username: String,
        actor_display_name: String,
        actor_avatar_url: Option<String>,
        event_type: String,
        post_id: Option<Uuid>,
        post_content: Option<String>,
        is_read: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<NotifRow> = sqlx::query_as(
        r#"
        SELECT n.id, n.user_id, n.actor_id,
               u.username as actor_username,
               u.display_name as actor_display_name,
               u.avatar_url as actor_avatar_url,
               n.event_type::text as event_type,
               n.post_id,
               p.content as post_content,
               n.is_read,
               n.created_at
        FROM notifications n
        JOIN users u ON n.actor_id = u.id
        LEFT JOIN posts p ON n.post_id = p.id
        WHERE n.user_id = $1
          AND ($2::UUID IS NULL OR n.id < $2)
        ORDER BY n.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let notifications = rows
        .into_iter()
        .filter_map(|row| {
            let event_type: NotificationEvent = row.event_type.parse().ok()?;
            Some(Notification {
                id: row.id,
                user_id: row.user_id,
                actor: UserSummary {
                    id: row.actor_id,
                    username: row.actor_username,
                    display_name: row.actor_display_name,
                    avatar_url: row.actor_avatar_url,
                },
                event_type,
                post_id: row.post_id,
                post_content: row.post_content,
                is_read: row.is_read,
                created_at: row.created_at,
            })
        })
        .collect();

    Ok(notifications)
}

/// Mark all notifications as read
#[server]
pub async fn mark_notifications_read() -> Result<(), ServerFnError> {
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    sqlx::query("UPDATE notifications SET is_read = TRUE WHERE user_id = $1 AND is_read = FALSE")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(())
}
