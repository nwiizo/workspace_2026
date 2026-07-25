use std::io::Write as _;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use std::time::Instant;

static MATCHER_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();
static MATCHER_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(serde::Serialize)]
pub(crate) struct MatcherDiagnosticSample {
    sequence: u64,
    pub(crate) pool_size_before: u64,
    pub(crate) pool_idle_before: u64,
    pub(crate) pool_in_use_before: u64,
    pub(crate) pool_begin_us: Option<u64>,
    pub(crate) pending_query_us: Option<u64>,
    pub(crate) available_query_us: Option<u64>,
    pub(crate) matching_update_us: Option<u64>,
    pub(crate) commit_us: Option<u64>,
    pub(crate) cache_invalidation_us: Option<u64>,
    pub(crate) pending_selected: usize,
    pub(crate) pending_selected_by_region: [usize; 2],
    pub(crate) pending_batch_full: bool,
    pub(crate) available_selected: usize,
    pub(crate) available_selected_by_region: [usize; 2],
    pub(crate) available_batch_full: bool,
    pub(crate) matching_attempted: usize,
    pub(crate) matched: usize,
    pub(crate) unmatched_in_batch: usize,
    pub(crate) matched_distance_sum: u64,
    pub(crate) matched_distance_max: Option<u64>,
    pub(crate) matched_distance_gt_200: usize,
    pub(crate) oldest_pending_id: Option<String>,
    pub(crate) oldest_pending_created_at_ms: Option<i64>,
    pub(crate) oldest_pending_age_ms: Option<i64>,
    total_us: u64,
    outcome: &'static str,
    pub(crate) terminal_phase: &'static str,
}

pub(crate) struct MatcherDiagnostic {
    started_at: Instant,
    checkpoint_at: Instant,
    pub(crate) sample: MatcherDiagnosticSample,
    emitted: bool,
}

impl MatcherDiagnostic {
    pub(crate) fn sampled(pool: &sqlx::MySqlPool) -> Option<Self> {
        let enabled = *MATCHER_DIAGNOSTICS_ENABLED.get_or_init(|| {
            std::env::var_os("ISUCON_DIAGNOSTIC").as_deref() == Some(std::ffi::OsStr::new("1"))
        });
        if !enabled {
            return None;
        }

        let sequence = MATCHER_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let started_at = Instant::now();
        let pool_size = u64::from(pool.size());
        let pool_idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            sample: MatcherDiagnosticSample {
                sequence,
                pool_size_before: pool_size,
                pool_idle_before: pool_idle,
                pool_in_use_before: pool_size.saturating_sub(pool_idle),
                pool_begin_us: None,
                pending_query_us: None,
                available_query_us: None,
                matching_update_us: None,
                commit_us: None,
                cache_invalidation_us: None,
                pending_selected: 0,
                pending_selected_by_region: [0; 2],
                pending_batch_full: false,
                available_selected: 0,
                available_selected_by_region: [0; 2],
                available_batch_full: false,
                matching_attempted: 0,
                matched: 0,
                unmatched_in_batch: 0,
                matched_distance_sum: 0,
                matched_distance_max: None,
                matched_distance_gt_200: 0,
                oldest_pending_id: None,
                oldest_pending_created_at_ms: None,
                oldest_pending_age_ms: None,
                total_us: 0,
                outcome: "error_or_cancelled",
                terminal_phase: "pool_begin",
            },
            emitted: false,
        })
    }

    pub(crate) fn elapsed_since_checkpoint_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.checkpoint_at).as_micros();
        self.checkpoint_at = now;
        elapsed.min(u128::from(u64::MAX)) as u64
    }

    pub(crate) fn observe_oldest_pending(&mut self, id: &str, created_at: chrono::NaiveDateTime) {
        self.sample.oldest_pending_id = Some(id.to_owned());
        self.sample.oldest_pending_created_at_ms = Some(created_at.and_utc().timestamp_millis());
        self.sample.oldest_pending_age_ms = Some(
            (chrono::Utc::now().naive_utc() - created_at)
                .num_milliseconds()
                .max(0),
        );
    }

    pub(crate) fn observe_match_distance(&mut self, distance: u64) {
        self.sample.matched_distance_sum =
            self.sample.matched_distance_sum.saturating_add(distance);
        self.sample.matched_distance_max =
            Some(self.sample.matched_distance_max.unwrap_or(0).max(distance));
        if distance > 200 {
            self.sample.matched_distance_gt_200 += 1;
        }
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        self.sample.total_us = self
            .started_at
            .elapsed()
            .as_micros()
            .min(u128::from(u64::MAX)) as u64;
        if let Ok(json) = serde_json::to_string(&self.sample) {
            let _ = writeln!(std::io::stdout().lock(), "MATCHER_DIAGNOSTIC {json}");
        }
    }

    pub(crate) fn emit_success(mut self) {
        self.sample.outcome = "success";
        self.sample.terminal_phase = "complete";
        self.emit_record();
    }
}

impl Drop for MatcherDiagnostic {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_record();
        }
    }
}
