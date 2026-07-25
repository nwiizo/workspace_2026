use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::Response,
};
use http_body::{Body as HttpBody, Frame, SizeHint};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex as StdMutex, RwLock as StdRwLock,
};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedSemaphorePermit, RwLock, Semaphore};

use crate::models::{Chair, Owner, User};

pub(crate) mod drive_diagnostic;
pub(crate) mod matcher_diagnostic;
pub(crate) mod notification_diagnostic;

pub(crate) const NOTIFICATION_RETRY_AFTER_MS: i32 = 30;
pub(crate) const CACHED_NOTIFICATION_RETRY_AFTER_MS: i32 = 100;
const DB_ADMISSION_DIAGNOSTIC_SAMPLE_EVERY: u64 = 64;
const DB_ADMISSION_FORCE_SAMPLE_MICROSECONDS: u64 = 30_000;
static DB_ADMISSION_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct AppState {
    /// General-purpose pool used by every DB workload except chair coordinates.
    pub pool: sqlx::MySqlPool,
    /// Reserved pool for POST /api/chair/coordinate.
    pub coordinate_pool: sqlx::MySqlPool,
    pub payment_client: reqwest::Client,
    pub auth_cache: AuthCache,
    pub notification_cache: NotificationCache,
    pub latest_chair_locations: LatestChairLocationCache,
    pub coordinate_write_queue: Option<crate::chair_handlers::CoordinateWriteQueue>,
    pub active_ride_evaluations: ActiveRideEvaluationTracker,
    pub maintenance_lock: Arc<RwLock<()>>,
    pub general_db_admission: DbAdmission,
}

#[derive(Debug, Clone, Default)]
pub struct DbAdmission {
    semaphore: Option<Arc<Semaphore>>,
}

#[derive(serde::Serialize)]
struct DbAdmissionDiagnosticSample {
    sequence: u64,
    periodic_sample: bool,
    label: String,
    wait_us: u64,
    available_before: usize,
    available_after: usize,
    pool_size_before: u32,
    pool_idle_before: usize,
    pool_in_use_before: u64,
    event_at_unix_us: u64,
}

impl DbAdmission {
    pub fn limited(permits: usize) -> Self {
        assert!(permits > 0, "database admission permits must be positive");
        Self {
            semaphore: Some(Arc::new(Semaphore::new(permits))),
        }
    }

    pub fn is_limited(&self) -> bool {
        self.semaphore.is_some()
    }

    pub async fn acquire(
        &self,
        label: &str,
        pool: &sqlx::MySqlPool,
    ) -> Option<OwnedSemaphorePermit> {
        let semaphore = self.semaphore.as_ref()?;
        let diagnostic_enabled = drive_diagnostic::enabled();
        let sequence = diagnostic_enabled
            .then(|| DB_ADMISSION_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed));
        let started_at = diagnostic_enabled.then(Instant::now);
        let available_before = semaphore.available_permits();
        let pool_size_before = pool.size();
        let pool_idle_before = pool.num_idle();
        let permit = Arc::clone(semaphore)
            .acquire_owned()
            .await
            .expect("database admission semaphore is never closed");

        if let (Some(sequence), Some(started_at)) = (sequence, started_at) {
            let wait_us = drive_diagnostic::duration_us(started_at.elapsed());
            let periodic_sample = sequence % DB_ADMISSION_DIAGNOSTIC_SAMPLE_EVERY == 0;
            if periodic_sample || wait_us >= DB_ADMISSION_FORCE_SAMPLE_MICROSECONDS {
                drive_diagnostic::emit(
                    "DB_ADMISSION_DIAGNOSTIC",
                    &DbAdmissionDiagnosticSample {
                        sequence,
                        periodic_sample,
                        label: label.to_owned(),
                        wait_us,
                        available_before,
                        available_after: semaphore.available_permits(),
                        pool_size_before,
                        pool_idle_before,
                        pool_in_use_before: u64::from(pool_size_before)
                            .saturating_sub(u64::try_from(pool_idle_before).unwrap_or(u64::MAX)),
                        event_at_unix_us: drive_diagnostic::unix_time_us(),
                    },
                );
            }
        }

        Some(permit)
    }
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

#[derive(Clone, Default)]
pub struct NotificationCache {
    inner: Arc<StdMutex<NotificationCacheState>>,
}

#[derive(Debug, Default)]
struct NotificationCacheState {
    generation: u64,
    app_revisions: HashMap<String, u64>,
    chair_revisions: HashMap<String, u64>,
    chair_stats_revisions: HashMap<String, u64>,
    app_payloads: HashMap<String, AppNotificationCacheEntry>,
    chair_payloads: HashMap<String, Bytes>,
}

#[derive(Debug)]
struct AppNotificationCacheEntry {
    payload: Bytes,
    chair_stats_revision: Option<ChairStatsCacheRevision>,
}

impl std::fmt::Debug for NotificationCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        formatter
            .debug_struct("NotificationCache")
            .field("generation", &state.generation)
            .field("app_payloads", &state.app_payloads.len())
            .field("chair_payloads", &state.chair_payloads.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NotificationCacheRevision {
    generation: u64,
    revision: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ChairStatsCacheRevision {
    chair_id: String,
    revision: u64,
}

impl NotificationCache {
    pub(crate) fn app(&self, user_id: &str) -> (Option<Bytes>, NotificationCacheRevision) {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let payload = state.app_payloads.get(user_id).and_then(|entry| {
            let dependency_is_current =
                entry
                    .chair_stats_revision
                    .as_ref()
                    .is_none_or(|dependency| {
                        state
                            .chair_stats_revisions
                            .get(&dependency.chair_id)
                            .copied()
                            .unwrap_or_default()
                            == dependency.revision
                    });
            dependency_is_current.then(|| entry.payload.clone())
        });
        (
            payload,
            NotificationCacheRevision {
                generation: state.generation,
                revision: state
                    .app_revisions
                    .get(user_id)
                    .copied()
                    .unwrap_or_default(),
            },
        )
    }

    pub(crate) fn chair(&self, chair_id: &str) -> (Option<Bytes>, NotificationCacheRevision) {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            state.chair_payloads.get(chair_id).cloned(),
            NotificationCacheRevision {
                generation: state.generation,
                revision: state
                    .chair_revisions
                    .get(chair_id)
                    .copied()
                    .unwrap_or_default(),
            },
        )
    }

    pub(crate) fn insert_app_if_current(
        &self,
        user_id: String,
        snapshot: NotificationCacheRevision,
        chair_stats_snapshot: Option<ChairStatsCacheRevision>,
        payload: Bytes,
    ) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = state
            .app_revisions
            .get(&user_id)
            .copied()
            .unwrap_or_default();
        let chair_stats_are_current = chair_stats_snapshot.as_ref().is_none_or(|dependency| {
            state
                .chair_stats_revisions
                .get(&dependency.chair_id)
                .copied()
                .unwrap_or_default()
                == dependency.revision
        });
        if state.generation == snapshot.generation
            && revision == snapshot.revision
            && chair_stats_are_current
        {
            state.app_payloads.insert(
                user_id,
                AppNotificationCacheEntry {
                    payload,
                    chair_stats_revision: chair_stats_snapshot,
                },
            );
        }
    }

    pub(crate) fn insert_chair_if_current(
        &self,
        chair_id: String,
        snapshot: NotificationCacheRevision,
        payload: Bytes,
    ) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = state
            .chair_revisions
            .get(&chair_id)
            .copied()
            .unwrap_or_default();
        if state.generation == snapshot.generation && revision == snapshot.revision {
            state.chair_payloads.insert(chair_id, payload);
        }
    }

    pub(crate) fn invalidate_app(&self, user_id: &str) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = state.app_revisions.entry(user_id.to_owned()).or_default();
        *revision = revision.wrapping_add(1);
        state.app_payloads.remove(user_id);
    }

    pub(crate) fn invalidate_chair(&self, chair_id: &str) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = state
            .chair_revisions
            .entry(chair_id.to_owned())
            .or_default();
        *revision = revision.wrapping_add(1);
        state.chair_payloads.remove(chair_id);
    }

    pub(crate) fn chair_stats_revision(&self, chair_id: &str) -> ChairStatsCacheRevision {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ChairStatsCacheRevision {
            chair_id: chair_id.to_owned(),
            revision: state
                .chair_stats_revisions
                .get(chair_id)
                .copied()
                .unwrap_or_default(),
        }
    }

    pub(crate) fn invalidate_chair_stats(&self, chair_id: &str) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revision = state
            .chair_stats_revisions
            .entry(chair_id.to_owned())
            .or_default();
        *revision = revision.wrapping_add(1);
    }

    pub fn clear(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.generation = state.generation.wrapping_add(1);
        state.app_revisions.clear();
        state.chair_revisions.clear();
        state.chair_stats_revisions.clear();
        state.app_payloads.clear();
        state.chair_payloads.clear();
    }
}

pub(crate) fn json_bytes_response(payload: Bytes) -> Response {
    Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload))
        .expect("static JSON response headers are valid")
}

#[derive(Debug, Clone, Default)]
pub struct ActiveRideEvaluationTracker {
    inner: Arc<StdMutex<ActiveRideEvaluationState>>,
}

#[derive(Debug, Default)]
struct ActiveRideEvaluationState {
    active_counts: HashMap<String, usize>,
    active_ride_counts: HashMap<String, usize>,
    completed_evaluations: HashMap<String, CompletedRideEvaluation>,
    completed_ride_evaluations: HashMap<String, CompletedRideEvaluation>,
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
    ride_id: String,
    generation: u64,
    tracker: ActiveRideEvaluationTracker,
}

pub(crate) struct ActiveRideEvaluationSnapshot {
    generation: u64,
    revision: u64,
    chair_ids: HashSet<String>,
    ride_ids: HashSet<String>,
    tracker: ActiveRideEvaluationTracker,
}

impl ActiveRideEvaluationTracker {
    pub(crate) fn begin(&self, chair_id: String, ride_id: String) -> ActiveRideEvaluationGuard {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state.active_counts.entry(chair_id.clone()).or_default() += 1;
        *state.active_ride_counts.entry(ride_id.clone()).or_default() += 1;
        let generation = state.generation;
        ActiveRideEvaluationGuard {
            chair_id,
            ride_id,
            generation,
            tracker: self.clone(),
        }
    }

    pub(crate) fn diagnostic_counts(&self, ride_id: &str) -> (usize, usize) {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            state.active_ride_counts.values().sum(),
            state.active_ride_counts.get(ride_id).copied().unwrap_or(0),
        )
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
            ride_ids: state.active_ride_counts.keys().cloned().collect(),
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

    pub(crate) fn ride_ids_overlapping(
        &self,
        mut snapshot: ActiveRideEvaluationSnapshot,
    ) -> HashSet<String> {
        let state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if snapshot.generation != state.generation {
            let ride_ids = std::mem::take(&mut snapshot.ride_ids);
            drop(state);
            return ride_ids;
        }
        snapshot
            .ride_ids
            .extend(state.active_ride_counts.keys().cloned());
        snapshot
            .ride_ids
            .extend(
                state
                    .completed_ride_evaluations
                    .iter()
                    .filter_map(|(ride_id, completed)| {
                        (completed.revision > snapshot.revision).then_some(ride_id.clone())
                    }),
            );
        let ride_ids = std::mem::take(&mut snapshot.ride_ids);
        drop(state);
        ride_ids
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
        state.completed_ride_evaluations.retain(|_, completed| {
            oldest_live_snapshot.is_some_and(|revision| completed.revision > revision)
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
        let Some(active_chair_count) = state.active_counts.get_mut(&self.chair_id) else {
            return;
        };
        *active_chair_count -= 1;
        let chair_completed = *active_chair_count == 0;
        if chair_completed {
            state.active_counts.remove(&self.chair_id);
        }

        let Some(active_ride_count) = state.active_ride_counts.get_mut(&self.ride_id) else {
            return;
        };
        *active_ride_count -= 1;
        let ride_completed = *active_ride_count == 0;
        if ride_completed {
            state.active_ride_counts.remove(&self.ride_id);
            state.revision = state.revision.saturating_add(1);
            let revision = state.revision;
            let now = Instant::now();
            state.completed_ride_evaluations.insert(
                self.ride_id.clone(),
                CompletedRideEvaluation {
                    revision,
                    unavailable_until: now,
                },
            );
            if chair_completed {
                state.completed_evaluations.insert(
                    self.chair_id.clone(),
                    CompletedRideEvaluation {
                        revision,
                        unavailable_until: now + EVALUATION_RESPONSE_DELIVERY_GRACE,
                    },
                );
            }
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
    recorded_at_high_watermarks: Arc<StdMutex<HashMap<String, chrono::NaiveDateTime>>>,
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
        let refreshed_high_watermarks = recorded_at_high_watermarks(&refreshed_locations);
        let mut cached_locations = self.inner.write().await;
        *cached_locations = refreshed_locations;
        let mut high_watermarks = self
            .recorded_at_high_watermarks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *high_watermarks = refreshed_high_watermarks;
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
        let refreshed_high_watermarks = recorded_at_high_watermarks(&refreshed_locations);
        *cached_locations = refreshed_locations;
        drop(cached_locations);

        // A coordinate may have reserved a timestamp but not committed yet.
        // Reconciliation can advance the reservation watermark from MySQL, but
        // must never move an in-flight process-local reservation backwards.
        let mut high_watermarks = self
            .recorded_at_high_watermarks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        merge_recorded_at_high_watermarks(&mut high_watermarks, refreshed_high_watermarks);
        Ok(())
    }

    pub(crate) fn reserve_recorded_at(
        &self,
        chair_id: &str,
        observed_at: chrono::NaiveDateTime,
    ) -> chrono::NaiveDateTime {
        // MySQL DATETIME(6) discards sub-microsecond precision. Normalize before
        // comparing so two distinct nanosecond values cannot collapse to the
        // same persisted timestamp after this function considered them ordered.
        let observed_at = truncate_to_microseconds(observed_at);
        let mut high_watermarks = self
            .recorded_at_high_watermarks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(high_watermark) = high_watermarks.get_mut(chair_id) {
            let recorded_at = next_chair_recorded_at(Some(*high_watermark), observed_at);
            *high_watermark = recorded_at;
            recorded_at
        } else {
            high_watermarks.insert(chair_id.to_owned(), observed_at);
            observed_at
        }
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

fn truncate_to_microseconds(value: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
    let sub_microsecond_nanoseconds = value.and_utc().timestamp_subsec_nanos() % 1_000;
    value - chrono::Duration::nanoseconds(i64::from(sub_microsecond_nanoseconds))
}

fn next_chair_recorded_at(
    previous: Option<chrono::NaiveDateTime>,
    observed_at: chrono::NaiveDateTime,
) -> chrono::NaiveDateTime {
    let Some(previous) = previous else {
        return observed_at;
    };
    if observed_at > previous {
        return observed_at;
    }

    previous
        .checked_add_signed(chrono::Duration::microseconds(1))
        .unwrap_or(previous)
}

fn recorded_at_high_watermarks(
    locations: &HashMap<String, LatestChairLocation>,
) -> HashMap<String, chrono::NaiveDateTime> {
    locations
        .iter()
        .map(|(chair_id, location)| (chair_id.clone(), location.recorded_at))
        .collect()
}

fn merge_recorded_at_high_watermark(
    high_watermarks: &mut HashMap<String, chrono::NaiveDateTime>,
    chair_id: &str,
    recorded_at: chrono::NaiveDateTime,
) {
    if let Some(high_watermark) = high_watermarks.get_mut(chair_id) {
        *high_watermark = (*high_watermark).max(recorded_at);
    } else {
        high_watermarks.insert(chair_id.to_owned(), recorded_at);
    }
}

fn merge_recorded_at_high_watermarks(
    high_watermarks: &mut HashMap<String, chrono::NaiveDateTime>,
    candidates: impl IntoIterator<Item = (String, chrono::NaiveDateTime)>,
) {
    for (chair_id, recorded_at) in candidates {
        merge_recorded_at_high_watermark(high_watermarks, &chair_id, recorded_at);
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
        merge_recorded_at_high_watermarks, ActiveRideEvaluationTracker, DbAdmission,
        LatestChairLocation, LatestChairLocationCache, NotificationCache,
        EVALUATION_RESPONSE_DELIVERY_GRACE,
    };
    use axum::body::Bytes;
    use axum::response::IntoResponse;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn database_admission_limits_concurrent_general_work() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://isucon:isucon@localhost/isuride")
            .expect("lazy pool");
        let admission = DbAdmission::limited(1);

        let first = admission
            .acquire("first", &pool)
            .await
            .expect("limited admission returns a guard");
        assert!(tokio::time::timeout(
            Duration::from_millis(10),
            admission.acquire("second", &pool)
        )
        .await
        .is_err());

        drop(first);
        assert!(tokio::time::timeout(
            Duration::from_millis(100),
            admission.acquire("second", &pool)
        )
        .await
        .expect("second admission must wake")
        .is_some());
    }

    #[tokio::test]
    async fn disabled_database_admission_does_not_return_a_guard() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://isucon:isucon@localhost/isuride")
            .expect("lazy pool");

        assert!(DbAdmission::default()
            .acquire("disabled", &pool)
            .await
            .is_none());
    }

    #[test]
    fn notification_cache_returns_a_current_payload() {
        let cache = NotificationCache::default();
        let (payload, revision) = cache.app("user-1");
        assert!(payload.is_none());

        cache.insert_app_if_current(
            "user-1".to_owned(),
            revision,
            None,
            Bytes::from_static(b"app-payload"),
        );

        assert_eq!(
            cache.app("user-1").0.unwrap(),
            Bytes::from_static(b"app-payload")
        );
        assert!(cache.chair("user-1").0.is_none());
    }

    #[test]
    fn notification_cache_rejects_an_insert_after_invalidation() {
        let cache = NotificationCache::default();
        let (_, stale_revision) = cache.chair("chair-1");
        cache.invalidate_chair("chair-1");

        cache.insert_chair_if_current(
            "chair-1".to_owned(),
            stale_revision,
            Bytes::from_static(b"stale"),
        );

        assert!(cache.chair("chair-1").0.is_none());
    }

    #[test]
    fn notification_cache_clear_rejects_a_previous_generation() {
        let cache = NotificationCache::default();
        let (_, stale_revision) = cache.app("user-1");
        cache.clear();

        cache.insert_app_if_current(
            "user-1".to_owned(),
            stale_revision,
            None,
            Bytes::from_static(b"stale"),
        );

        assert!(cache.app("user-1").0.is_none());
    }

    #[test]
    fn app_notification_cache_tracks_cross_user_chair_stats_changes() {
        let cache = NotificationCache::default();
        let (_, app_revision) = cache.app("past-user");
        let stale_stats_revision = cache.chair_stats_revision("shared-chair");

        cache.insert_app_if_current(
            "past-user".to_owned(),
            app_revision,
            Some(stale_stats_revision.clone()),
            Bytes::from_static(b"old-stats"),
        );
        assert_eq!(
            cache.app("past-user").0.unwrap(),
            Bytes::from_static(b"old-stats")
        );

        // A later ride can be evaluated by a different user while changing
        // statistics embedded in past-user's notification payload.
        cache.invalidate_chair_stats("shared-chair");
        assert!(cache.app("past-user").0.is_none());

        cache.insert_app_if_current(
            "past-user".to_owned(),
            app_revision,
            Some(stale_stats_revision),
            Bytes::from_static(b"stale-reinsert"),
        );
        assert!(cache.app("past-user").0.is_none());
    }

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
    fn latest_chair_location_cache_reserves_strictly_increasing_recorded_at() {
        let cache = LatestChairLocationCache::default();
        let first_observation = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();
        let regressed_observation = chrono::DateTime::from_timestamp_micros(1_000)
            .unwrap()
            .naive_utc();
        let future_observation = chrono::DateTime::from_timestamp_micros(3_000)
            .unwrap()
            .naive_utc();

        let first = cache.reserve_recorded_at("chair-1", first_observation);
        let after_regression = cache.reserve_recorded_at("chair-1", regressed_observation);
        let after_tie = cache.reserve_recorded_at("chair-1", after_regression);
        let future = cache.reserve_recorded_at("chair-1", future_observation);
        let other_chair = cache.reserve_recorded_at("chair-2", regressed_observation);

        assert_eq!(first, first_observation);
        assert_eq!(
            after_regression,
            first_observation + chrono::Duration::microseconds(1)
        );
        assert_eq!(
            after_tie,
            after_regression + chrono::Duration::microseconds(1)
        );
        assert_eq!(future, future_observation);
        assert_eq!(other_chair, regressed_observation);
    }

    #[test]
    fn latest_chair_location_cache_orders_values_that_mysql_truncates_to_one_microsecond() {
        let cache = LatestChairLocationCache::default();
        let first_observation = chrono::DateTime::from_timestamp(0, 2_000_100)
            .unwrap()
            .naive_utc();
        let second_observation = chrono::DateTime::from_timestamp(0, 2_000_200)
            .unwrap()
            .naive_utc();
        let persisted_first = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();

        let first = cache.reserve_recorded_at("chair-1", first_observation);
        let second = cache.reserve_recorded_at("chair-1", second_observation);

        assert_eq!(first, persisted_first);
        assert_eq!(second, persisted_first + chrono::Duration::microseconds(1));
    }

    #[test]
    fn latest_chair_location_cache_serializes_concurrent_recorded_at_reservations() {
        const REQUESTS: usize = 32;

        let cache = LatestChairLocationCache::default();
        let barrier = Arc::new(std::sync::Barrier::new(REQUESTS));
        let observed_at = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();
        let mut reservations = std::thread::scope(|scope| {
            let handles = (0..REQUESTS)
                .map(|_| {
                    let cache = cache.clone();
                    let barrier = Arc::clone(&barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        cache.reserve_recorded_at("chair-1", observed_at)
                    })
                })
                .collect::<Vec<_>>();

            handles
                .into_iter()
                .map(|handle| handle.join().expect("reservation thread must not panic"))
                .collect::<Vec<_>>()
        });
        reservations.sort_unstable();

        let expected = (0..REQUESTS)
            .map(|offset| {
                observed_at
                    + chrono::Duration::microseconds(
                        i64::try_from(offset).expect("request count fits i64"),
                    )
            })
            .collect::<Vec<_>>();
        assert_eq!(reservations, expected);
    }

    #[test]
    fn recorded_at_reconciliation_never_rewinds_process_reservations() {
        let cache = LatestChairLocationCache::default();
        let process_reservation = chrono::DateTime::from_timestamp_micros(3_000)
            .unwrap()
            .naive_utc();
        let older_database_value = chrono::DateTime::from_timestamp_micros(2_000)
            .unwrap()
            .naive_utc();
        let newer_database_value = chrono::DateTime::from_timestamp_micros(5_000)
            .unwrap()
            .naive_utc();

        cache.reserve_recorded_at("chair-1", process_reservation);
        {
            let mut high_watermarks = cache
                .recorded_at_high_watermarks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            merge_recorded_at_high_watermarks(
                &mut high_watermarks,
                [("chair-1".to_owned(), older_database_value)],
            );
        }
        let after_older_snapshot = cache.reserve_recorded_at("chair-1", older_database_value);
        assert_eq!(
            after_older_snapshot,
            process_reservation + chrono::Duration::microseconds(1)
        );

        {
            let mut high_watermarks = cache
                .recorded_at_high_watermarks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            merge_recorded_at_high_watermarks(
                &mut high_watermarks,
                [("chair-1".to_owned(), newer_database_value)],
            );
        }
        let after_newer_snapshot = cache.reserve_recorded_at("chair-1", older_database_value);
        assert_eq!(
            after_newer_snapshot,
            newer_database_value + chrono::Duration::microseconds(1)
        );
    }

    #[test]
    fn active_ride_evaluation_is_removed_when_the_guard_drops() {
        let tracker = ActiveRideEvaluationTracker::default();
        let first_guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
        let second_guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());

        assert_eq!(tracker.diagnostic_counts("ride-1"), (2, 2));
        assert!(tracker.chair_ids().contains("chair-1"));
        drop(first_guard);
        assert_eq!(tracker.diagnostic_counts("ride-1"), (1, 1));
        assert!(tracker.chair_ids().contains("chair-1"));
        drop(second_guard);
        assert_eq!(tracker.diagnostic_counts("ride-1"), (0, 0));
        assert!(!tracker.chair_ids().contains("chair-1"));
    }

    #[test]
    fn active_ride_evaluation_records_overlap_with_a_nearby_request() {
        let tracker = ActiveRideEvaluationTracker::default();

        let completed_before_request =
            tracker.begin("chair-before".to_owned(), "ride-before".to_owned());
        drop(completed_before_request);
        let request_snapshot =
            tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);

        let active_during_request =
            tracker.begin("chair-active".to_owned(), "ride-active".to_owned());
        let completed_during_request =
            tracker.begin("chair-completed".to_owned(), "ride-completed".to_owned());
        drop(completed_during_request);

        let after_delivery_grace = Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE;
        let overlapping = tracker.chair_ids_overlapping_at(request_snapshot, after_delivery_grace);
        assert!(!overlapping.contains("chair-before"));
        assert!(overlapping.contains("chair-active"));
        assert!(overlapping.contains("chair-completed"));

        drop(active_during_request);
    }

    #[test]
    fn ride_overlap_excludes_only_evaluations_overlapping_the_sales_request() {
        let tracker = ActiveRideEvaluationTracker::default();

        drop(tracker.begin("chair-before".to_owned(), "ride-before".to_owned()));
        let request_snapshot = tracker.snapshot();

        let active_during_request =
            tracker.begin("chair-active".to_owned(), "ride-active".to_owned());
        drop(tracker.begin("chair-completed".to_owned(), "ride-completed".to_owned()));

        let overlapping = tracker.ride_ids_overlapping(request_snapshot);
        assert!(!overlapping.contains("ride-before"));
        assert!(overlapping.contains("ride-active"));
        assert!(overlapping.contains("ride-completed"));

        drop(active_during_request);
    }

    #[test]
    fn completed_evaluation_stays_unavailable_during_response_delivery_grace() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
        drop(guard);

        assert!(tracker
            .chair_ids_overlapping(tracker.snapshot())
            .contains("chair-1"));
    }

    #[test]
    fn evaluation_snapshot_survives_grace_expiry_during_a_nearby_request() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
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
        let guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
        drop(guard);
        tracker.clear();

        assert!(tracker.chair_ids_overlapping(tracker.snapshot()).is_empty());
    }

    #[test]
    fn stale_guard_cannot_remove_an_active_evaluation_from_a_new_generation() {
        let tracker = ActiveRideEvaluationTracker::default();
        let stale_guard = tracker.begin("chair-1".to_owned(), "ride-stale".to_owned());
        tracker.clear();

        let current_guard = tracker.begin("chair-1".to_owned(), "ride-current".to_owned());
        drop(stale_guard);
        assert!(tracker.chair_ids().contains("chair-1"));

        drop(current_guard);
        assert!(!tracker.chair_ids().contains("chair-1"));
        assert!(tracker
            .chair_ids_overlapping(tracker.snapshot())
            .contains("chair-1"));
    }

    #[test]
    fn stale_guard_cannot_remove_an_active_ride_from_a_new_generation() {
        let tracker = ActiveRideEvaluationTracker::default();
        let stale_guard = tracker.begin("chair-stale".to_owned(), "ride-1".to_owned());
        tracker.clear();

        let current_guard = tracker.begin("chair-current".to_owned(), "ride-1".to_owned());
        drop(stale_guard);
        assert!(tracker
            .ride_ids_overlapping(tracker.snapshot())
            .contains("ride-1"));

        drop(current_guard);
        assert!(tracker.ride_ids_overlapping(tracker.snapshot()).is_empty());
    }

    #[test]
    fn evaluation_tracker_prunes_expired_completed_entries() {
        let tracker = ActiveRideEvaluationTracker::default();
        for chair_number in 0..128 {
            drop(tracker.begin(
                format!("chair-{chair_number}"),
                format!("ride-{chair_number}"),
            ));
        }

        let snapshot = tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);
        let state = tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(state.completed_evaluations.is_empty());
        assert!(state.completed_ride_evaluations.is_empty());
        drop(state);
        drop(snapshot);
    }

    #[test]
    fn pruning_preserves_completion_needed_by_an_older_live_snapshot() {
        let tracker = ActiveRideEvaluationTracker::default();
        let older_snapshot = tracker.snapshot();
        drop(tracker.begin("chair-1".to_owned(), "ride-1".to_owned()));

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

    #[test]
    fn ride_completion_is_retained_only_while_an_older_snapshot_needs_it() {
        let tracker = ActiveRideEvaluationTracker::default();
        let older_snapshot = tracker.snapshot();
        drop(tracker.begin("chair-1".to_owned(), "ride-1".to_owned()));

        let newer_snapshot =
            tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);
        drop(newer_snapshot);
        assert!(tracker
            .ride_ids_overlapping(older_snapshot)
            .contains("ride-1"));

        let cleanup_snapshot =
            tracker.snapshot_at(Instant::now() + EVALUATION_RESPONSE_DELIVERY_GRACE);
        assert!(tracker
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .completed_ride_evaluations
            .is_empty());
        drop(cleanup_snapshot);
    }

    #[tokio::test]
    async fn active_ride_evaluation_lives_until_response_body_is_consumed() {
        let tracker = ActiveRideEvaluationTracker::default();
        let guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
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
        let guard = tracker.begin("chair-1".to_owned(), "ride-1".to_owned());
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
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
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
