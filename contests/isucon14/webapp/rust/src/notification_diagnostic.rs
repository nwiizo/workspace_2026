use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use std::time::Instant;

const NOTIFICATION_DIAGNOSTIC_SAMPLE_EVERY: u64 = 64;
static APP_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static CHAIR_DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static NOTIFICATION_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) enum NotificationEndpoint {
    App,
    Chair,
}

impl NotificationEndpoint {
    fn name(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Chair => "chair",
        }
    }

    fn next_sequence(self) -> u64 {
        match self {
            Self::App => APP_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            Self::Chair => CHAIR_DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum NotificationConnectionStage {
    InitialLookup,
    Transaction,
}

#[derive(serde::Serialize)]
pub(crate) struct NotificationDiagnosticSample {
    endpoint: &'static str,
    sequence: u64,
    periodic_sample: bool,
    trace_ride: bool,
    ride_id: Option<String>,
    ride_status: Option<String>,
    recipient_id: Option<String>,
    response_built_at_unix_us: Option<u64>,
    pub(crate) cache_lookup_us: Option<u64>,
    pub(crate) initial_pool_acquire_us: Option<u64>,
    pub(crate) initial_pool_size_before: Option<u64>,
    pub(crate) initial_pool_idle_before: Option<u64>,
    pub(crate) initial_pool_in_use_before: Option<u64>,
    pub(crate) latest_ride_query_us: Option<u64>,
    initial_connection_owned_us: Option<u64>,
    pub(crate) dependency_revision_us: Option<u64>,
    pub(crate) transaction_pool_acquire_us: Option<u64>,
    pub(crate) transaction_begin_us: Option<u64>,
    pub(crate) transaction_pool_size_before: Option<u64>,
    pub(crate) transaction_pool_idle_before: Option<u64>,
    pub(crate) transaction_pool_in_use_before: Option<u64>,
    pub(crate) ride_query_us: Option<u64>,
    pub(crate) pending_status_query_us: Option<u64>,
    pub(crate) latest_status_query_us: Option<u64>,
    pub(crate) fare_query_us: Option<u64>,
    pub(crate) chair_query_us: Option<u64>,
    pub(crate) chair_stats_query_us: Option<u64>,
    pub(crate) user_query_us: Option<u64>,
    pub(crate) sent_update_us: Option<u64>,
    pub(crate) commit_us: Option<u64>,
    transaction_connection_owned_us: Option<u64>,
    connection_owned_us: Option<u64>,
    pub(crate) response_us: Option<u64>,
    total_us: u64,
    pub(crate) path: &'static str,
    pub(crate) cache_insert_attempted: bool,
    outcome: &'static str,
    pub(crate) terminal_phase: &'static str,
}

pub(crate) struct NotificationDiagnostic {
    started_at: Instant,
    checkpoint_at: Instant,
    connection_acquired_at: Option<Instant>,
    connection_stage: Option<NotificationConnectionStage>,
    pub(crate) sample: NotificationDiagnosticSample,
    force_emit: bool,
    emitted: bool,
}

impl NotificationDiagnostic {
    pub(crate) fn sampled(endpoint: NotificationEndpoint) -> Option<Self> {
        let enabled = *NOTIFICATION_DIAGNOSTICS_ENABLED.get_or_init(|| {
            std::env::var_os("ISUCON_DIAGNOSTIC").as_deref() == Some(std::ffi::OsStr::new("1"))
        });
        if !enabled {
            return None;
        }

        let sequence = endpoint.next_sequence();
        let periodic_sample = sequence.checked_rem(NOTIFICATION_DIAGNOSTIC_SAMPLE_EVERY) == Some(0);

        let started_at = Instant::now();
        Some(Self {
            started_at,
            checkpoint_at: started_at,
            connection_acquired_at: None,
            connection_stage: None,
            sample: NotificationDiagnosticSample {
                endpoint: endpoint.name(),
                sequence,
                periodic_sample,
                trace_ride: false,
                ride_id: None,
                ride_status: None,
                recipient_id: None,
                response_built_at_unix_us: None,
                cache_lookup_us: None,
                initial_pool_acquire_us: None,
                initial_pool_size_before: None,
                initial_pool_idle_before: None,
                initial_pool_in_use_before: None,
                latest_ride_query_us: None,
                initial_connection_owned_us: None,
                dependency_revision_us: None,
                transaction_pool_acquire_us: None,
                transaction_begin_us: None,
                transaction_pool_size_before: None,
                transaction_pool_idle_before: None,
                transaction_pool_in_use_before: None,
                ride_query_us: None,
                pending_status_query_us: None,
                latest_status_query_us: None,
                fare_query_us: None,
                chair_query_us: None,
                chair_stats_query_us: None,
                user_query_us: None,
                sent_update_us: None,
                commit_us: None,
                transaction_connection_owned_us: None,
                connection_owned_us: None,
                response_us: None,
                total_us: 0,
                path: "unknown",
                cache_insert_attempted: false,
                outcome: "error_or_cancelled",
                terminal_phase: "cache_lookup",
            },
            force_emit: false,
            emitted: false,
        })
    }

    pub(crate) fn trace_ride_event(
        &mut self,
        ride_id: &str,
        ride_status: &str,
        recipient_id: &str,
    ) {
        if !matches!(ride_status, "PICKUP" | "CARRYING" | "ARRIVED")
            || !crate::drive_diagnostic::should_trace_ride(ride_id)
        {
            return;
        }

        self.force_emit = true;
        self.sample.trace_ride = true;
        self.sample.ride_id = Some(ride_id.to_owned());
        self.sample.ride_status = Some(ride_status.to_owned());
        self.sample.recipient_id = Some(recipient_id.to_owned());
    }

    pub(crate) fn elapsed_since_checkpoint_us(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.checkpoint_at).as_micros();
        self.checkpoint_at = now;
        elapsed.min(u128::from(u64::MAX)) as u64
    }

    pub(crate) fn observe_pool(
        &mut self,
        pool: &sqlx::MySqlPool,
        stage: NotificationConnectionStage,
    ) {
        let size = u64::from(pool.size());
        let idle = u64::try_from(pool.num_idle()).unwrap_or(u64::MAX);
        match stage {
            NotificationConnectionStage::InitialLookup => {
                self.sample.initial_pool_size_before = Some(size);
                self.sample.initial_pool_idle_before = Some(idle);
                self.sample.initial_pool_in_use_before = Some(size.saturating_sub(idle));
            }
            NotificationConnectionStage::Transaction => {
                self.sample.transaction_pool_size_before = Some(size);
                self.sample.transaction_pool_idle_before = Some(idle);
                self.sample.transaction_pool_in_use_before = Some(size.saturating_sub(idle));
            }
        }
    }

    pub(crate) fn connection_acquired(&mut self, stage: NotificationConnectionStage) {
        self.connection_acquired_at = Some(Instant::now());
        self.connection_stage = Some(stage);
    }

    pub(crate) fn connection_released(&mut self) {
        let Some(acquired_at) = self.connection_acquired_at.take() else {
            return;
        };
        let elapsed_us = acquired_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        match self.connection_stage.take() {
            Some(NotificationConnectionStage::InitialLookup) => {
                self.sample.initial_connection_owned_us = Some(elapsed_us);
            }
            Some(NotificationConnectionStage::Transaction) => {
                self.sample.transaction_connection_owned_us = Some(elapsed_us);
            }
            None => {}
        }
        self.sample.connection_owned_us = Some(
            self.sample
                .initial_connection_owned_us
                .unwrap_or_default()
                .saturating_add(
                    self.sample
                        .transaction_connection_owned_us
                        .unwrap_or_default(),
                ),
        );
    }

    fn emit_record(&mut self) {
        self.emitted = true;
        if !self.sample.periodic_sample && !self.force_emit {
            return;
        }
        if self.connection_acquired_at.is_some() {
            self.connection_released();
        }
        self.sample.response_built_at_unix_us = Some(crate::drive_diagnostic::unix_time_us());
        self.sample.total_us = self
            .started_at
            .elapsed()
            .as_micros()
            .min(u128::from(u64::MAX)) as u64;
        crate::drive_diagnostic::emit("NOTIFICATION_DIAGNOSTIC", &self.sample);
    }

    pub(crate) fn emit_success(mut self) {
        self.sample.outcome = "success";
        self.sample.terminal_phase = "complete";
        self.emit_record();
    }
}

impl Drop for NotificationDiagnostic {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_record();
        }
    }
}
