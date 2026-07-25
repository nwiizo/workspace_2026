use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};
use std::time::Instant;
use tokio::sync::mpsc;
use ulid::Ulid;

use crate::models::{Chair, Owner, Ride, RideStatus, User};
use crate::notification_diagnostic::{
    NotificationConnectionStage, NotificationDiagnostic, NotificationEndpoint,
};
use crate::{AppState, Coordinate, Error};
use sqlx::Acquire;

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
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    jar: CookieJar,
    axum::Json(req): axum::Json<ChairPostChairsRequest>,
) -> Result<(CookieJar, (StatusCode, axum::Json<ChairPostChairsResponse>)), Error> {
    let _admission_guard = general_db_admission
        .acquire("chair_post_chairs", &pool)
        .await;
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
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    axum::Json(req): axum::Json<PostChairActivityRequest>,
) -> Result<StatusCode, Error> {
    let _admission_guard = general_db_admission
        .acquire("chair_post_activity", &pool)
        .await;
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
    periodic_sample: bool,
    trace_ride: bool,
    ride_id: Option<String>,
    chair_id: Option<String>,
    location_id: Option<String>,
    latitude: Option<i32>,
    longitude: Option<i32>,
    recorded_at_ms: Option<i64>,
    observed_at_unix_us: Option<i64>,
    recorded_at_unix_us: Option<i64>,
    recorded_at_adjustment_us: Option<i64>,
    queued: bool,
    queue_generation: Option<u64>,
    queue_shard: Option<usize>,
    queue_depth_before: Option<usize>,
    queue_enqueue_wait_us: u64,
    queue_admission_wait_us: u64,
    queue_wait_us: u64,
    acknowledged_at_unix_us: Option<u64>,
    transition_status: Option<String>,
    committed_at_unix_us: Option<u64>,
    recorded_to_commit_us: Option<u64>,
    event_at_unix_us: Option<u64>,
    cache_lookup_us: u64,
    pool_acquire_us: u64,
    transaction_begin_us: u64,
    pool_begin_us: u64,
    pool_size_before: Option<u64>,
    pool_idle_before: Option<u64>,
    pool_in_use_before: Option<u64>,
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
    recorded_at_instant: Option<Instant>,
    sample: CoordinateDiagnosticSample,
    force_emit: bool,
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
        let periodic_sample = sequence.checked_rem(COORDINATE_DIAGNOSTIC_SAMPLE_EVERY) == Some(0);

        let started_at = Instant::now();
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            recorded_at_instant: None,
            sample: CoordinateDiagnosticSample {
                sequence,
                periodic_sample,
                trace_ride: false,
                ride_id: None,
                chair_id: None,
                location_id: None,
                latitude: None,
                longitude: None,
                recorded_at_ms: None,
                observed_at_unix_us: None,
                recorded_at_unix_us: None,
                recorded_at_adjustment_us: None,
                queued: false,
                queue_generation: None,
                queue_shard: None,
                queue_depth_before: None,
                queue_enqueue_wait_us: 0,
                queue_admission_wait_us: 0,
                queue_wait_us: 0,
                acknowledged_at_unix_us: None,
                transition_status: None,
                committed_at_unix_us: None,
                recorded_to_commit_us: None,
                event_at_unix_us: None,
                cache_lookup_us: 0,
                pool_acquire_us: 0,
                transaction_begin_us: 0,
                pool_begin_us: 0,
                pool_size_before: None,
                pool_idle_before: None,
                pool_in_use_before: None,
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
            force_emit: false,
            emitted: false,
        })
    }

    fn trace_ride_event(
        &mut self,
        ride_id: &str,
        chair_id: &str,
        coordinate: &Coordinate,
        recorded_at_ms: i64,
    ) {
        if !crate::drive_diagnostic::should_trace_ride(ride_id) {
            return;
        }

        self.force_emit = true;
        self.sample.trace_ride = true;
        self.sample.ride_id = Some(ride_id.to_owned());
        self.sample.chair_id = Some(chair_id.to_owned());
        self.sample.latitude = Some(coordinate.latitude);
        self.sample.longitude = Some(coordinate.longitude);
        self.sample.recorded_at_ms = Some(recorded_at_ms);
    }

    fn elapsed_since_checkpoint_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.checkpoint_at).as_micros();
        self.checkpoint_at = now;
        elapsed.min(u128::from(u64::MAX)) as u64
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        if !self.sample.periodic_sample && !self.force_emit {
            return;
        }
        self.sample.event_at_unix_us = Some(crate::drive_diagnostic::unix_time_us());
        let total_us = self.started_at.elapsed().as_micros();
        self.sample.total_us = total_us.min(u128::from(u64::MAX)) as u64;
        crate::drive_diagnostic::emit("COORDINATE_DIAGNOSTIC", &self.sample);
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

#[derive(Debug, Clone)]
pub struct CoordinateWriteQueue {
    senders: Arc<Vec<mpsc::Sender<QueuedCoordinate>>>,
    generation: Arc<AtomicU64>,
    accepted: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
    stale: Arc<AtomicU64>,
    full: Arc<AtomicU64>,
}

#[derive(Clone)]
struct CoordinateWorkerContext {
    pool: sqlx::MySqlPool,
    latest_chair_locations: crate::LatestChairLocationCache,
    notification_cache: crate::NotificationCache,
    maintenance_lock: Arc<tokio::sync::RwLock<()>>,
    generation: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
    stale: Arc<AtomicU64>,
    admission: Arc<tokio::sync::Semaphore>,
}

struct QueuedCoordinate {
    chair_id: String,
    coordinate: Coordinate,
    location_id: String,
    recorded_at: chrono::NaiveDateTime,
    recorded_at_instant: Instant,
    enqueued_at: Instant,
    generation: u64,
    diagnostic: Option<CoordinateDiagnostic>,
}

impl CoordinateWriteQueue {
    pub fn spawn(
        shards: usize,
        capacity_per_shard: usize,
        max_in_flight: usize,
        pool: sqlx::MySqlPool,
        latest_chair_locations: crate::LatestChairLocationCache,
        notification_cache: crate::NotificationCache,
        maintenance_lock: Arc<tokio::sync::RwLock<()>>,
    ) -> Self {
        assert!(shards > 0, "coordinate queue must have at least one shard");
        assert!(
            capacity_per_shard > 0,
            "coordinate queue capacity must be positive"
        );
        assert!(
            max_in_flight > 0 && max_in_flight <= shards,
            "coordinate queue in-flight limit must be in 1..=shards"
        );

        let generation = Arc::new(AtomicU64::new(0));
        let accepted = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicU64::new(0));
        let stale = Arc::new(AtomicU64::new(0));
        let full = Arc::new(AtomicU64::new(0));
        let admission = Arc::new(tokio::sync::Semaphore::new(max_in_flight));
        let mut senders = Vec::with_capacity(shards);

        for shard in 0..shards {
            let (sender, mut receiver) = mpsc::channel::<QueuedCoordinate>(capacity_per_shard);
            senders.push(sender);
            let context = CoordinateWorkerContext {
                pool: pool.clone(),
                latest_chair_locations: latest_chair_locations.clone(),
                notification_cache: notification_cache.clone(),
                maintenance_lock: maintenance_lock.clone(),
                generation: generation.clone(),
                completed: completed.clone(),
                failed: failed.clone(),
                stale: stale.clone(),
                admission: admission.clone(),
            };
            tokio::spawn(async move {
                while let Some(mut job) = receiver.recv().await {
                    let admission_started_at = Instant::now();
                    let _admission_guard = context
                        .admission
                        .acquire()
                        .await
                        .expect("coordinate queue admission semaphore remains open");
                    if let Some(diagnostic) = &mut job.diagnostic {
                        diagnostic.sample.queue_admission_wait_us =
                            crate::drive_diagnostic::duration_us(admission_started_at.elapsed());
                    }
                    let _maintenance_guard = context.maintenance_lock.read().await;
                    if job.generation != context.generation.load(Ordering::Acquire) {
                        context.stale.fetch_add(1, Ordering::Relaxed);
                        if let Some(mut diagnostic) = job.diagnostic.take() {
                            diagnostic.force_emit = true;
                            diagnostic.sample.outcome = "stale_generation";
                            diagnostic.sample.terminal_phase = "queue_generation";
                            diagnostic.emit_record();
                        }
                        continue;
                    }

                    if let Err(error) = persist_queued_coordinate(&context, &mut job).await {
                        context.failed.fetch_add(1, Ordering::Relaxed);
                        tracing::error!(
                            %error,
                            shard,
                            chair_id = %job.chair_id,
                            location_id = %job.location_id,
                            "queued coordinate persistence failed after HTTP acknowledgement"
                        );
                    } else {
                        context.completed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }

        tracing::info!(
            shards,
            capacity_per_shard,
            max_in_flight,
            "enabled asynchronous coordinate write queue"
        );
        Self {
            senders: Arc::new(senders),
            generation,
            accepted,
            completed,
            failed,
            stale,
            full,
        }
    }

    pub fn advance_generation(&self) {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        tracing::info!(
            generation,
            accepted = self.accepted.load(Ordering::Relaxed),
            completed = self.completed.load(Ordering::Relaxed),
            failed = self.failed.load(Ordering::Relaxed),
            stale = self.stale.load(Ordering::Relaxed),
            full = self.full.load(Ordering::Relaxed),
            "advanced coordinate queue generation for initialization"
        );
    }

    fn enqueue(
        &self,
        chair_id: String,
        coordinate: Coordinate,
        location_id: String,
        recorded_at: chrono::NaiveDateTime,
        recorded_at_instant: Instant,
        mut diagnostic: Option<CoordinateDiagnostic>,
    ) -> Result<(), Error> {
        let shard = coordinate_queue_shard(&chair_id, self.senders.len());
        let sender = &self.senders[shard];
        let depth_before = sender.max_capacity().saturating_sub(sender.capacity());
        let generation = self.generation.load(Ordering::Acquire);
        let enqueue_started_at = Instant::now();
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.queued = true;
            diagnostic.sample.queue_generation = Some(generation);
            diagnostic.sample.queue_shard = Some(shard);
            diagnostic.sample.queue_depth_before = Some(depth_before);
            diagnostic.sample.terminal_phase = "queue_enqueue";
        }
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.queue_enqueue_wait_us =
                crate::drive_diagnostic::duration_us(enqueue_started_at.elapsed());
            diagnostic.sample.acknowledged_at_unix_us =
                Some(crate::drive_diagnostic::unix_time_us());
            diagnostic.sample.terminal_phase = "queue_wait";
        }
        let job = QueuedCoordinate {
            chair_id,
            coordinate,
            location_id,
            recorded_at,
            recorded_at_instant,
            enqueued_at: Instant::now(),
            generation,
            diagnostic,
        };
        match sender.try_send(job) {
            Ok(()) => {
                self.accepted.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(mut job)) => {
                self.full.fetch_add(1, Ordering::Relaxed);
                if let Some(mut diagnostic) = job.diagnostic.take() {
                    diagnostic.force_emit = true;
                    diagnostic.sample.acknowledged_at_unix_us = None;
                    diagnostic.sample.outcome = "queue_full";
                    diagnostic.sample.terminal_phase = "queue_enqueue";
                    diagnostic.emit_record();
                }
                tracing::warn!(
                    shard,
                    chair_id = %job.chair_id,
                    location_id = %job.location_id,
                    "coordinate write queue is full; request was not acknowledged"
                );
                Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "coordinate write queue is full",
                )
                .into())
            }
            Err(mpsc::error::TrySendError::Closed(mut job)) => {
                if let Some(mut diagnostic) = job.diagnostic.take() {
                    diagnostic.force_emit = true;
                    diagnostic.sample.acknowledged_at_unix_us = None;
                    diagnostic.sample.outcome = "queue_closed";
                    diagnostic.sample.terminal_phase = "queue_enqueue";
                    diagnostic.emit_record();
                }
                tracing::error!(
                    shard,
                    chair_id = %job.chair_id,
                    location_id = %job.location_id,
                    "coordinate write queue worker stopped; request was not acknowledged"
                );
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "coordinate write queue worker stopped",
                )
                .into())
            }
        }
    }
}

fn coordinate_queue_shard(chair_id: &str, shards: usize) -> usize {
    // FNV-1a is deterministic across processes and does not allocate.
    let hash = chair_id.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    usize::try_from(hash % u64::try_from(shards).expect("shard count fits u64"))
        .expect("shard index fits usize")
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

#[derive(sqlx::FromRow)]
struct CoordinateCurrentRide {
    id: String,
    user_id: String,
    evaluation: Option<i32>,
    pickup_latitude: i32,
    pickup_longitude: i32,
    destination_latitude: i32,
    destination_longitude: i32,
}

async fn persist_queued_coordinate(
    context: &CoordinateWorkerContext,
    job: &mut QueuedCoordinate,
) -> Result<(), Error> {
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.queue_wait_us =
            crate::drive_diagnostic::duration_us(job.enqueued_at.elapsed());
        diagnostic.checkpoint_at = Instant::now();
        diagnostic.sample.terminal_phase = "cache_lookup";
    }
    let current_location_exists = context.latest_chair_locations.contains(&job.chair_id).await;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.cache_lookup_us = diagnostic.elapsed_since_checkpoint_us();
        let pool_size = u64::from(context.pool.size());
        let pool_idle = u64::try_from(context.pool.num_idle()).unwrap_or(u64::MAX);
        diagnostic.sample.pool_size_before = Some(pool_size);
        diagnostic.sample.pool_idle_before = Some(pool_idle);
        diagnostic.sample.pool_in_use_before = Some(pool_size.saturating_sub(pool_idle));
        diagnostic.sample.terminal_phase = "pool_acquire";
    }

    let mut connection = context.pool.acquire().await?;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.pool_acquire_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "transaction_begin";
    }
    let mut tx = connection.begin().await?;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.transaction_begin_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.pool_begin_us = diagnostic
            .sample
            .pool_acquire_us
            .saturating_add(diagnostic.sample.transaction_begin_us);
        diagnostic.sample.terminal_phase = "history_insert";
    }
    let mut notification_user_id = None;

    sqlx::query(
        "INSERT INTO chair_locations (id, chair_id, latitude, longitude, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&job.location_id)
    .bind(&job.chair_id)
    .bind(job.coordinate.latitude)
    .bind(job.coordinate.longitude)
    .bind(job.recorded_at)
    .execute(&mut *tx)
    .await?;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.history_insert_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "current_write";
    }

    if current_location_exists {
        if let Some(diagnostic) = &mut job.diagnostic {
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
        .bind(&job.location_id)
        .bind(job.coordinate.latitude)
        .bind(job.coordinate.longitude)
        .bind(job.recorded_at)
        .bind(&job.chair_id)
        .bind(job.recorded_at)
        .bind(job.recorded_at)
        .bind(&job.location_id)
        .execute(&mut *tx)
        .await?;

        if current_location_update.rows_affected() == 0 {
            if let Some(diagnostic) = &mut job.diagnostic {
                diagnostic.sample.current_write_path = "update_fallback";
            }
            upsert_chair_current_location(
                &mut tx,
                &job.chair_id,
                &job.location_id,
                &job.coordinate,
                job.recorded_at,
            )
            .await?;
        }
    } else {
        if let Some(diagnostic) = &mut job.diagnostic {
            diagnostic.sample.current_write_path = "upsert_missing";
        }
        upsert_chair_current_location(
            &mut tx,
            &job.chair_id,
            &job.location_id,
            &job.coordinate,
            job.recorded_at,
        )
        .await?;
    }
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.current_write_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "ride_lookup";
    }

    let ride: Option<CoordinateCurrentRide> = sqlx::query_as(
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
    .bind(&job.chair_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.ride_lookup_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "transition";
    }
    if let Some(ride) = ride {
        if ride.evaluation.is_none() {
            if let Some(diagnostic) = &mut job.diagnostic {
                diagnostic.trace_ride_event(
                    &ride.id,
                    &job.chair_id,
                    &job.coordinate,
                    job.recorded_at.and_utc().timestamp_millis(),
                );
            }
        }
        let is_pickup = job.coordinate.latitude == ride.pickup_latitude
            && job.coordinate.longitude == ride.pickup_longitude;
        let is_destination = job.coordinate.latitude == ride.destination_latitude
            && job.coordinate.longitude == ride.destination_longitude;

        if ride.evaluation.is_none() && (is_pickup || is_destination) {
            if let Some(diagnostic) = &mut job.diagnostic {
                diagnostic.sample.transition_candidate = true;
            }
            let evaluation: Option<i32> =
                sqlx::query_scalar("SELECT evaluation FROM rides WHERE id = ? FOR UPDATE")
                    .bind(&ride.id)
                    .fetch_one(&mut *tx)
                    .await?;
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
                    if let Some(diagnostic) = &mut job.diagnostic {
                        diagnostic.sample.transition_inserted = true;
                        diagnostic.sample.transition_status = Some(next_status.to_owned());
                    }
                }
            }
        }
    }
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.transition_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "commit";
    }

    tx.commit().await?;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.commit_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.committed_at_unix_us = Some(crate::drive_diagnostic::unix_time_us());
        diagnostic.sample.recorded_to_commit_us = Some(crate::drive_diagnostic::duration_us(
            job.recorded_at_instant.elapsed(),
        ));
        if diagnostic
            .sample
            .recorded_to_commit_us
            .is_some_and(|elapsed_us| elapsed_us >= 1_000_000)
        {
            diagnostic.force_emit = true;
        }
        diagnostic.sample.terminal_phase = "cache_update";
    }
    drop(connection);

    if let Some(user_id) = notification_user_id {
        context.notification_cache.invalidate_app(&user_id);
        context.notification_cache.invalidate_chair(&job.chair_id);
    }
    context
        .latest_chair_locations
        .update(
            job.chair_id.clone(),
            job.location_id.clone(),
            job.coordinate.latitude,
            job.coordinate.longitude,
            job.recorded_at,
        )
        .await;
    if let Some(diagnostic) = &mut job.diagnostic {
        diagnostic.sample.cache_update_us = diagnostic.elapsed_since_checkpoint_us();
    }
    if let Some(diagnostic) = job.diagnostic.take() {
        diagnostic.emit_success();
    }
    Ok(())
}

async fn chair_post_coordinate(
    State(AppState {
        coordinate_pool: pool,
        latest_chair_locations,
        notification_cache,
        coordinate_write_queue,
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
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.chair_id = Some(chair.id.clone());
        diagnostic.sample.latitude = Some(req.latitude);
        diagnostic.sample.longitude = Some(req.longitude);
    }
    if let Some(queue) = coordinate_write_queue {
        let location_id = Ulid::new().to_string();
        let recorded_at_instant = Instant::now();
        let observed_at = chrono::Utc::now().naive_utc();
        let recorded_at = latest_chair_locations.reserve_recorded_at(&chair.id, observed_at);
        if let Some(diagnostic) = &mut diagnostic {
            let recorded_at_adjustment_us = recorded_at
                .signed_duration_since(observed_at)
                .num_microseconds()
                .unwrap_or(i64::MAX);
            diagnostic.recorded_at_instant = Some(recorded_at_instant);
            diagnostic.sample.location_id = Some(location_id.clone());
            diagnostic.sample.observed_at_unix_us = Some(observed_at.and_utc().timestamp_micros());
            diagnostic.sample.recorded_at_unix_us = Some(recorded_at.and_utc().timestamp_micros());
            diagnostic.sample.recorded_at_adjustment_us = Some(recorded_at_adjustment_us);
            if recorded_at_adjustment_us > 0 {
                diagnostic.force_emit = true;
            }
        }
        queue.enqueue(
            chair.id,
            req,
            location_id,
            recorded_at,
            recorded_at_instant,
            diagnostic,
        )?;
        return Ok(axum::Json(ChairPostCoordinateResponse {
            recorded_at: recorded_at.and_utc().timestamp_millis(),
        }));
    }
    let current_location_exists = latest_chair_locations.contains(&chair.id).await;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.cache_lookup_us = elapsed_us;
        let pool_size = u64::from(pool.size());
        let pool_idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        diagnostic.sample.pool_size_before = Some(pool_size);
        diagnostic.sample.pool_idle_before = Some(pool_idle);
        diagnostic.sample.pool_in_use_before = Some(pool_size.saturating_sub(pool_idle));
        diagnostic.sample.terminal_phase = "pool_acquire";
    }
    let mut connection = pool.acquire().await?;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.pool_acquire_us = elapsed_us;
        diagnostic.sample.terminal_phase = "transaction_begin";
    }
    let mut tx = connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.transaction_begin_us = elapsed_us;
        diagnostic.sample.pool_begin_us =
            diagnostic.sample.pool_acquire_us.saturating_add(elapsed_us);
        diagnostic.sample.terminal_phase = "history_insert";
    }
    let mut notification_user_id = None;

    let chair_location_id = Ulid::new().to_string();
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.location_id = Some(chair_location_id.clone());
    }
    let recorded_at_instant = Instant::now();
    let observed_at = chrono::Utc::now().naive_utc();
    let recorded_at = latest_chair_locations.reserve_recorded_at(&chair.id, observed_at);
    if let Some(diagnostic) = &mut diagnostic {
        let recorded_at_adjustment_us = recorded_at
            .signed_duration_since(observed_at)
            .num_microseconds()
            .unwrap_or(i64::MAX);
        diagnostic.recorded_at_instant = Some(recorded_at_instant);
        diagnostic.sample.observed_at_unix_us = Some(observed_at.and_utc().timestamp_micros());
        diagnostic.sample.recorded_at_unix_us = Some(recorded_at.and_utc().timestamp_micros());
        diagnostic.sample.recorded_at_adjustment_us = Some(recorded_at_adjustment_us);
        if recorded_at_adjustment_us > 0 {
            diagnostic.force_emit = true;
        }
    }
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
        if ride.evaluation.is_none() {
            if let Some(diagnostic) = &mut diagnostic {
                diagnostic.trace_ride_event(
                    &ride.id,
                    &chair.id,
                    &req,
                    recorded_at.and_utc().timestamp_millis(),
                );
            }
        }
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
                        diagnostic.sample.transition_status = Some(next_status.to_owned());
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
        let committed_at_unix_us = crate::drive_diagnostic::unix_time_us();
        diagnostic.sample.committed_at_unix_us = Some(committed_at_unix_us);
        diagnostic.sample.recorded_to_commit_us =
            diagnostic.recorded_at_instant.map(|recorded_at_instant| {
                crate::drive_diagnostic::duration_us(recorded_at_instant.elapsed())
            });
        if diagnostic
            .sample
            .recorded_to_commit_us
            .is_some_and(|elapsed_us| elapsed_us >= 1_000_000)
        {
            diagnostic.force_emit = true;
        }
        diagnostic.sample.terminal_phase = "cache_update";
    }
    drop(connection);
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
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
) -> Result<Response, Error> {
    let mut diagnostic = NotificationDiagnostic::sampled(NotificationEndpoint::Chair);
    let (cached_payload, cache_revision) = notification_cache.chair(&chair.id);
    if let Some(diagnostic) = &mut diagnostic {
        let elapsed_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.cache_lookup_us = Some(elapsed_us);
    }
    if let Some(cached_payload) = cached_payload {
        let response = crate::json_bytes_response(cached_payload);
        if let Some(mut diagnostic) = diagnostic {
            diagnostic.sample.path = "cache_hit";
            diagnostic.sample.terminal_phase = "response";
            diagnostic.sample.response_us = Some(diagnostic.elapsed_since_checkpoint_us());
            diagnostic.emit_success();
        }
        return Ok(response);
    }

    let admission_guard = general_db_admission
        .acquire("chair_get_notification", &pool)
        .await;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.observe_pool(&pool, NotificationConnectionStage::InitialLookup);
        diagnostic.sample.terminal_phase = "initial_pool_acquire";
    }
    let mut initial_connection = pool.acquire().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_acquired(NotificationConnectionStage::InitialLookup);
        diagnostic.sample.initial_pool_acquire_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "latest_ride_query";
    }
    let ride_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM rides WHERE chair_id = ? ORDER BY updated_at DESC LIMIT 1")
            .bind(&chair.id)
            .fetch_optional(&mut *initial_connection)
            .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.latest_ride_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }
    if ride_exists.is_none() {
        drop(initial_connection);
        drop(admission_guard);
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.connection_released();
        }
        let payload = axum::body::Bytes::from(serde_json::to_vec(&ChairGetNotificationResponse {
            data: None,
            retry_after_ms: Some(crate::CACHED_NOTIFICATION_RETRY_AFTER_MS),
        })?);
        notification_cache.insert_chair_if_current(chair.id, cache_revision, payload.clone());
        let response = crate::json_bytes_response(payload);
        if let Some(mut diagnostic) = diagnostic {
            diagnostic.sample.path = "no_ride";
            diagnostic.sample.cache_insert_attempted = true;
            diagnostic.sample.terminal_phase = "response";
            diagnostic.sample.response_us = Some(diagnostic.elapsed_since_checkpoint_us());
            diagnostic.emit_success();
        }
        return Ok(response);
    }

    // ライドがある場合は、通知対象の読み取りから通知済み更新までを
    // 同じトランザクションに閉じ込める。
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.reuse_connection_for_transaction();
        diagnostic.sample.terminal_phase = "transaction_begin";
    }
    let mut tx = initial_connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.transaction_begin_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "ride_query";
    }
    let ride: Ride = sqlx::query_as(
        r#"
SELECT rides.*
FROM rides
LEFT JOIN ride_statuses AS matching_status
       ON matching_status.ride_id = rides.id
      AND matching_status.status = 'MATCHING'
LEFT JOIN ride_statuses AS completed_status
       ON completed_status.ride_id = rides.id
      AND completed_status.status = 'COMPLETED'
WHERE rides.chair_id = ?
ORDER BY CASE
    WHEN matching_status.chair_sent_at IS NOT NULL
     AND completed_status.chair_sent_at IS NULL THEN 0
    WHEN matching_status.id IS NOT NULL
     AND matching_status.chair_sent_at IS NULL
     AND completed_status.chair_sent_at IS NULL THEN 1
    ELSE 2
END,
matching_status.chair_sent_at DESC,
rides.updated_at DESC,
rides.created_at DESC,
rides.id DESC
LIMIT 1
        "#,
    )
    .bind(&chair.id)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.ride_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "pending_status_query";
    }

    let yet_sent_ride_status: Option<RideStatus> =
        sqlx::query_as("SELECT * FROM ride_statuses WHERE ride_id = ? AND chair_sent_at IS NULL ORDER BY status ASC LIMIT 1")
        .bind(&ride.id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.pending_status_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }
    let (yet_sent_ride_status_id, status) = if let Some(yet_sent_ride_status) = yet_sent_ride_status
    {
        (Some(yet_sent_ride_status.id), yet_sent_ride_status.status)
    } else {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.terminal_phase = "latest_status_query";
        }
        let latest_status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.latest_status_query_us =
                Some(diagnostic.elapsed_since_checkpoint_us());
        }
        (None, latest_status)
    };
    if yet_sent_ride_status_id.is_some() {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.trace_ride_event(&ride.id, &status, &chair.id);
        }
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.terminal_phase = "user_query";
    }
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ? FOR SHARE")
        .bind(ride.user_id)
        .fetch_one(&mut *tx)
        .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.user_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }

    if let Some(yet_sent_ride_status_id) = &yet_sent_ride_status_id {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.terminal_phase = "sent_update";
        }
        sqlx::query("UPDATE ride_statuses SET chair_sent_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(yet_sent_ride_status_id)
            .execute(&mut *tx)
            .await?;
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.sent_update_us = Some(diagnostic.elapsed_since_checkpoint_us());
        }
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.terminal_phase = "commit";
    }
    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.commit_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }
    drop(initial_connection);
    drop(admission_guard);
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_released();
        diagnostic.sample.terminal_phase = "response";
    }

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
    let response = crate::json_bytes_response(payload);
    if let Some(mut diagnostic) = diagnostic {
        diagnostic.sample.path = if yet_sent_ride_status_id.is_some() {
            "pending_status"
        } else {
            "steady_state"
        };
        diagnostic.sample.cache_insert_attempted = cacheable;
        diagnostic.sample.response_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.emit_success();
    }
    Ok(response)
}

#[derive(Debug, serde::Deserialize)]
struct PostChairRidesRideIDStatusRequest {
    status: String,
}

#[derive(serde::Serialize)]
struct RideStatusDiagnosticSample {
    committed_at_unix_us: Option<u64>,
    event_at_unix_us: Option<u64>,
    ride_id: String,
    chair_id: String,
    status: String,
    pool_size_before: u64,
    pool_idle_before: u64,
    pool_in_use_before: u64,
    pool_acquire_us: u64,
    transaction_begin_us: u64,
    ride_lock_us: u64,
    status_write_us: u64,
    commit_us: u64,
    total_us: u64,
    outcome: &'static str,
    terminal_phase: &'static str,
}

struct RideStatusDiagnostic {
    started_at: Instant,
    checkpoint_at: Instant,
    sample: RideStatusDiagnosticSample,
    emitted: bool,
}

impl RideStatusDiagnostic {
    fn traced(pool: &sqlx::MySqlPool, ride_id: &str, chair_id: &str, status: &str) -> Option<Self> {
        if status != "CARRYING" || !crate::drive_diagnostic::should_trace_ride(ride_id) {
            return None;
        }

        let started_at = Instant::now();
        let pool_size = u64::from(pool.size());
        let pool_idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            sample: RideStatusDiagnosticSample {
                committed_at_unix_us: None,
                event_at_unix_us: None,
                ride_id: ride_id.to_owned(),
                chair_id: chair_id.to_owned(),
                status: status.to_owned(),
                pool_size_before: pool_size,
                pool_idle_before: pool_idle,
                pool_in_use_before: pool_size.saturating_sub(pool_idle),
                pool_acquire_us: 0,
                transaction_begin_us: 0,
                ride_lock_us: 0,
                status_write_us: 0,
                commit_us: 0,
                total_us: 0,
                outcome: "error_or_cancelled",
                terminal_phase: "pool_acquire",
            },
            emitted: false,
        })
    }

    fn elapsed_since_checkpoint_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = crate::drive_diagnostic::duration_us(now.duration_since(self.checkpoint_at));
        self.checkpoint_at = now;
        elapsed
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        self.sample.event_at_unix_us = Some(crate::drive_diagnostic::unix_time_us());
        self.sample.total_us = crate::drive_diagnostic::duration_us(self.started_at.elapsed());
        crate::drive_diagnostic::emit("RIDE_STATUS_DIAGNOSTIC", &self.sample);
    }

    fn emit_success(mut self) {
        self.sample.outcome = "success";
        self.sample.terminal_phase = "complete";
        self.emit_record();
    }
}

impl Drop for RideStatusDiagnostic {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_record();
        }
    }
}

async fn chair_post_ride_status(
    State(AppState {
        pool,
        notification_cache,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(chair): axum::Extension<Chair>,
    Path((ride_id,)): Path<(String,)>,
    axum::Json(req): axum::Json<PostChairRidesRideIDStatusRequest>,
) -> Result<StatusCode, Error> {
    let mut diagnostic = RideStatusDiagnostic::traced(&pool, &ride_id, &chair.id, &req.status);
    let admission_guard = general_db_admission
        .acquire("chair_post_ride_status", &pool)
        .await;
    let mut connection = pool.acquire().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.pool_acquire_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "transaction_begin";
    }
    let mut tx = connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.transaction_begin_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "ride_lock";
    }

    let Some(ride): Option<Ride> = sqlx::query_as("SELECT * FROM rides WHERE id = ? FOR UPDATE")
        .bind(ride_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Err(Error::NotFound("rides not found"));
    };
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.ride_lock_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "status_write";
    }

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
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.status_write_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "commit";
    }

    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.commit_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.committed_at_unix_us = Some(crate::drive_diagnostic::unix_time_us());
        diagnostic.sample.terminal_phase = "cache_invalidation";
    }
    drop(connection);
    drop(admission_guard);
    notification_cache.invalidate_app(&ride.user_id);
    notification_cache.invalidate_chair(&chair.id);
    if let Some(diagnostic) = diagnostic {
        diagnostic.emit_success();
    }

    Ok(StatusCode::NO_CONTENT)
}
