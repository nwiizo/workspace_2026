use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use http_body::{Body as HttpBody, Frame, SizeHint};
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::task::{Context, Poll};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: sqlx::MySqlPool,
    pub payment_client: reqwest::Client,
    pub latest_chair_locations: LatestChairLocationCache,
    pub active_ride_evaluations: ActiveRideEvaluationTracker,
    pub maintenance_lock: Arc<RwLock<()>>,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveRideEvaluationTracker {
    inner: Arc<StdMutex<HashMap<String, usize>>>,
}

#[derive(Debug)]
pub(crate) struct ActiveRideEvaluationGuard {
    chair_id: String,
    tracker: ActiveRideEvaluationTracker,
}

impl ActiveRideEvaluationTracker {
    pub(crate) fn begin(&self, chair_id: String) -> ActiveRideEvaluationGuard {
        let mut active_evaluations = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *active_evaluations.entry(chair_id.clone()).or_default() += 1;
        ActiveRideEvaluationGuard {
            chair_id,
            tracker: self.clone(),
        }
    }

    pub(crate) fn chair_ids(&self) -> HashSet<String> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }
}

impl Drop for ActiveRideEvaluationGuard {
    fn drop(&mut self) {
        let mut active_evaluations = self
            .tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(active_count) = active_evaluations.get_mut(&self.chair_id) else {
            return;
        };
        *active_count -= 1;
        if *active_count == 0 {
            active_evaluations.remove(&self.chair_id);
        }
    }
}

struct ActiveRideEvaluationBody {
    inner: Pin<Box<Body>>,
    _guard: ActiveRideEvaluationGuard,
}

impl HttpBody for ActiveRideEvaluationBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.get_mut().inner.as_mut().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

pub(crate) fn hold_active_evaluation_until_response_drop(
    response: Response,
    guard: Option<ActiveRideEvaluationGuard>,
) -> Response {
    let Some(guard) = guard else {
        return response;
    };

    response.map(|body| {
        Body::new(ActiveRideEvaluationBody {
            inner: Box::pin(body),
            _guard: guard,
        })
    })
}

#[derive(Debug, Clone)]
struct LatestChairLocation {
    latitude: i32,
    longitude: i32,
    recorded_at: chrono::NaiveDateTime,
    id: String,
}

#[derive(Debug, Clone, Default)]
pub struct LatestChairLocationCache {
    inner: Arc<RwLock<HashMap<String, LatestChairLocation>>>,
    reconciliation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, sqlx::FromRow)]
struct LatestChairLocationRow {
    chair_id: String,
    latitude: i32,
    longitude: i32,
    recorded_at: chrono::NaiveDateTime,
    id: String,
}

impl LatestChairLocationCache {
    pub async fn load(pool: &sqlx::MySqlPool) -> sqlx::Result<Self> {
        ensure_chair_current_locations(pool).await?;
        let cache = Self::default();
        cache.refresh(pool).await?;
        Ok(cache)
    }

    pub async fn refresh(&self, pool: &sqlx::MySqlPool) -> sqlx::Result<()> {
        let _reconciliation_guard = self.reconciliation_lock.lock().await;
        let refreshed_locations = fetch_latest_chair_locations(pool).await?;
        let mut cached_locations = self.inner.write().await;
        *cached_locations = refreshed_locations;
        Ok(())
    }

    pub async fn reconcile(&self, pool: &sqlx::MySqlPool) -> sqlx::Result<()> {
        let _reconciliation_guard = self.reconciliation_lock.lock().await;
        let mut refreshed_locations = fetch_latest_chair_locations(pool).await?;
        let mut cached_locations = self.inner.write().await;

        // A coordinate can commit after the SELECT snapshot and update the process
        // cache before this write lock is acquired. Merge those cache entries into
        // the fetched snapshot so reconciliation neither loses that commit nor
        // blocks every nearby read while waiting for MySQL.
        merge_newer_locations(&mut refreshed_locations, cached_locations.drain());
        *cached_locations = refreshed_locations;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        chair_id: String,
        id: String,
        latitude: i32,
        longitude: i32,
        recorded_at: chrono::NaiveDateTime,
    ) {
        let location = LatestChairLocation {
            latitude,
            longitude,
            recorded_at,
            id,
        };
        let mut latest_locations = self.inner.write().await;
        insert_if_newer(&mut latest_locations, chair_id, location);
    }

    pub(crate) async fn coordinates_for<'a>(
        &self,
        chair_ids: impl IntoIterator<Item = &'a str>,
    ) -> Vec<Option<Coordinate>> {
        let latest_locations = self.inner.read().await;
        chair_ids
            .into_iter()
            .map(|chair_id| {
                latest_locations.get(chair_id).map(|location| Coordinate {
                    latitude: location.latitude,
                    longitude: location.longitude,
                })
            })
            .collect()
    }

    pub(crate) async fn contains(&self, chair_id: &str) -> bool {
        self.inner.read().await.contains_key(chair_id)
    }
}

impl LatestChairLocation {
    fn is_newer_than(&self, other: &Self) -> bool {
        self.recorded_at > other.recorded_at
            || (self.recorded_at == other.recorded_at && self.id > other.id)
    }
}

fn insert_if_newer(
    latest_locations: &mut HashMap<String, LatestChairLocation>,
    chair_id: String,
    location: LatestChairLocation,
) {
    match latest_locations.entry(chair_id) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(location);
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            if location.is_newer_than(entry.get()) {
                entry.insert(location);
            }
        }
    }
}

fn merge_newer_locations(
    latest_locations: &mut HashMap<String, LatestChairLocation>,
    candidates: impl IntoIterator<Item = (String, LatestChairLocation)>,
) {
    for (chair_id, location) in candidates {
        insert_if_newer(latest_locations, chair_id, location);
    }
}

async fn fetch_latest_chair_locations(
    pool: &sqlx::MySqlPool,
) -> sqlx::Result<HashMap<String, LatestChairLocation>> {
    let rows: Vec<LatestChairLocationRow> = sqlx::query_as(
        r#"
SELECT chair_id,
       latitude,
       longitude,
       created_at AS recorded_at,
       location_id AS id
FROM chair_current_locations
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut latest_locations = HashMap::with_capacity(rows.len());
    for row in rows {
        let location = LatestChairLocation {
            latitude: row.latitude,
            longitude: row.longitude,
            recorded_at: row.recorded_at,
            id: row.id,
        };
        insert_if_newer(&mut latest_locations, row.chair_id, location);
    }
    Ok(latest_locations)
}

async fn ensure_chair_current_locations(pool: &sqlx::MySqlPool) -> sqlx::Result<()> {
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS chair_current_locations
(
  chair_id    VARCHAR(26) NOT NULL COMMENT '椅子ID',
  location_id VARCHAR(26) NOT NULL COMMENT 'chair_locationsのID',
  latitude    INTEGER     NOT NULL COMMENT '経度',
  longitude   INTEGER     NOT NULL COMMENT '緯度',
  created_at  DATETIME(6) NOT NULL COMMENT '登録日時',
  PRIMARY KEY (chair_id)
)
COMMENT = '椅子ごとの最新位置テーブル'
        "#,
    )
    .execute(pool)
    .await?;

    // The table is a derived current-state projection. Recompute the canonical
    // latest row for every chair so startup repairs both missing rows and stale
    // partial migrations in an existing Docker volume.
    sqlx::query(
        r#"
INSERT INTO chair_current_locations (
  chair_id,
  location_id,
  latitude,
  longitude,
  created_at
)
SELECT chair_id,
       id,
       latitude,
       longitude,
       created_at
FROM (
  SELECT chair_id,
         id,
         latitude,
         longitude,
         created_at,
         ROW_NUMBER() OVER (
           PARTITION BY chair_id
           ORDER BY created_at DESC, id DESC
         ) AS row_rank
  FROM chair_locations
) AS ranked_locations
WHERE row_rank = 1
ON DUPLICATE KEY UPDATE
  latitude = IF(
    VALUES(created_at) > chair_current_locations.created_at
      OR (
        VALUES(created_at) = chair_current_locations.created_at
        AND VALUES(location_id) > chair_current_locations.location_id
      ),
    VALUES(latitude),
    chair_current_locations.latitude
  ),
  longitude = IF(
    VALUES(created_at) > chair_current_locations.created_at
      OR (
        VALUES(created_at) = chair_current_locations.created_at
        AND VALUES(location_id) > chair_current_locations.location_id
      ),
    VALUES(longitude),
    chair_current_locations.longitude
  ),
  location_id = IF(
    VALUES(created_at) > chair_current_locations.created_at
      OR (
        VALUES(created_at) = chair_current_locations.created_at
        AND VALUES(location_id) > chair_current_locations.location_id
      ),
    VALUES(location_id),
    chair_current_locations.location_id
  ),
  created_at = GREATEST(VALUES(created_at), chair_current_locations.created_at)
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        hold_active_evaluation_until_response_drop, insert_if_newer, merge_newer_locations,
        ActiveRideEvaluationTracker, LatestChairLocation, LatestChairLocationCache,
    };
    use axum::response::IntoResponse;
    use std::collections::HashMap;

    #[tokio::test]
    async fn latest_chair_location_cache_keeps_the_newest_coordinate() {
        let cache = LatestChairLocationCache::default();
        let older = chrono::DateTime::from_timestamp_micros(1_000)
            .unwrap()
            .naive_utc();
        let newer = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();

        cache
            .update(
                "chair-1".to_owned(),
                "location-new".to_owned(),
                20,
                30,
                newer,
            )
            .await;
        cache
            .update(
                "chair-1".to_owned(),
                "location-old".to_owned(),
                10,
                15,
                older,
            )
            .await;

        let locations = cache.coordinates_for(["chair-1"]).await;
        let location = locations[0].as_ref().unwrap();
        assert_eq!((location.latitude, location.longitude), (20, 30));
    }

    #[tokio::test]
    async fn latest_chair_location_cache_breaks_timestamp_ties_by_id() {
        let cache = LatestChairLocationCache::default();
        let recorded_at = chrono::DateTime::from_timestamp_micros(1_000)
            .unwrap()
            .naive_utc();

        cache
            .update(
                "chair-1".to_owned(),
                "location-a".to_owned(),
                10,
                15,
                recorded_at,
            )
            .await;
        cache
            .update(
                "chair-1".to_owned(),
                "location-b".to_owned(),
                20,
                30,
                recorded_at,
            )
            .await;

        let locations = cache.coordinates_for(["chair-1"]).await;
        let location = locations[0].as_ref().unwrap();
        assert_eq!((location.latitude, location.longitude), (20, 30));
    }

    #[test]
    fn active_ride_evaluation_is_removed_when_the_guard_drops() {
        let tracker = ActiveRideEvaluationTracker::default();
        let first_guard = tracker.begin("chair-1".to_owned());
        let second_guard = tracker.begin("chair-1".to_owned());

        assert!(tracker.chair_ids().contains("chair-1"));
        drop(first_guard);
        assert!(tracker.chair_ids().contains("chair-1"));
        drop(second_guard);
        assert!(!tracker.chair_ids().contains("chair-1"));
    }

    #[tokio::test]
    async fn active_ride_evaluation_lives_until_response_body_is_consumed() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned());
        let response = hold_active_evaluation_until_response_drop(
            axum::Json("ok").into_response(),
            Some(guard),
        );

        assert!(tracker.chair_ids().contains("chair-1"));
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), br#""ok""#);
        assert!(!tracker.chair_ids().contains("chair-1"));
    }

    #[test]
    fn active_ride_evaluation_is_removed_when_response_body_is_disconnected() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned());
        let response = hold_active_evaluation_until_response_drop(
            axum::Json("ok").into_response(),
            Some(guard),
        );

        assert!(tracker.chair_ids().contains("chair-1"));
        drop(response);
        assert!(!tracker.chair_ids().contains("chair-1"));
    }

    #[test]
    fn versioned_insert_does_not_overwrite_a_newer_coordinate() {
        let older = chrono::DateTime::from_timestamp_micros(1_000)
            .unwrap()
            .naive_utc();
        let newer = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();
        let mut locations = HashMap::from([(
            "chair-1".to_owned(),
            LatestChairLocation {
                latitude: 20,
                longitude: 30,
                recorded_at: newer,
                id: "location-new".to_owned(),
            },
        )]);

        insert_if_newer(
            &mut locations,
            "chair-1".to_owned(),
            LatestChairLocation {
                latitude: 10,
                longitude: 15,
                recorded_at: older,
                id: "location-old".to_owned(),
            },
        );

        let location = locations.get("chair-1").unwrap();
        assert_eq!((location.latitude, location.longitude), (20, 30));
    }

    #[test]
    fn reconciliation_snapshot_does_not_overwrite_a_concurrent_cache_update() {
        let older = chrono::DateTime::from_timestamp_micros(1_000)
            .unwrap()
            .naive_utc();
        let newer = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();
        let mut fetched_snapshot = HashMap::from([(
            "chair-1".to_owned(),
            LatestChairLocation {
                latitude: 10,
                longitude: 15,
                recorded_at: older,
                id: "location-old".to_owned(),
            },
        )]);
        let cache_updates_after_snapshot = [(
            "chair-1".to_owned(),
            LatestChairLocation {
                latitude: 20,
                longitude: 30,
                recorded_at: newer,
                id: "location-new".to_owned(),
            },
        )];

        merge_newer_locations(&mut fetched_snapshot, cache_updates_after_snapshot);

        let location = fetched_snapshot.get("chair-1").unwrap();
        assert_eq!((location.latitude, location.longitude), (20, 30));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("failed to initialize: stdout={stdout} stderr={stderr}")]
    Initialize { stdout: String, stderr: String },
    #[error("{0}")]
    PaymentGateway(#[from] crate::payment_gateway::PaymentGatewayError),
    #[error("{0}")]
    BadRequest(&'static str),
    #[error("{0}")]
    Unauthorized(&'static str),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(&'static str),
}
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::PaymentGateway(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        #[derive(Debug, serde::Serialize)]
        struct ErrorBody {
            message: String,
        }
        let message = self.to_string();
        tracing::error!("{message}");

        (status, axum::Json(ErrorBody { message })).into_response()
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Coordinate {
    pub latitude: i32,
    pub longitude: i32,
}

pub fn secure_random_str(b: usize) -> String {
    use rand::RngCore as _;
    let mut buf = vec![0; b];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut buf);
    hex::encode(&buf)
}

pub async fn get_latest_ride_status<'e, E>(executor: E, ride_id: &str) -> sqlx::Result<String>
where
    E: 'e + sqlx::Executor<'e, Database = sqlx::MySql>,
{
    sqlx::query_scalar(
        "SELECT status FROM ride_statuses WHERE ride_id = ? ORDER BY status DESC LIMIT 1",
    )
    .bind(ride_id)
    .fetch_one(executor)
    .await
}

// マンハッタン距離を求める
pub fn calculate_distance(
    a_latitude: i32,
    a_longitude: i32,
    b_latitude: i32,
    b_longitude: i32,
) -> i32 {
    (a_latitude - b_latitude).abs() + (a_longitude - b_longitude).abs()
}

const INITIAL_FARE: i32 = 500;
const FARE_PER_DISTANCE: i32 = 100;

pub fn calculate_fare(
    pickup_latitude: i32,
    pickup_longitude: i32,
    dest_latitude: i32,
    dest_longitude: i32,
) -> i32 {
    let metered_fare = FARE_PER_DISTANCE
        * calculate_distance(
            pickup_latitude,
            pickup_longitude,
            dest_latitude,
            dest_longitude,
        );
    INITIAL_FARE + metered_fare
}

pub mod app_handlers;
pub mod chair_handlers;
pub mod internal_handlers;
pub mod middlewares;
pub mod models;
pub mod owner_handlers;
pub mod payment_gateway;
