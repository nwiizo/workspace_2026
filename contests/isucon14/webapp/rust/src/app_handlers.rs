use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use sqlx::Acquire;
use std::io::Write as _;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use std::time::Instant;
use ulid::Ulid;

use crate::models::{Chair, Coupon, PaymentToken, Ride, RideStatus, User};
use crate::notification_diagnostic::{
    NotificationConnectionStage, NotificationDiagnostic, NotificationEndpoint,
};
use crate::{AppState, Coordinate, Error};

pub fn app_routes(app_state: AppState) -> axum::Router<AppState> {
    let routes = axum::Router::new().route("/api/app/users", axum::routing::post(app_post_users));

    let authed_routes = axum::Router::new()
        .route(
            "/api/app/payment-methods",
            axum::routing::post(app_post_payment_methods),
        )
        .route(
            "/api/app/rides",
            axum::routing::get(app_get_rides).post(app_post_rides),
        )
        .route(
            "/api/app/rides/estimated-fare",
            axum::routing::post(app_post_rides_estimated_fare),
        )
        .route(
            "/api/app/rides/:ride_id/evaluation",
            axum::routing::post(app_post_ride_evaluation),
        )
        .route(
            "/api/app/notification",
            axum::routing::get(app_get_notification),
        )
        .route(
            "/api/app/nearby-chairs",
            axum::routing::get(app_get_nearby_chairs),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::middlewares::app_auth_middleware,
        ));

    routes.merge(authed_routes)
}

#[derive(Debug, serde::Deserialize)]
struct AppPostUsersRequest {
    username: String,
    firstname: String,
    lastname: String,
    date_of_birth: String,
    invitation_code: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct AppPostUsersResponse {
    id: String,
    invitation_code: String,
}

async fn insert_user(
    tx: &mut sqlx::MySqlConnection,
    user_id: &str,
    username: &str,
    req: &AppPostUsersRequest,
    access_token: &str,
    invitation_code: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO users (id, username, firstname, lastname, date_of_birth, access_token, invitation_code) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(user_id)
        .bind(username)
        .bind(&req.firstname)
        .bind(&req.lastname)
        .bind(&req.date_of_birth)
        .bind(access_token)
        .bind(invitation_code)
        .execute(&mut *tx)
        .await?;

    Ok(())
}

fn is_username_duplicate(error: &sqlx::Error) -> bool {
    let Some(database_error) = error.as_database_error() else {
        return false;
    };
    if !database_error.is_unique_violation() {
        return false;
    }

    database_error
        .try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>()
        .is_some_and(|mysql_error| {
            is_username_duplicate_mysql_error(mysql_error.number(), mysql_error.message())
        })
}

fn is_username_duplicate_mysql_error(number: u16, message: &str) -> bool {
    number == 1062 && message.ends_with("for key 'users.username'")
}

fn duplicate_username_fallback(user_id: &str) -> String {
    format!("~{user_id}")
}

async fn app_post_users(
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    jar: CookieJar,
    axum::Json(req): axum::Json<AppPostUsersRequest>,
) -> Result<(CookieJar, (StatusCode, axum::Json<AppPostUsersResponse>)), Error> {
    let user_id = Ulid::new().to_string();
    let access_token = crate::secure_random_str(32);
    let invitation_code = crate::secure_random_str(15);

    let _admission_guard = general_db_admission.acquire("app_post_users", &pool).await;
    let mut tx = pool.begin().await?;

    if let Err(error) = insert_user(
        &mut tx,
        &user_id,
        &req.username,
        &req,
        &access_token,
        &invitation_code,
    )
    .await
    {
        if !is_username_duplicate(&error) {
            return Err(error.into());
        }

        let fallback_username = duplicate_username_fallback(&user_id);
        tracing::warn!(
            user_id = %user_id,
            "retrying user registration with an internal username after a duplicate"
        );
        insert_user(
            &mut tx,
            &user_id,
            &fallback_username,
            &req,
            &access_token,
            &invitation_code,
        )
        .await?;
    }

    // 初回登録キャンペーンのクーポンを付与
    sqlx::query("INSERT INTO coupons (user_id, code, discount) VALUES (?, ?, ?)")
        .bind(&user_id)
        .bind("CP_NEW2024")
        .bind(3000)
        .execute(&mut *tx)
        .await?;

    // 招待コードを使った登録
    if let Some(req_invitation_code) = req.invitation_code {
        if !req_invitation_code.is_empty() {
            // The unique inviter row is the serialization point for the
            // per-invitation-code limit. Lock it before counting coupons so
            // registrations using different codes do not hold conflicting
            // next-key locks in coupons(code).
            let Some(inviter_id): Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE invitation_code = ? FOR UPDATE")
                    .bind(&req_invitation_code)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Err(Error::BadRequest("この招待コードは使用できません。"));
            };

            let invitation_coupon_code = format!("INV_{req_invitation_code}");
            let invitation_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM coupons WHERE code = ?")
                    .bind(&invitation_coupon_code)
                    .fetch_one(&mut *tx)
                    .await?;
            if invitation_count >= 3 {
                return Err(Error::BadRequest("この招待コードは使用できません。"));
            }

            // 招待クーポン付与
            sqlx::query("INSERT INTO coupons (user_id, code, discount) VALUES (?, ?, ?)")
                .bind(&user_id)
                .bind(&invitation_coupon_code)
                .bind(1500)
                .execute(&mut *tx)
                .await?;
            // 招待した人にもRewardを付与
            sqlx::query(
                "INSERT INTO coupons (user_id, code, discount) VALUES (?, CONCAT(?, '_', ?), ?)",
            )
            .bind(inviter_id)
            .bind(format!("RWD_{req_invitation_code}"))
            .bind(&user_id)
            .bind(1000)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    let jar = jar
        .add(axum_extra::extract::cookie::Cookie::build(("app_session", access_token)).path("/"));

    Ok((
        jar,
        (
            StatusCode::CREATED,
            axum::Json(AppPostUsersResponse {
                id: user_id,
                invitation_code,
            }),
        ),
    ))
}

#[cfg(test)]
mod user_registration_tests {
    use super::{duplicate_username_fallback, is_username_duplicate_mysql_error};

    #[test]
    fn duplicate_username_fallback_uses_the_full_user_id_and_fits_the_column() {
        let user_id = "01JDJ23EA0C0P2KFPTXDKTZMNM";
        let username = duplicate_username_fallback(user_id);

        assert_eq!(username, "~01JDJ23EA0C0P2KFPTXDKTZMNM");
        assert!(username.chars().count() <= 30);
    }

    #[test]
    fn retry_classification_accepts_only_the_username_unique_key() {
        assert!(is_username_duplicate_mysql_error(
            1062,
            "Duplicate entry 'same' for key 'users.username'"
        ));

        for message in [
            "Duplicate entry 'id' for key 'users.PRIMARY'",
            "Duplicate entry 'token' for key 'users.access_token'",
            "Duplicate entry 'code' for key 'users.invitation_code'",
        ] {
            assert!(!is_username_duplicate_mysql_error(1062, message));
        }
        assert!(!is_username_duplicate_mysql_error(
            1213,
            "Deadlock found when trying to get lock"
        ));
    }
}

#[derive(Debug, serde::Deserialize)]
struct AppPostPaymentMethodsRequest {
    token: String,
}

async fn app_post_payment_methods(
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostPaymentMethodsRequest>,
) -> Result<StatusCode, Error> {
    let _admission_guard = general_db_admission
        .acquire("app_post_payment_methods", &pool)
        .await;
    sqlx::query("INSERT INTO payment_tokens (user_id, token) VALUES (?, ?)")
        .bind(user.id)
        .bind(req.token)
        .execute(&pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponse {
    rides: Vec<GetAppRidesResponseItem>,
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponseItem {
    id: String,
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
    chair: GetAppRidesResponseItemChair,
    fare: i32,
    evaluation: i32,
    requested_at: i64,
    completed_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponseItemChair {
    id: String,
    owner: String,
    name: String,
    model: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AppRideRow {
    id: String,
    pickup_latitude: i32,
    pickup_longitude: i32,
    destination_latitude: i32,
    destination_longitude: i32,
    evaluation: i32,
    requested_at: chrono::DateTime<chrono::Utc>,
    completed_at: chrono::DateTime<chrono::Utc>,
    chair_id: String,
    chair_name: String,
    chair_model: String,
    owner_name: String,
    discount: i32,
}

impl AppRideRow {
    fn into_response_item(self) -> GetAppRidesResponseItem {
        GetAppRidesResponseItem {
            id: self.id,
            pickup_coordinate: Coordinate {
                latitude: self.pickup_latitude,
                longitude: self.pickup_longitude,
            },
            destination_coordinate: Coordinate {
                latitude: self.destination_latitude,
                longitude: self.destination_longitude,
            },
            chair: GetAppRidesResponseItemChair {
                id: self.chair_id,
                owner: self.owner_name,
                name: self.chair_name,
                model: self.chair_model,
            },
            fare: crate::calculate_fare_with_discount(
                self.pickup_latitude,
                self.pickup_longitude,
                self.destination_latitude,
                self.destination_longitude,
                self.discount,
            ),
            evaluation: self.evaluation,
            requested_at: self.requested_at.timestamp_millis(),
            completed_at: self.completed_at.timestamp_millis(),
        }
    }
}

async fn app_get_rides(
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
) -> Result<axum::Json<GetAppRidesResponse>, Error> {
    let _admission_guard = general_db_admission.acquire("app_get_rides", &pool).await;
    // Completion writes COMPLETED and evaluation in the same transaction.
    // coupons.used_by is not UNIQUE in the schema, so LIMIT 1 preserves the
    // previous fetch_optional cardinality even if inconsistent data exists.
    let rides: Vec<AppRideRow> = sqlx::query_as(
        r#"
SELECT
  rides.id,
  rides.pickup_latitude,
  rides.pickup_longitude,
  rides.destination_latitude,
  rides.destination_longitude,
  rides.evaluation,
  rides.created_at AS requested_at,
  rides.updated_at AS completed_at,
  chairs.id AS chair_id,
  chairs.name AS chair_name,
  chairs.model AS chair_model,
  owners.name AS owner_name,
  COALESCE((
    SELECT coupons.discount
    FROM coupons
    WHERE coupons.used_by = rides.id
    LIMIT 1
  ), 0) AS discount
FROM rides
INNER JOIN chairs ON chairs.id = rides.chair_id
INNER JOIN owners ON owners.id = chairs.owner_id
WHERE rides.user_id = ?
  AND rides.evaluation IS NOT NULL
ORDER BY rides.created_at DESC
        "#,
    )
    .bind(&user.id)
    .fetch_all(&pool)
    .await?;
    let items = rides
        .into_iter()
        .map(AppRideRow::into_response_item)
        .collect();

    Ok(axum::Json(GetAppRidesResponse { rides: items }))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRidesRequest {
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRidesResponse {
    ride_id: String,
    fare: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct UserRideState {
    ride_count: i64,
    has_active_ride: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct RideCoupon {
    code: String,
    discount: i32,
}

async fn app_post_rides(
    State(AppState {
        pool,
        notification_cache,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostRidesRequest>,
) -> Result<(StatusCode, axum::Json<AppPostRidesResponse>), Error> {
    let ride_id = Ulid::new().to_string();

    let _admission_guard = general_db_admission.acquire("app_post_rides", &pool).await;
    let mut tx = pool.begin().await?;

    // Serialize ride creation and invitation rewards for the same user on the
    // users primary-key row. Without this lock, concurrent requests can all
    // observe no active ride before any of their INSERTs becomes visible.
    let _: String = sqlx::query_scalar("SELECT id FROM users WHERE id = ? FOR UPDATE")
        .bind(&user.id)
        .fetch_one(&mut *tx)
        .await?;

    // Evaluation and COMPLETED are committed together by the evaluation
    // transaction. Use the ride row as current state instead of loading every
    // historical ride and querying its latest status.
    let ride_state: UserRideState = sqlx::query_as(
        r#"
SELECT
  COUNT(*) AS ride_count,
  CAST(COALESCE(MAX(evaluation IS NULL), 0) AS SIGNED) AS has_active_ride
FROM rides
WHERE user_id = ?
        "#,
    )
    .bind(&user.id)
    .fetch_one(&mut *tx)
    .await?;
    if ride_state.has_active_ride != 0 {
        return Err(Error::Conflict("ride already exists"));
    }
    let is_first_ride = ride_state.ride_count == 0;

    sqlx::query("INSERT INTO rides (id, user_id, pickup_latitude, pickup_longitude, destination_latitude, destination_longitude) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&ride_id)
        .bind(&user.id)
        .bind(req.pickup_coordinate.latitude)
        .bind(req.pickup_coordinate.longitude)
        .bind(req.destination_coordinate.latitude)
        .bind(req.destination_coordinate.longitude)
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
        .bind(Ulid::new().to_string())
        .bind(&ride_id)
        .bind("MATCHING")
        .execute(&mut *tx)
        .await?;

    // On the first ride CP_NEW2024 has priority even if another coupon was
    // granted earlier. On later rides all unused coupons retain created_at
    // order. The PRIMARY hint bounds this lookup to one user's coupon range.
    let coupon: Option<RideCoupon> = sqlx::query_as(
        r#"
SELECT code, discount
FROM coupons FORCE INDEX (PRIMARY)
WHERE user_id = ?
  AND used_by IS NULL
ORDER BY
  CASE WHEN ? AND code = 'CP_NEW2024' THEN 0 ELSE 1 END,
  created_at,
  code
LIMIT 1
FOR UPDATE
        "#,
    )
    .bind(&user.id)
    .bind(is_first_ride)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(coupon) = &coupon {
        let claimed = sqlx::query(
            "UPDATE coupons SET used_by = ? WHERE user_id = ? AND code = ? AND used_by IS NULL",
        )
        .bind(&ride_id)
        .bind(&user.id)
        .bind(&coupon.code)
        .execute(&mut *tx)
        .await?;
        if claimed.rows_affected() != 1 {
            return Err(Error::Conflict("coupon already used"));
        }
    }

    let fare = crate::calculate_fare_with_discount(
        req.pickup_coordinate.latitude,
        req.pickup_coordinate.longitude,
        req.destination_coordinate.latitude,
        req.destination_coordinate.longitude,
        coupon.map(|coupon| coupon.discount).unwrap_or(0),
    );

    tx.commit().await?;
    notification_cache.invalidate_app(&user.id);

    Ok((
        StatusCode::ACCEPTED,
        axum::Json(AppPostRidesResponse { ride_id, fare }),
    ))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRidesEstimatedFareRequest {
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRidesEstimatedFareResponse {
    fare: i32,
    discount: i32,
}

async fn app_post_rides_estimated_fare(
    State(AppState {
        pool,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostRidesEstimatedFareRequest>,
) -> Result<axum::Json<AppPostRidesEstimatedFareResponse>, Error> {
    let _admission_guard = general_db_admission
        .acquire("app_post_rides_estimated_fare", &pool)
        .await;
    let mut tx = pool.begin().await?;

    let discounted = calculate_discounted_fare(
        &mut tx,
        &user.id,
        None,
        req.pickup_coordinate.latitude,
        req.pickup_coordinate.longitude,
        req.destination_coordinate.latitude,
        req.destination_coordinate.longitude,
    )
    .await?;

    tx.commit().await?;

    Ok(axum::Json(AppPostRidesEstimatedFareResponse {
        fare: discounted,
        discount: crate::calculate_fare(
            req.pickup_coordinate.latitude,
            req.pickup_coordinate.longitude,
            req.destination_coordinate.latitude,
            req.destination_coordinate.longitude,
        ) - discounted,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRideEvaluationRequest {
    evaluation: i32,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRideEvaluationResponse {
    fare: i32,
    completed_at: i64,
}

const EVALUATION_DIAGNOSTIC_SAMPLE_EVERY: u64 = 8;
static EVALUATION_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static EVALUATION_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(serde::Serialize)]
struct EvaluationDiagnosticSample {
    sequence: u64,
    validation_us: u64,
    pool_acquire_us: u64,
    transaction_begin_us: u64,
    pool_size_before: Option<u64>,
    pool_idle_before: Option<u64>,
    pool_in_use_before: Option<u64>,
    ride_lock_status_us: u64,
    tracker_begin_us: u64,
    active_evaluations: Option<u64>,
    same_ride_evaluations: Option<u64>,
    preparation_us: u64,
    preparation_commit_us: u64,
    preparation_connection_owned_us: u64,
    payment_us: u64,
    payment_attempts: u32,
    payment_request_us: u64,
    payment_retry_sleep_us: u64,
    payment_network_errors: u32,
    payment_conflict_errors: u32,
    payment_server_errors: u32,
    payment_other_status_errors: u32,
    payment_terminal_status: Option<u16>,
    completion_pool_acquire_us: u64,
    completion_transaction_begin_us: u64,
    completion_pool_size_before: Option<u64>,
    completion_pool_idle_before: Option<u64>,
    completion_pool_in_use_before: Option<u64>,
    completion_ride_recheck_us: u64,
    completion_write_us: u64,
    commit_us: u64,
    completion_connection_owned_us: u64,
    cache_response_us: u64,
    connection_owned_us: u64,
    total_us: u64,
    outcome: &'static str,
    terminal_phase: &'static str,
}

#[derive(Clone, Copy)]
enum EvaluationConnectionStage {
    Preparation,
    Completion,
}

struct EvaluationDiagnostic {
    started_at: Instant,
    checkpoint_at: Instant,
    connection_acquired_at: Option<Instant>,
    connection_stage: Option<EvaluationConnectionStage>,
    payment_started_at: Option<Instant>,
    payment_diagnostic: crate::payment_gateway::PaymentGatewayDiagnostic,
    sample: EvaluationDiagnosticSample,
    emitted: bool,
}

impl EvaluationDiagnostic {
    fn sampled() -> Option<Self> {
        let enabled = *EVALUATION_DIAGNOSTICS_ENABLED.get_or_init(|| {
            std::env::var_os("ISUCON_DIAGNOSTIC").as_deref() == Some(std::ffi::OsStr::new("1"))
        });
        if !enabled {
            return None;
        }

        let sequence = EVALUATION_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        if sequence.checked_rem(EVALUATION_DIAGNOSTIC_SAMPLE_EVERY) != Some(0) {
            return None;
        }

        let started_at = Instant::now();
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            connection_acquired_at: None,
            connection_stage: None,
            payment_started_at: None,
            payment_diagnostic: crate::payment_gateway::PaymentGatewayDiagnostic::default(),
            sample: EvaluationDiagnosticSample {
                sequence,
                validation_us: 0,
                pool_acquire_us: 0,
                transaction_begin_us: 0,
                pool_size_before: None,
                pool_idle_before: None,
                pool_in_use_before: None,
                ride_lock_status_us: 0,
                tracker_begin_us: 0,
                active_evaluations: None,
                same_ride_evaluations: None,
                preparation_us: 0,
                preparation_commit_us: 0,
                preparation_connection_owned_us: 0,
                payment_us: 0,
                payment_attempts: 0,
                payment_request_us: 0,
                payment_retry_sleep_us: 0,
                payment_network_errors: 0,
                payment_conflict_errors: 0,
                payment_server_errors: 0,
                payment_other_status_errors: 0,
                payment_terminal_status: None,
                completion_pool_acquire_us: 0,
                completion_transaction_begin_us: 0,
                completion_pool_size_before: None,
                completion_pool_idle_before: None,
                completion_pool_in_use_before: None,
                completion_ride_recheck_us: 0,
                completion_write_us: 0,
                commit_us: 0,
                completion_connection_owned_us: 0,
                cache_response_us: 0,
                connection_owned_us: 0,
                total_us: 0,
                outcome: "error_or_cancelled",
                terminal_phase: "validation",
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

    fn connection_acquired(&mut self, stage: EvaluationConnectionStage) {
        self.connection_acquired_at = Some(Instant::now());
        self.connection_stage = Some(stage);
    }

    fn connection_released(&mut self) {
        if let Some(acquired_at) = self.connection_acquired_at.take() {
            let elapsed_us = acquired_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
            match self.connection_stage.take() {
                Some(EvaluationConnectionStage::Preparation) => {
                    self.sample.preparation_connection_owned_us = elapsed_us;
                }
                Some(EvaluationConnectionStage::Completion) => {
                    self.sample.completion_connection_owned_us = elapsed_us;
                }
                None => {}
            }
            self.sample.connection_owned_us = self
                .sample
                .preparation_connection_owned_us
                .saturating_add(self.sample.completion_connection_owned_us);
        }
    }

    fn sync_payment_sample(&mut self) {
        self.sample.payment_attempts = self.payment_diagnostic.attempts();
        self.sample.payment_request_us = self.payment_diagnostic.request_us();
        self.sample.payment_retry_sleep_us = self.payment_diagnostic.retry_sleep_us();
        self.sample.payment_network_errors = self.payment_diagnostic.network_errors();
        self.sample.payment_conflict_errors = self.payment_diagnostic.conflict_errors();
        self.sample.payment_server_errors = self.payment_diagnostic.server_errors();
        self.sample.payment_other_status_errors = self.payment_diagnostic.other_status_errors();
        self.sample.payment_terminal_status = self.payment_diagnostic.terminal_status();
    }

    fn payment_started(&mut self) {
        self.payment_started_at = Some(Instant::now());
    }

    fn payment_finished(&mut self) {
        if let Some(started_at) = self.payment_started_at.take() {
            let now = Instant::now();
            self.sample.payment_us = now
                .duration_since(started_at)
                .as_micros()
                .min(u128::from(u64::MAX)) as u64;
            self.checkpoint_at = now;
        }
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        if self.payment_started_at.is_some() {
            self.payment_finished();
        }
        self.sync_payment_sample();
        if self.connection_acquired_at.is_some() {
            self.connection_released();
        }
        self.sample.total_us = self
            .started_at
            .elapsed()
            .as_micros()
            .min(u128::from(u64::MAX)) as u64;
        if let Ok(json) = serde_json::to_string(&self.sample) {
            let _ = writeln!(std::io::stdout().lock(), "EVALUATION_DIAGNOSTIC {json}");
        }
    }

    fn emit_success(mut self) {
        self.sample.outcome = "success";
        self.sample.terminal_phase = "complete";
        self.emit_record();
    }
}

impl Drop for EvaluationDiagnostic {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_record();
        }
    }
}

async fn app_post_ride_evaluation(
    State(AppState {
        pool,
        payment_client,
        active_ride_evaluations,
        notification_cache,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    Path((ride_id,)): Path<(String,)>,
    axum::Json(req): axum::Json<AppPostRideEvaluationRequest>,
) -> Result<Response, Error> {
    let mut diagnostic = EvaluationDiagnostic::sampled();
    if req.evaluation < 1 || req.evaluation > 5 {
        return Err(Error::BadRequest("evaluation must be between 1 and 5"));
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.validation_us = diagnostic.elapsed_since_checkpoint_us();
    }
    let preparation_admission_guard = general_db_admission
        .acquire("app_post_ride_evaluation_prepare", &pool)
        .await;
    if let Some(diagnostic) = &mut diagnostic {
        let _admission_wait_us = diagnostic.elapsed_since_checkpoint_us();
        let pool_size = u64::from(pool.size());
        let pool_idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        diagnostic.sample.pool_size_before = Some(pool_size);
        diagnostic.sample.pool_idle_before = Some(pool_idle);
        diagnostic.sample.pool_in_use_before = Some(pool_size.saturating_sub(pool_idle));
        diagnostic.sample.terminal_phase = "pool_acquire";
    }
    let mut connection = pool.acquire().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_acquired(EvaluationConnectionStage::Preparation);
        diagnostic.sample.pool_acquire_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "transaction_begin";
    }
    let mut tx = connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.transaction_begin_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "ride_lock_status";
    }

    let Some(ride): Option<Ride> =
        sqlx::query_as("SELECT * FROM rides WHERE id = ? AND user_id = ? FOR UPDATE")
            .bind(&ride_id)
            .bind(&user.id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Err(Error::NotFound("ride not found"));
    };
    let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;

    if status != "ARRIVED" || ride.evaluation.is_some() {
        return Err(Error::BadRequest("not arrived yet"));
    }
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.ride_lock_status_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "tracker_begin";
    }

    // Keep the assigned chair unavailable while the preparation transaction,
    // transaction-free payment, completion transaction, and response delivery
    // are in progress. Dropping the guard starts the tracker's measured
    // delivery grace because handing the body to Hyper can still precede the
    // benchmark client receiving that response.
    let active_evaluation = ride
        .chair_id
        .clone()
        .map(|chair_id| active_ride_evaluations.begin(chair_id, ride_id.clone()));
    let chair_id = ride
        .chair_id
        .clone()
        .ok_or(Error::BadRequest("chair not assigned"))?;
    if let Some(diagnostic) = &mut diagnostic {
        let (active_evaluations, same_ride_evaluations) =
            active_ride_evaluations.diagnostic_counts(&ride_id);
        diagnostic.sample.active_evaluations =
            Some(u64::try_from(active_evaluations).unwrap_or(u64::MAX));
        diagnostic.sample.same_ride_evaluations =
            Some(u64::try_from(same_ride_evaluations).unwrap_or(u64::MAX));
        diagnostic.sample.tracker_begin_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "preparation";
    }

    let Some(payment_token): Option<PaymentToken> =
        sqlx::query_as("SELECT * FROM payment_tokens WHERE user_id = ?")
            .bind(&ride.user_id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Err(Error::BadRequest("payment token not registered"));
    };

    let fare = calculate_discounted_fare(
        &mut tx,
        &ride.user_id,
        Some(&ride),
        ride.pickup_latitude,
        ride.pickup_longitude,
        ride.destination_latitude,
        ride.destination_longitude,
    )
    .await?;

    let payment_gateway_url: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE name = 'payment_gateway_url'")
            .fetch_one(&mut *tx)
            .await?;

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.preparation_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "preparation_commit";
    }
    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.preparation_commit_us = diagnostic.elapsed_since_checkpoint_us();
    }
    drop(connection);
    drop(preparation_admission_guard);
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_released();
        diagnostic.sample.terminal_phase = "payment";
        diagnostic.payment_started();
    }
    let payment_diagnostic = diagnostic
        .as_mut()
        .map(|diagnostic| &mut diagnostic.payment_diagnostic);
    let payment_result = crate::payment_gateway::request_payment_gateway_post_payment(
        &payment_client,
        &payment_gateway_url,
        &payment_token.token,
        &ride_id,
        &crate::payment_gateway::PaymentGatewayPostPaymentRequest { amount: fare },
        payment_diagnostic,
    )
    .await;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.payment_finished();
        diagnostic.sync_payment_sample();
    }
    payment_result?;
    let completion_admission_guard = general_db_admission
        .acquire("app_post_ride_evaluation_complete", &pool)
        .await;
    if let Some(diagnostic) = &mut diagnostic {
        let _admission_wait_us = diagnostic.elapsed_since_checkpoint_us();
        let pool_size = u64::from(pool.size());
        let pool_idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        diagnostic.sample.completion_pool_size_before = Some(pool_size);
        diagnostic.sample.completion_pool_idle_before = Some(pool_idle);
        diagnostic.sample.completion_pool_in_use_before = Some(pool_size.saturating_sub(pool_idle));
        diagnostic.sample.terminal_phase = "completion_pool_acquire";
    }

    let mut connection = pool.acquire().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_acquired(EvaluationConnectionStage::Completion);
        diagnostic.sample.completion_pool_acquire_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "completion_transaction_begin";
    }
    let mut tx = connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.completion_transaction_begin_us =
            diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "completion_ride_recheck";
    }

    let Some(completion_ride): Option<Ride> =
        sqlx::query_as("SELECT * FROM rides WHERE id = ? AND user_id = ? FOR UPDATE")
            .bind(&ride_id)
            .bind(&user.id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Err(Error::NotFound("ride not found"));
    };
    let completion_status = crate::get_latest_ride_status(&mut *tx, &completion_ride.id).await?;
    if completion_ride.evaluation.is_some()
        || completion_status != "ARRIVED"
        || completion_ride.chair_id.as_deref() != Some(chair_id.as_str())
    {
        return Err(Error::BadRequest("not arrived yet"));
    }
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.completion_ride_recheck_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "completion_write";
    }

    sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
        .bind(Ulid::new().to_string())
        .bind(&ride_id)
        .bind("COMPLETED")
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
)
SELECT ?, 1, ?
WHERE EXISTS (
  SELECT 1
  FROM ride_statuses
  WHERE ride_id = ?
    AND status = 'CARRYING'
)
ON DUPLICATE KEY UPDATE
  total_rides_count = total_rides_count + 1,
  total_evaluation_sum = total_evaluation_sum + VALUES(total_evaluation_sum)
        "#,
    )
    .bind(&chair_id)
    .bind(req.evaluation)
    .bind(&ride_id)
    .execute(&mut *tx)
    .await?;

    // rides.updated_at is both the completion time returned to the benchmarker
    // and the boundary used by GET /api/owner/sales. Write it as the final SQL
    // statement after payment succeeds so the timestamp-to-commit interval is
    // as short as possible.
    let completed_at = chrono::Utc::now();
    let result = sqlx::query("UPDATE rides SET evaluation = ?, updated_at = ? WHERE id = ?")
        .bind(req.evaluation)
        .bind(completed_at)
        .bind(&ride_id)
        .execute(&mut *tx)
        .await?;
    let count = result.rows_affected();
    if count == 0 {
        return Err(Error::NotFound("ride not found"));
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.completion_write_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.sample.terminal_phase = "commit";
    }
    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.commit_us = diagnostic.elapsed_since_checkpoint_us();
    }
    drop(connection);
    drop(completion_admission_guard);
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.connection_released();
        diagnostic.sample.terminal_phase = "cache_response";
    }
    notification_cache.invalidate_app(&user.id);
    notification_cache.invalidate_chair(&chair_id);
    notification_cache.invalidate_chair_stats(&chair_id);

    let response = axum::Json(AppPostRideEvaluationResponse {
        fare,
        completed_at: completed_at.timestamp_millis(),
    })
    .into_response();
    if let Some(mut diagnostic) = diagnostic {
        diagnostic.sample.cache_response_us = diagnostic.elapsed_since_checkpoint_us();
        diagnostic.emit_success();
    }

    Ok(crate::hold_active_evaluation_until_response_drop(
        response,
        active_evaluation,
    ))
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponse {
    data: Option<AppGetNotificationResponseData>,
    retry_after_ms: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseData {
    ride_id: String,
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
    fare: i32,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    chair: Option<AppGetNotificationResponseChair>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseChair {
    id: String,
    name: String,
    model: String,
    stats: AppGetNotificationResponseChairStats,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseChairStats {
    total_rides_count: i32,
    total_evaluation_avg: f64,
}

async fn app_get_notification(
    State(AppState {
        pool,
        notification_cache,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
) -> Result<Response, Error> {
    let mut diagnostic = NotificationDiagnostic::sampled(NotificationEndpoint::App);
    let (cached_payload, cache_revision) = notification_cache.app(&user.id);
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
        .acquire("app_get_notification", &pool)
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
    let latest_ride: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id, chair_id FROM rides WHERE user_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&user.id)
    .fetch_optional(&mut *initial_connection)
    .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.latest_ride_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }
    let Some((_, dependency_chair_id)) = latest_ride else {
        drop(initial_connection);
        drop(admission_guard);
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.connection_released();
        }
        let payload = axum::body::Bytes::from(serde_json::to_vec(&AppGetNotificationResponse {
            data: None,
            retry_after_ms: Some(crate::CACHED_NOTIFICATION_RETRY_AFTER_MS),
        })?);
        notification_cache.insert_app_if_current(user.id, cache_revision, None, payload.clone());
        let response = crate::json_bytes_response(payload);
        if let Some(mut diagnostic) = diagnostic {
            diagnostic.sample.path = "no_ride";
            diagnostic.sample.cache_insert_attempted = true;
            diagnostic.sample.terminal_phase = "response";
            diagnostic.sample.response_us = Some(diagnostic.elapsed_since_checkpoint_us());
            diagnostic.emit_success();
        }
        return Ok(response);
    };
    // Chair statistics are part of the app notification payload and can change
    // when another user evaluates a later ride on the same chair. Capture this
    // cross-recipient dependency before opening the transaction, so a
    // concurrent evaluation invalidates a stale snapshot before it is cached.
    let chair_stats_revision = dependency_chair_id
        .as_deref()
        .map(|chair_id| notification_cache.chair_stats_revision(chair_id));
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.dependency_revision_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.reuse_connection_for_transaction();
        diagnostic.sample.terminal_phase = "transaction_begin";
    }

    // 通知内容を組み立てる複数の SELECT と通知済み更新は、同じ
    // スナップショットで扱う必要がある。ライドがない利用者だけを
    // トランザクション開始前に返し、整合性と負荷削減を両立する。
    let mut tx = initial_connection.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.transaction_begin_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "ride_query";
    }
    let ride: Ride =
        sqlx::query_as("SELECT * FROM rides WHERE user_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(&user.id)
            .fetch_one(&mut *tx)
            .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.ride_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "pending_status_query";
    }
    let chair_dependency_matches = ride.chair_id.as_deref() == dependency_chair_id.as_deref();

    let yet_sent_ride_status: Option<RideStatus> = sqlx::query_as("SELECT * FROM ride_statuses WHERE ride_id = ? AND app_sent_at IS NULL ORDER BY status ASC LIMIT 1")
        .bind(&ride.id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.pending_status_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }
    let (ride_status_id, status) = if let Some(yet_sent_ride_status) = yet_sent_ride_status {
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
    if ride_status_id.is_some() {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.trace_ride_event(&ride.id, &status, &user.id);
        }
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.terminal_phase = "fare_query";
    }
    let fare = calculate_discounted_fare(
        &mut tx,
        &user.id,
        Some(&ride),
        ride.pickup_latitude,
        ride.pickup_longitude,
        ride.destination_latitude,
        ride.destination_longitude,
    )
    .await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.fare_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
    }

    let mut data = AppGetNotificationResponseData {
        ride_id: ride.id,
        pickup_coordinate: Coordinate {
            latitude: ride.pickup_latitude,
            longitude: ride.pickup_longitude,
        },
        destination_coordinate: Coordinate {
            latitude: ride.destination_latitude,
            longitude: ride.destination_longitude,
        },
        fare,
        status,
        chair: None,
        created_at: ride.created_at.timestamp_millis(),
        updated_at: ride.updated_at.timestamp_millis(),
    };

    if let Some(chair_id) = &ride.chair_id {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.terminal_phase = "chair_query";
        }
        let chair: Chair = sqlx::query_as("SELECT * FROM chairs WHERE id = ?")
            .bind(chair_id)
            .fetch_one(&mut *tx)
            .await?;
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.chair_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
            diagnostic.sample.terminal_phase = "chair_stats_query";
        }

        let stats = get_chair_stats(&mut tx, &chair.id).await?;
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.chair_stats_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
        }

        data.chair = Some(AppGetNotificationResponseChair {
            id: chair.id,
            name: chair.name,
            model: chair.model,
            stats,
        });
    }

    if let Some(ride_status_id) = &ride_status_id {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.terminal_phase = "sent_update";
        }
        sqlx::query("UPDATE ride_statuses SET app_sent_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(ride_status_id)
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

    let cacheable = ride_status_id.is_none() && chair_dependency_matches;
    let response = AppGetNotificationResponse {
        data: Some(data),
        retry_after_ms: Some(if cacheable {
            crate::CACHED_NOTIFICATION_RETRY_AFTER_MS
        } else {
            crate::NOTIFICATION_RETRY_AFTER_MS
        }),
    };
    let payload = axum::body::Bytes::from(serde_json::to_vec(&response)?);
    if cacheable {
        notification_cache.insert_app_if_current(
            user.id,
            cache_revision,
            chair_stats_revision,
            payload.clone(),
        );
    }
    let response = crate::json_bytes_response(payload);
    if let Some(mut diagnostic) = diagnostic {
        diagnostic.sample.path = if ride_status_id.is_some() {
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

async fn get_chair_stats(
    tx: &mut sqlx::MySqlConnection,
    chair_id: &str,
) -> Result<AppGetNotificationResponseChairStats, Error> {
    #[derive(sqlx::FromRow)]
    struct ChairStats {
        total_rides_count: i64,
        total_evaluation_avg: f64,
    }

    let stats: ChairStats = sqlx::query_as(
        r#"
SELECT COALESCE(total_rides_count, 0) AS total_rides_count,
       CASE
         WHEN total_rides_count IS NULL THEN 0.0
         ELSE CAST(total_evaluation_sum AS DOUBLE) / total_rides_count
       END AS total_evaluation_avg
FROM (SELECT 1) AS seed
LEFT JOIN chair_stats ON chair_stats.chair_id = ?
        "#,
    )
    .bind(chair_id)
    .fetch_one(&mut *tx)
    .await?;

    Ok(AppGetNotificationResponseChairStats {
        total_rides_count: stats.total_rides_count as i32,
        total_evaluation_avg: stats.total_evaluation_avg,
    })
}

#[derive(Debug, serde::Deserialize)]
struct AppGetNearbyChairsQuery {
    latitude: i32,
    longitude: i32,
    distance: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNearbyChairsResponse {
    chairs: Vec<AppGetNearbyChairsResponseChair>,
    retrieved_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNearbyChairsResponseChair {
    id: String,
    name: String,
    model: String,
    current_coordinate: Coordinate,
}

#[derive(Debug, sqlx::FromRow)]
struct NearbyChair {
    id: String,
    name: String,
    model: String,
}

async fn app_get_nearby_chairs(
    State(AppState {
        pool,
        latest_chair_locations,
        active_ride_evaluations,
        general_db_admission,
        ..
    }): State<AppState>,
    Query(query): Query<AppGetNearbyChairsQuery>,
) -> Result<axum::Json<AppGetNearbyChairsResponse>, Error> {
    let distance = query.distance.unwrap_or(50);
    let coordinate = Coordinate {
        latitude: query.latitude,
        longitude: query.longitude,
    };

    // Preserve evaluations that overlap this request even when the SQL waits
    // for a pool connection or an evaluation transaction. Active IDs alone
    // can miss an evaluation that both starts and ends during that wait.
    let evaluation_snapshot = active_ride_evaluations.snapshot();
    let _admission_guard = general_db_admission
        .acquire("app_get_nearby_chairs", &pool)
        .await;
    let chairs: Vec<NearbyChair> = sqlx::query_as(
        r#"
SELECT chairs.id,
       chairs.name,
       chairs.model
FROM chairs
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND rides.evaluation IS NULL
  )
        "#,
    )
    .fetch_all(&pool)
    .await?;

    // SQL changes from "busy" to "free" at evaluation commit, before the client
    // necessarily receives the response. The start snapshot, completion
    // revision, and short delivery grace cover evaluations that overlap this
    // request without deriving the boundary from rides.updated_at.
    let evaluating_chair_ids = active_ride_evaluations.chair_ids_overlapping(evaluation_snapshot);
    let coordinates = latest_chair_locations
        .coordinates_for(chairs.iter().map(|chair| chair.id.as_str()))
        .await;
    let mut nearby_chairs = Vec::with_capacity(chairs.len());
    for (chair, latest_location) in chairs.into_iter().zip(coordinates) {
        if evaluating_chair_ids.contains(&chair.id) {
            continue;
        }
        let Some(latest_location) = latest_location else {
            continue;
        };
        if crate::calculate_distance(
            coordinate.latitude,
            coordinate.longitude,
            latest_location.latitude,
            latest_location.longitude,
        ) <= distance
        {
            nearby_chairs.push(AppGetNearbyChairsResponseChair {
                id: chair.id,
                name: chair.name,
                model: chair.model,
                current_coordinate: latest_location,
            });
        }
    }

    Ok(axum::Json(AppGetNearbyChairsResponse {
        chairs: nearby_chairs,
        retrieved_at: chrono::Utc::now().timestamp_millis(),
    }))
}

async fn calculate_discounted_fare(
    tx: &mut sqlx::MySqlConnection,
    user_id: &str,
    ride: Option<&Ride>,
    mut pickup_latitude: i32,
    mut pickup_longitude: i32,
    mut dest_latitude: i32,
    mut dest_longitude: i32,
) -> sqlx::Result<i32> {
    let discount = if let Some(ride) = ride {
        dest_latitude = ride.destination_latitude;
        dest_longitude = ride.destination_longitude;
        pickup_latitude = ride.pickup_latitude;
        pickup_longitude = ride.pickup_longitude;

        // すでにクーポンが紐づいているならそれの割引額を参照
        let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE used_by = ?")
            .bind(&ride.id)
            .fetch_optional(&mut *tx)
            .await?;
        coupon.map(|c| c.discount).unwrap_or(0)
    } else {
        // 初回利用クーポンを最優先で使う
        let coupon: Option<Coupon> = sqlx::query_as(
            "SELECT * FROM coupons WHERE user_id = ? AND code = 'CP_NEW2024' AND used_by IS NULL",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(coupon) = coupon {
            coupon.discount
        } else {
            // 無いなら他のクーポンを付与された順番に使う
            let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE user_id = ? AND used_by IS NULL ORDER BY created_at LIMIT 1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
            coupon.map(|c| c.discount).unwrap_or(0)
        }
    };

    Ok(crate::calculate_fare_with_discount(
        pickup_latitude,
        pickup_longitude,
        dest_latitude,
        dest_longitude,
        discount,
    ))
}
