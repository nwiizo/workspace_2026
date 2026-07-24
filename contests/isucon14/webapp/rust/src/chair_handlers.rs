use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use ulid::Ulid;

use crate::models::{Chair, Owner, Ride, RideStatus, User};
use crate::{AppState, Coordinate, Error};

pub fn chair_routes(app_state: AppState) -> axum::Router<AppState> {
    let routes =
        axum::Router::new().route("/api/chair/chairs", axum::routing::post(chair_post_chairs));

    let authed_routes = axum::Router::new()
        .route(
            "/api/chair/activity",
            axum::routing::post(chair_post_activity),
        )
        .route(
            "/api/chair/coordinate",
            axum::routing::post(chair_post_coordinate),
        )
        .route(
            "/api/chair/notification",
            axum::routing::get(chair_get_notification),
        )
        .route(
            "/api/chair/rides/:ride_id/status",
            axum::routing::post(chair_post_ride_status),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::middlewares::chair_auth_middleware,
        ));

    routes.merge(authed_routes)
}

#[derive(Debug, serde::Deserialize)]
struct ChairPostChairsRequest {
    name: String,
    model: String,
    chair_register_token: String,
}

#[derive(Debug, serde::Serialize)]
struct ChairPostChairsResponse {
    id: String,
    owner_id: String,
}

async fn chair_post_chairs(
    State(AppState { pool, .. }): State<AppState>,
    jar: CookieJar,
    axum::Json(req): axum::Json<ChairPostChairsRequest>,
) -> Result<(CookieJar, (StatusCode, axum::Json<ChairPostChairsResponse>)), Error> {
    let Some(owner): Option<Owner> =
        sqlx::query_as("SELECT * FROM owners WHERE chair_register_token = ?")
            .bind(req.chair_register_token)
            .fetch_optional(&pool)
            .await?
    else {
        return Err(Error::Unauthorized("invalid chair_register_token"));
    };

    let chair_id = Ulid::new().to_string();
    let access_token = crate::secure_random_str(32);

    sqlx::query("INSERT INTO chairs (id, owner_id, name, model, is_active, access_token) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&chair_id)
        .bind(&owner.id)
        .bind(req.name)
        .bind(req.model)
        .bind(false)
        .bind(&access_token)
        .execute(&pool)
        .await?;

    let jar = jar.add(Cookie::build(("chair_session", access_token)).path("/"));

    Ok((
        jar,
        (
            StatusCode::CREATED,
            axum::Json(ChairPostChairsResponse {
                id: chair_id,
                owner_id: owner.id,
            }),
        ),
    ))
}

#[derive(Debug, serde::Deserialize)]
struct PostChairActivityRequest {
    is_active: bool,
}

async fn chair_post_activity(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    axum::Json(req): axum::Json<PostChairActivityRequest>,
) -> Result<StatusCode, Error> {
    sqlx::query("UPDATE chairs SET is_active = ? WHERE id = ?")
        .bind(req.is_active)
        .bind(chair.id)
        .execute(&pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize)]
struct ChairPostCoordinateResponse {
    recorded_at: i64,
}

async fn chair_post_coordinate(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    axum::Json(req): axum::Json<Coordinate>,
) -> Result<axum::Json<ChairPostCoordinateResponse>, Error> {
    #[derive(sqlx::FromRow)]
    struct CurrentRide {
        id: String,
        evaluation: Option<i32>,
        pickup_latitude: i32,
        pickup_longitude: i32,
        destination_latitude: i32,
        destination_longitude: i32,
    }

    let mut tx = pool.begin().await?;

    let chair_location_id = Ulid::new().to_string();
    let recorded_at = chrono::Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO chair_locations (id, chair_id, latitude, longitude, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&chair_location_id)
    .bind(&chair.id)
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(recorded_at)
    .execute(&mut *tx)
    .await?;

    let ride: Option<CurrentRide> = sqlx::query_as(
        r#"
SELECT rides.id,
       rides.evaluation,
       rides.pickup_latitude,
       rides.pickup_longitude,
       rides.destination_latitude,
       rides.destination_longitude
FROM rides
WHERE rides.chair_id = ?
ORDER BY rides.updated_at DESC
LIMIT 1
        "#,
    )
    .bind(&chair.id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(ride) = ride {
        let is_pickup =
            req.latitude == ride.pickup_latitude && req.longitude == ride.pickup_longitude;
        let is_destination = req.latitude == ride.destination_latitude
            && req.longitude == ride.destination_longitude;

        if ride.evaluation.is_none() && (is_pickup || is_destination) {
            let evaluation: Option<i32> =
                sqlx::query_scalar("SELECT evaluation FROM rides WHERE id = ? FOR UPDATE")
                    .bind(&ride.id)
                    .fetch_one(&mut *tx)
                    .await?;
            // The earlier current-ride query is a consistent read and can establish a
            // REPEATABLE READ snapshot before this request waits for the ride lock.
            // Use a locking read here so the transition decision sees the status
            // committed by the previous ride-lock holder instead of that old snapshot.
            let latest_status: String = sqlx::query_scalar(
                "SELECT status FROM ride_statuses WHERE ride_id = ? ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
            )
            .bind(&ride.id)
            .fetch_one(&mut *tx)
            .await?;
            let next_status = match latest_status.as_str() {
                "ENROUTE" if is_pickup => Some("PICKUP"),
                "CARRYING" if is_destination => Some("ARRIVED"),
                _ => None,
            };
            if evaluation.is_none() {
                if let Some(next_status) = next_status {
                    sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
                        .bind(Ulid::new().to_string())
                        .bind(&ride.id)
                        .bind(next_status)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
    }

    tx.commit().await?;

    Ok(axum::Json(ChairPostCoordinateResponse {
        recorded_at: recorded_at.and_utc().timestamp_millis(),
    }))
}

#[derive(Debug, serde::Serialize)]
struct SimpleUser {
    id: String,
    name: String,
}

#[derive(Debug, serde::Serialize)]
struct ChairGetNotificationResponse {
    data: Option<ChairGetNotificationResponseData>,
    retry_after_ms: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
struct ChairGetNotificationResponseData {
    ride_id: String,
    user: SimpleUser,
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
    status: String,
}

async fn chair_get_notification(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
) -> Result<axum::Json<ChairGetNotificationResponse>, Error> {
    let ride_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM rides WHERE chair_id = ? ORDER BY updated_at DESC LIMIT 1")
            .bind(&chair.id)
            .fetch_optional(&pool)
            .await?;
    if ride_exists.is_none() {
        return Ok(axum::Json(ChairGetNotificationResponse {
            data: None,
            retry_after_ms: Some(30),
        }));
    }

    // ライドがある場合は、通知対象の読み取りから通知済み更新までを
    // 同じトランザクションに閉じ込める。
    let mut tx = pool.begin().await?;
    let ride: Ride =
        sqlx::query_as("SELECT * FROM rides WHERE chair_id = ? ORDER BY updated_at DESC LIMIT 1")
            .bind(&chair.id)
            .fetch_one(&mut *tx)
            .await?;

    let yet_sent_ride_status: Option<RideStatus> =
        sqlx::query_as("SELECT * FROM ride_statuses WHERE ride_id = ? AND chair_sent_at IS NULL ORDER BY created_at ASC LIMIT 1")
        .bind(&ride.id)
        .fetch_optional(&mut *tx)
        .await?;
    let (yet_sent_ride_status_id, status) = if let Some(yet_sent_ride_status) = yet_sent_ride_status
    {
        (Some(yet_sent_ride_status.id), yet_sent_ride_status.status)
    } else {
        (
            None,
            crate::get_latest_ride_status(&mut *tx, &ride.id).await?,
        )
    };

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ? FOR SHARE")
        .bind(ride.user_id)
        .fetch_one(&mut *tx)
        .await?;

    if let Some(yet_sent_ride_status_id) = yet_sent_ride_status_id {
        sqlx::query("UPDATE ride_statuses SET chair_sent_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(yet_sent_ride_status_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(axum::Json(ChairGetNotificationResponse {
        data: Some(ChairGetNotificationResponseData {
            ride_id: ride.id,
            user: SimpleUser {
                id: user.id,
                name: format!("{} {}", user.firstname, user.lastname),
            },
            pickup_coordinate: Coordinate {
                latitude: ride.pickup_latitude,
                longitude: ride.pickup_longitude,
            },
            destination_coordinate: Coordinate {
                latitude: ride.destination_latitude,
                longitude: ride.destination_longitude,
            },
            status,
        }),
        retry_after_ms: Some(30),
    }))
}

#[derive(Debug, serde::Deserialize)]
struct PostChairRidesRideIDStatusRequest {
    status: String,
}

async fn chair_post_ride_status(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    Path((ride_id,)): Path<(String,)>,
    axum::Json(req): axum::Json<PostChairRidesRideIDStatusRequest>,
) -> Result<StatusCode, Error> {
    let mut tx = pool.begin().await?;

    let Some(ride): Option<Ride> = sqlx::query_as("SELECT * FROM rides WHERE id = ? FOR UPDATE")
        .bind(ride_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Err(Error::NotFound("rides not found"));
    };

    if ride.chair_id.is_none_or(|chair_id| chair_id != chair.id) {
        return Err(Error::BadRequest("not assigned to this ride"));
    }
    if ride.evaluation.is_some() {
        return Err(Error::BadRequest("ride already completed"));
    }

    match req.status.as_str() {
        // Acknowledge the ride
        "ENROUTE" => {
            let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;
            if status == "ENROUTE" {
                return Ok(StatusCode::NO_CONTENT);
            }
            if status != "MATCHING" {
                return Err(Error::BadRequest("ride is not waiting for acknowledgment"));
            }
            sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
                .bind(Ulid::new().to_string())
                .bind(ride.id)
                .bind("ENROUTE")
                .execute(&mut *tx)
                .await?;
        }
        // After Picking up user
        "CARRYING" => {
            let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;
            if status != "PICKUP" {
                return Err(Error::BadRequest("chair has not arrived yet"));
            }
            sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
                .bind(Ulid::new().to_string())
                .bind(ride.id)
                .bind("CARRYING")
                .execute(&mut *tx)
                .await?;
        }
        _ => {
            return Err(Error::BadRequest("invalid status"));
        }
    };

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
