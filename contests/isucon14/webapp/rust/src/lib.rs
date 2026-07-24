use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use http_body::{Body as HttpBody, Frame, SizeHint};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use crate::models::{Chair, Owner, User};

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: sqlx::MySqlPool,
    pub payment_client: reqwest::Client,
    pub auth_cache: AuthCache,
    pub latest_chair_locations: LatestChairLocationCache,
    pub active_ride_evaluations: ActiveRideEvaluationTracker,
    pub maintenance_lock: Arc<RwLock<()>>,
}

#[derive(Clone, Default)]
pub struct AuthCache {
    users: Arc<StdRwLock<HashMap<String, User>>>,
    owners: Arc<StdRwLock<HashMap<String, Owner>>>,
    chairs: Arc<StdRwLock<HashMap<String, Chair>>>,
}

impl std::fmt::Debug for AuthCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthCache")
            .field(
                "users",
                &self
                    .users
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .len(),
            )
            .field(
                "owners",
                &self
                    .owners
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .len(),
            )
            .field(
                "chairs",
                &self
                    .chairs
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .len(),
            )
            .finish()
    }
}

impl AuthCache {
    pub async fn load(pool: &sqlx::MySqlPool) -> sqlx::Result<Self> {
        let cache = Self::default();
        cache.refresh(pool).await?;
        Ok(cache)
    }

    pub async fn refresh(&self, pool: &sqlx::MySqlPool) -> sqlx::Result<()> {
        let (users, owners, chairs) = tokio::try_join!(
            sqlx::query_as::<_, User>("SELECT * FROM users").fetch_all(pool),
            sqlx::query_as::<_, Owner>("SELECT * FROM owners").fetch_all(pool),
            sqlx::query_as::<_, Chair>("SELECT * FROM chairs").fetch_all(pool),
        )?;

        *self
            .users
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = users
            .into_iter()
            .map(|user| (user.access_token.clone(), user))
            .collect();
        *self
            .owners
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = owners
            .into_iter()
            .map(|owner| (owner.access_token.clone(), owner))
            .collect();
        *self
            .chairs
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = chairs
            .into_iter()
            .map(|chair| (chair.access_token.clone(), chair))
            .collect();
        Ok(())
    }

    pub fn clear(&self) {
        self.users
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.owners
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.chairs
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn user(&self, access_token: &str) -> Option<User> {
        self.users
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(access_token)
            .cloned()
    }

    pub(crate) fn insert_user(&self, user: User) {
        self.users
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(user.access_token.clone(), user);
    }

    pub(crate) fn owner(&self, access_token: &str) -> Option<Owner> {
        self.owners
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(access_token)
            .cloned()
    }

    pub(crate) fn insert_owner(&self, owner: Owner) {
        self.owners
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(owner.access_token.clone(), owner);
    }

    pub(crate) fn chair(&self, access_token: &str) -> Option<Chair> {
        self.chairs
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(access_token)
            .cloned()
    }

    pub(crate) fn insert_chair(&self, chair: Chair) {
        self.chairs
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(chair.access_token.clone(), chair);
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActiveRideEvaluationTracker {
    inner: Arc<StdMutex<ActiveRideEvaluationState>>,
}

#[derive(Debug, Default)]
struct ActiveRideEvaluationState {
    active_counts: HashMap<String, usize>,
    completed_evaluations: HashMap<String, CompletedRideEvaluation>,
    live_snapshot_revisions: BTreeMap<u64, usize>,
    revision: u64,
    generation: u64,
}

#[derive(Debug)]
struct CompletedRideEvaluation {
    revision: u64,
    unavailable_until: Instant,
}

const EVALUATION_RESPONSE_DELIVERY_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) struct ActiveRideEvaluationGuard {
    chair_id: String,
    generation: u64,
    tracker: ActiveRideEvaluationTracker,
}

pub(crate) struct ActiveRideEvaluationSnapshot {
    generation: u64,
    revision: u64,
    chair_ids: HashSet<String>,
    tracker: ActiveRideEvaluationTracker,
}

impl ActiveRideEvaluationTracker {
    pub(crate) fn begin(&self, chair_id: String) -> ActiveRideEvaluationGuard {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state.active_counts.entry(chair_id.clone()).or_default() += 1;
        let generation = state.generation;
        ActiveRideEvaluationGuard {
            chair_id,
            generation,
            tracker: self.clone(),
        }
    }

    #[cfg(test)]
    fn chair_ids(&self) -> HashSet<String> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .active_counts
            .keys()
            .cloned()
            .collect()
    }

    pub(crate) fn snapshot(&self) -> ActiveRideEvaluationSnapshot {
        self.snapshot_at(Instant::now())
    }

    fn snapshot_at(&self, now: Instant) -> ActiveRideEvaluationSnapshot {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Self::prune_completed_evaluations(&mut state, now);
        let revision = state.revision;
        *state.live_snapshot_revisions.entry(revision).or_default() += 1;
        ActiveRideEvaluationSnapshot {
            generation: state.generation,
            revision,
            chair_ids: state
                .active_counts
                .keys()
                .chain(
                    state
                        .completed_evaluations
                        .iter()
                        .filter_map(|(chair_id, completed)| {
                            (completed.unavailable_until > now).then_some(chair_id)
                        }),
                )
                .cloned()
                .collect(),
            tracker: self.clone(),
        }
    }

    pub(crate) fn chair_ids_overlapping(
        &self,
        snapshot: ActiveRideEvaluationSnapshot,
    ) -> HashSet<String> {
        self.chair_ids_overlapping_at(snapshot, Instant::now())
    }

    fn chair_ids_overlapping_at(
        &self,
        mut snapshot: ActiveRideEvaluationSnapshot,
        now: Instant,
    ) -> HashSet<String> {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if snapshot.generation != state.generation {
            let chair_ids = std::mem::take(&mut snapshot.chair_ids);
            drop(state);
            return chair_ids;
        }
        snapshot
            .chair_ids
            .extend(state.active_counts.keys().cloned());
        snapshot
            .chair_ids
            .extend(
                state
                    .completed_evaluations
                    .iter()
                    .filter_map(|(chair_id, completed)| {
                        (completed.revision > snapshot.revision
                            || completed.unavailable_until > now)
                            .then_some(chair_id.clone())
                    }),
            );
        let chair_ids = std::mem::take(&mut snapshot.chair_ids);
        drop(state);
        chair_ids
    }

    pub fn clear(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let generation = state
            .generation
            .checked_add(1)
            .expect("evaluation tracker generation overflow");
        *state = ActiveRideEvaluationState {
            generation,
            ..ActiveRideEvaluationState::default()
        };
    }

    fn prune_completed_evaluations(state: &mut ActiveRideEvaluationState, now: Instant) {
        let oldest_live_snapshot = state.live_snapshot_revisions.keys().next().copied();
        state.completed_evaluations.retain(|_, completed| {
            completed.unavailable_until > now
                || oldest_live_snapshot.is_some_and(|revision| completed.revision > revision)
        });
    }
}

impl Drop for ActiveRideEvaluationGuard {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.generation != state.generation {
            return;
        }
        let Some(active_count) = state.active_counts.get_mut(&self.chair_id) else {
            return;
        };
        *active_count -= 1;
        if *active_count == 0 {
            state.active_counts.remove(&self.chair_id);
            state.revision = state.revision.saturating_add(1);
            let revision = state.revision;
            state.completed_evaluations.insert(
                self.chair_id.clone(),
                CompletedRideEvaluation {
                    revision,
                    unavailable_until: Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE,
                },
            );
        }
    }
}

impl Drop for ActiveRideEvaluationSnapshot {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.generation != state.generation {
            return;
        }
        let Some(snapshot_count) = state.live_snapshot_revisions.get_mut(&self.revision) else {
            return;
        };
        *snapshot_count -= 1;
        if *snapshot_count == 0 {
            state.live_snapshot_revisions.remove(&self.revision);
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

pub async fn ensure_chair_stats(pool: &sqlx::MySqlPool) -> sqlx::Result<()> {
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS chair_stats
(
  chair_id             VARCHAR(26) NOT NULL COMMENT '椅子ID',
  total_rides_count    INTEGER     NOT NULL COMMENT '完了ライド数',
  total_evaluation_sum BIGINT      NOT NULL COMMENT '完了ライドの評価合計',
  PRIMARY KEY (chair_id)
)
COMMENT = '椅子ごとの完了ライド集計テーブル'
        "#,
    )
    .execute(pool)
    .await?;

    // Existing installations can start this binary before the next initialize.
    // Replace the entire projection from the immutable ride/status history so a
    // restart repairs missing, incorrect, and stale rows. InnoDB keeps the old
    // committed projection visible to other connections until this transaction
    // commits, rather than exposing the table between DELETE and INSERT.
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM chair_stats")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
)
SELECT chair_id,
       COUNT(*)        AS total_rides_count,
       SUM(evaluation) AS total_evaluation_sum
FROM (
  SELECT rides.id,
         rides.chair_id,
         rides.evaluation
  FROM rides
  INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
  WHERE rides.chair_id IS NOT NULL
    AND rides.evaluation IS NOT NULL
  GROUP BY rides.id, rides.chair_id, rides.evaluation
  HAVING SUM(ride_statuses.status = 'ARRIVED') > 0
     AND SUM(ride_statuses.status = 'CARRYING') > 0
     AND SUM(ride_statuses.status = 'COMPLETED') > 0
) AS completed_rides
GROUP BY chair_id
        "#,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        hold_active_evaluation_until_response_drop, insert_if_newer, merge_newer_locations,
        ActiveRideEvaluationTracker, LatestChairLocation, LatestChairLocationCache,
        EVALUATION_RESPONSE_DELIVERY_GRACE,
    };
    use axum::response::IntoResponse;
    use std::collections::HashMap;
    use std::time::Instant;

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

    #[test]
    fn active_ride_evaluation_records_overlap_with_a_nearby_request() {
        let tracker = ActiveRideEvaluationTracker::default();

        let completed_before_request = tracker.begin("chair-before".to_owned());
        drop(completed_before_request);
        let request_snapshot =
            tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);

        let active_during_request = tracker.begin("chair-active".to_owned());
        let completed_during_request = tracker.begin("chair-completed".to_owned());
        drop(completed_during_request);

        let after_delivery_grace = Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE;
        let overlapping = tracker.chair_ids_overlapping_at(request_snapshot, after_delivery_grace);
        assert!(!overlapping.contains("chair-before"));
        assert!(overlapping.contains("chair-active"));
        assert!(overlapping.contains("chair-completed"));

        drop(active_during_request);
    }

    #[test]
    fn completed_evaluation_stays_unavailable_during_response_delivery_grace() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned());
        drop(guard);

        assert!(tracker
            .chair_ids_overlapping(tracker.snapshot())
            .contains("chair-1"));
    }

    #[test]
    fn evaluation_snapshot_survives_grace_expiry_during_a_nearby_request() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned());
        drop(guard);
        let request_snapshot = tracker.snapshot();

        assert!(tracker
            .chair_ids_overlapping_at(
                request_snapshot,
                Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE,
            )
            .contains("chair-1"));
    }

    #[test]
    fn evaluation_tracker_clear_removes_previous_generation() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned());
        drop(guard);
        tracker.clear();

        assert!(tracker.chair_ids_overlapping(tracker.snapshot()).is_empty());
    }

    #[test]
    fn stale_guard_cannot_remove_an_active_evaluation_from_a_new_generation() {
        let tracker = ActiveRideEvaluationTracker::default();
        let stale_guard = tracker.begin("chair-1".to_owned());
        tracker.clear();

        let current_guard = tracker.begin("chair-1".to_owned());
        drop(stale_guard);
        assert!(tracker.chair_ids().contains("chair-1"));

        drop(current_guard);
        assert!(!tracker.chair_ids().contains("chair-1"));
        assert!(tracker
            .chair_ids_overlapping(tracker.snapshot())
            .contains("chair-1"));
    }

    #[test]
    fn evaluation_tracker_prunes_expired_completed_entries() {
        let tracker = ActiveRideEvaluationTracker::default();
        for chair_number in 0..128 {
            drop(tracker.begin(format!("chair-{chair_number}")));
        }

        let snapshot = tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);
        assert!(tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .completed_evaluations
            .is_empty());
        drop(snapshot);
    }

    #[test]
    fn pruning_preserves_completion_needed_by_an_older_live_snapshot() {
        let tracker = ActiveRideEvaluationTracker::default();
        let older_snapshot = tracker.snapshot();
        drop(tracker.begin("chair-1".to_owned()));

        let newer_snapshot =
            tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);
        drop(newer_snapshot);

        assert!(tracker
            .chair_ids_overlapping_at(
                older_snapshot,
                Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE,
            )
            .contains("chair-1"));
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
