use axum::extract::State;
use axum::http::StatusCode;

use crate::{AppState, Error};

pub fn internal_routes() -> axum::Router<AppState> {
    axum::Router::new().route(
        "/api/internal/matching",
        axum::routing::get(internal_get_matching),
    )
}

// このAPIをインスタンス内から一定間隔で叩かせることで、椅子とライドをマッチングさせる
async fn internal_get_matching(
    State(AppState {
        pool,
        notification_cache,
        ..
    }): State<AppState>,
) -> Result<StatusCode, Error> {
    const MATCHING_BATCH_SIZE: i64 = 64;

    #[derive(sqlx::FromRow)]
    struct PendingRide {
        id: String,
        user_id: String,
        pickup_latitude: i32,
        pickup_longitude: i32,
    }

    #[derive(sqlx::FromRow)]
    struct AvailableChair {
        id: String,
        latitude: i32,
        longitude: i32,
    }

    let mut tx = pool.begin().await?;

    let pending_rides: Vec<PendingRide> = sqlx::query_as(
        r#"
SELECT id, user_id, pickup_latitude, pickup_longitude
FROM rides
WHERE chair_id IS NULL
ORDER BY created_at
LIMIT ?
FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(MATCHING_BATCH_SIZE)
    .fetch_all(&mut *tx)
    .await?;

    if pending_rides.is_empty() {
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let mut available_chairs: Vec<AvailableChair> = sqlx::query_as(
        r#"
SELECT chairs.id,
       chair_current_locations.latitude,
       chair_current_locations.longitude
FROM chairs
INNER JOIN chair_current_locations
        ON chair_current_locations.chair_id = chairs.id
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND (
            SELECT COUNT(ride_statuses.chair_sent_at)
            FROM ride_statuses
            WHERE ride_statuses.ride_id = rides.id
        ) <> 6
  )
ORDER BY chairs.id
LIMIT ?
FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(MATCHING_BATCH_SIZE)
    .fetch_all(&mut *tx)
    .await?;

    let mut matched_notifications = Vec::with_capacity(pending_rides.len());
    for ride in pending_rides {
        let Some((chair_index, _)) =
            available_chairs
                .iter()
                .enumerate()
                .min_by_key(|(_, chair)| {
                    crate::calculate_distance(
                        ride.pickup_latitude,
                        ride.pickup_longitude,
                        chair.latitude,
                        chair.longitude,
                    )
                })
        else {
            break;
        };
        let chair = available_chairs.swap_remove(chair_index);

        let result = sqlx::query("UPDATE rides SET chair_id = ? WHERE id = ? AND chair_id IS NULL")
            .bind(&chair.id)
            .bind(&ride.id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 1 {
            matched_notifications.push((ride.user_id, chair.id));
        }
    }

    tx.commit().await?;
    for (user_id, chair_id) in matched_notifications {
        notification_cache.invalidate_app(&user_id);
        notification_cache.invalidate_chair(&chair_id);
    }

    Ok(StatusCode::NO_CONTENT)
}
