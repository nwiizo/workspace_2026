use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use std::io::Write as _;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use std::time::Instant;
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

const COORDINATE_DIAGNOSTIC_SAMPLE_EVERY: u64 = 64;
static COORDINATE_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static COORDINATE_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(serde::Serialize)]
struct CoordinateDiagnosticSample {
    sequence: u64,
    cache_lookup_us: u64,
    pool_begin_us: u64,
    history_insert_us: u64,
    current_write_us: u64,
    ride_lookup_us: u64,
    transition_us: u64,
    commit_us: u64,
    cache_update_us: u64,
    total_us: u64,
    current_write_path: &'static str,
    transition_candidate: bool,
    transition_inserted: bool,
    outcome: &'static str,
    terminal_phase: &'static str,
}

struct CoordinateDiagnostic {
    started_at: Instant,
    checkpoint_at: Instant,
    sample: CoordinateDiagnosticSample,
    emitted: bool,
}

impl CoordinateDiagnostic {
    fn sampled() -> Option<Self> {
        let enabled = *COORDINATE_DIAGNOSTICS_ENABLED.get_or_init(|| {
            std::env::var_os("ISUCON_DIAGNOSTIC").as_deref() == Some(std::ffi::OsStr::new("1"))
        });
        if !enabled {
            return None;
        }

        let sequence = COORDINATE_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        if sequence.checked_rem(COORDINATE_DIAGNOSTIC_SAMPLE_EVERY) != Some(0) {
            return None;
        }

        let started_at = Instant::now();
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            sample: CoordinateDiagnosticSample {
                sequence,
                cache_lookup_us: 0,
                pool_begin_us: 0,
                history_insert_us: 0,
                current_write_us: 0,
                ride_lookup_us: 0,
                transition_us: 0,
                commit_us: 0,
                cache_update_us: 0,
                total_us: 0,
                current_write_path: "unknown",
                transition_candidate: false,
                transition_inserted: false,
                outcome: "error_or_cancelled",
                terminal_phase: "cache_lookup",
            },
            emitted: false,
        })
    }

    fn elapsed_since_checkpoint_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.checkpoint_at).as_micros();
        self.checkpoint_at = now;
        elapsed.min(u128::from(u64::MAX)) as u64
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        let total_us = self.started_at.elapsed().as_micros();
        self.sample.total_us = total_us.min(u128::from(u64::MAX)) as u64;
        if let Ok(json) = serde_json::to_string(&self.sample) {
            let _ = writeln!(std::io::stdout().lock(), "COORDINATE_DIAGNOSTIC {json}");
        }
    }

    fn emit_success(mut self) {
        self.sample.outcome = "success";
        self.sample.terminal_phase = "complete";
        self.emit_record();
    }
}

impl Drop for CoordinateDiagnostic {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_record();
        }
    }
}

async fn upsert_chair_current_location(
    tx: &mut sqlx::MySqlConnection,
    chair_id: &str,
    location_id: &str,
    coordinate: &Coordinate,
    recorded_at: chrono::NaiveDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
INSERT INTO chair_current_locations (
    chair_id,
    location_id,
    latitude,
    longitude,
    created_at
)
VALUES (?, ?, ?, ?, ?) AS new
ON DUPLICATE KEY UPDATE
    latitude = IF(
        new.created_at > chair_current_locations.created_at
            OR (
                new.created_at = chair_current_locations.created_at
                AND new.location_id > chair_current_locations.location_id
            ),
        new.latitude,
        chair_current_locations.latitude
    ),
    longitude = IF(
        new.created_at > chair_current_locations.created_at
            OR (
                new.created_at = chair_current_locations.created_at
                AND new.location_id > chair_current_locations.location_id
            ),
        new.longitude,
        chair_current_locations.longitude
    ),
    location_id = IF(
        new.created_at > chair_current_locations.created_at
            OR (
                new.created_at = chair_current_locations.created_at
                AND new.location_id > chair_current_locations.location_id
            ),
        new.location_id,
        chair_current_locations.location_id
    ),
    created_at = GREATEST(new.created_at, chair_current_locations.created_at)
        "#,
    )
    .bind(chair_id)
    .bind(location_id)
    .bind(coordinate.latitude)
    .bind(coordinate.longitude)
    .bind(recorded_at)
    .execute(tx)
    .await?;
    Ok(())
}

async fn chair_post_coordinate(
    State(AppState {
        pool,
        latest_chair_locations,
        notification_cache,
        ..
    }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    axum::Json(req): axum::Json<Coordinate>,
) -> Result<axum::Json<ChairPostCoordinateResponse>, Error> {
    #[derive(sqlx::FromRow)]
    struct CurrentRide {
        id: String,
        user_id: String,
        evaluation: Option<i32>,
        pickup_latitude: i32,
        pickup_longitude: i32,
        destination_latitude: i32,
        destination_longitude: i32,
    }

    let mut diagnostic = CoordinateDiagnostic::sampled();
    let current_location_exists = latest_chair_locations.contains(&chair.id).await;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.cache_lookup_us = elapsed_us;
        diagnostic.sample.terminal_phase = "pool_begin";
    }
    let mut tx = pool.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.pool_begin_us = elapsed_us;
        diagnostic.sample.terminal_phase = "history_insert";
    }
    let mut notification_user_id = None;

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
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.history_insert_us = elapsed_us;
        diagnostic.sample.terminal_phase = "current_write";
    }

    if current_location_exists {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.current_write_path = "update";
        }
        let current_location_update = sqlx::query(
            r#"
UPDATE chair_current_locations
SET location_id = ?,
    latitude = ?,
    longitude = ?,
    created_at = ?
WHERE chair_id = ?
  AND (
      created_at < ?
      OR (created_at = ? AND location_id < ?)
  )
        "#,
        )
        .bind(&chair_location_id)
        .bind(req.latitude)
        .bind(req.longitude)
        .bind(recorded_at)
        .bind(&chair.id)
        .bind(recorded_at)
        .bind(recorded_at)
        .bind(&chair_location_id)
        .execute(&mut *tx)
        .await?;

        if current_location_update.rows_affected() == 0 {
            if let Some(diagnostic) = &mut diagnostic {
                diagnostic.sample.current_write_path = "update_fallback";
            }
            // A stale cache or a concurrent newer update can make the guarded
            // UPDATE affect zero rows. The atomic fallback repairs both cases.
            upsert_chair_current_location(
                &mut tx,
                &chair.id,
                &chair_location_id,
                &req,
                recorded_at,
            )
            .await?;
        }
    } else {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.current_write_path = "upsert_missing";
        }
        // Updating a missing row under REPEATABLE READ acquires a gap lock. Many
        // first-coordinate transactions can then deadlock when they all insert.
        // Start with one atomic upsert when the cache says no current row exists.
        upsert_chair_current_location(&mut tx, &chair.id, &chair_location_id, &req, recorded_at)
            .await?;
    }
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.current_write_us = elapsed_us;
        diagnostic.sample.terminal_phase = "ride_lookup";
    }

    let ride: Option<CurrentRide> = sqlx::query_as(
        r#"
SELECT rides.id,
       rides.user_id,
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
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.ride_lookup_us = elapsed_us;
        diagnostic.sample.terminal_phase = "transition";
    }
    if let Some(ride) = ride {
        let is_pickup =
            req.latitude == ride.pickup_latitude && req.longitude == ride.pickup_longitude;
        let is_destination = req.latitude == ride.destination_latitude
            && req.longitude == ride.destination_longitude;

        if ride.evaluation.is_none() && (is_pickup || is_destination) {
            if let Some(diagnostic) = &mut diagnostic {
                diagnostic.sample.transition_candidate = true;
            }
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
                "SELECT status FROM ride_statuses WHERE ride_id = ? ORDER BY status DESC LIMIT 1 FOR UPDATE",
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
                    notification_user_id = Some(ride.user_id);
                    if let Some(diagnostic) = &mut diagnostic {
                        diagnostic.sample.transition_inserted = true;
                    }
                }
            }
        }
    }
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.transition_us = elapsed_us;
        diagnostic.sample.terminal_phase = "commit";
    }

    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.commit_us = elapsed_us;
        diagnostic.sample.terminal_phase = "cache_update";
    }
    if let Some(user_id) = notification_user_id {
        notification_cache.invalidate_app(&user_id);
        notification_cache.invalidate_chair(&chair.id);
    }
    latest_chair_locations
        .update(
            chair.id,
            chair_location_id,
            req.latitude,
            req.longitude,
            recorded_at,
        )
        .await;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.cache_update_us = elapsed_us;
    }

    let response = axum::Json(ChairPostCoordinateResponse {
        recorded_at: recorded_at.and_utc().timestamp_millis(),
    });
    if let Some(diagnostic) = diagnostic {
        diagnostic.emit_success();
    }
    Ok(response)
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
    State(AppState {
        pool,
        notification_cache,
        ..
    }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
) -> Result<Response, Error> {
    let (cached_payload, cache_revision) = notification_cache.chair(&chair.id);
    if let Some(cached_payload) = cached_payload {
        return Ok(crate::json_bytes_response(cached_payload));
    }

    let ride_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM rides WHERE chair_id = ? ORDER BY updated_at DESC LIMIT 1")
            .bind(&chair.id)
            .fetch_optional(&pool)
            .await?;
    if ride_exists.is_none() {
        let payload = axum::body::Bytes::from(serde_json::to_vec(&ChairGetNotificationResponse {
            data: None,
            retry_after_ms: Some(crate::CACHED_NOTIFICATION_RETRY_AFTER_MS),
        })?);
        notification_cache.insert_chair_if_current(chair.id, cache_revision, payload.clone());
        return Ok(crate::json_bytes_response(payload));
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
        sqlx::query_as("SELECT * FROM ride_statuses WHERE ride_id = ? AND chair_sent_at IS NULL ORDER BY status ASC LIMIT 1")
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

    if let Some(yet_sent_ride_status_id) = &yet_sent_ride_status_id {
        sqlx::query("UPDATE ride_statuses SET chair_sent_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(yet_sent_ride_status_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    let cacheable = yet_sent_ride_status_id.is_none();
    let response = ChairGetNotificationResponse {
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
        retry_after_ms: Some(if cacheable {
            crate::CACHED_NOTIFICATION_RETRY_AFTER_MS
        } else {
            crate::NOTIFICATION_RETRY_AFTER_MS
        }),
    };
    let payload = axum::body::Bytes::from(serde_json::to_vec(&response)?);
    if cacheable {
        notification_cache.insert_chair_if_current(chair.id, cache_revision, payload.clone());
    }
    Ok(crate::json_bytes_response(payload))
}

#[derive(Debug, serde::Deserialize)]
struct PostChairRidesRideIDStatusRequest {
    status: String,
}

async fn chair_post_ride_status(
    State(AppState {
        pool,
        notification_cache,
        ..
    }): State<AppState>,
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
                .bind(&ride.id)
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
                .bind(&ride.id)
                .bind("CARRYING")
                .execute(&mut *tx)
                .await?;
        }
        _ => {
            return Err(Error::BadRequest("invalid status"));
        }
    };

    tx.commit().await?;
    notification_cache.invalidate_app(&ride.user_id);
    notification_cache.invalidate_chair(&chair.id);

    Ok(StatusCode::NO_CONTENT)
}
