use std::io::Write as _;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, OnceLock,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TRACE_RIDE_BUCKETS: u64 = 32;
const DIAGNOSTIC_QUEUE_CAPACITY: usize = 16_384;
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

static DRIVE_DIAGNOSTICS_ENABLED: OnceLock<bool> = OnceLock::new();
static DIAGNOSTIC_SENDER: OnceLock<Option<mpsc::SyncSender<DiagnosticMessage>>> = OnceLock::new();
static DIAGNOSTIC_DROPPED_LINES: AtomicU64 = AtomicU64::new(0);

enum DiagnosticMessage {
    Line(String),
    Flush(mpsc::SyncSender<u64>),
}

pub(crate) fn enabled() -> bool {
    *DRIVE_DIAGNOSTICS_ENABLED.get_or_init(|| {
        std::env::var_os("ISUCON_DIAGNOSTIC").as_deref() == Some(std::ffi::OsStr::new("1"))
    })
}

fn ride_bucket(ride_id: &str) -> u64 {
    ride_id
        .as_bytes()
        .iter()
        .fold(FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
        })
        % TRACE_RIDE_BUCKETS
}

pub(crate) fn should_trace_ride(ride_id: &str) -> bool {
    enabled() && ride_bucket(ride_id) == 0
}

pub(crate) fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn unix_time_us() -> u64 {
    duration_us(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn diagnostic_sender() -> Option<&'static mpsc::SyncSender<DiagnosticMessage>> {
    DIAGNOSTIC_SENDER
        .get_or_init(|| {
            let (sender, receiver) =
                mpsc::sync_channel::<DiagnosticMessage>(DIAGNOSTIC_QUEUE_CAPACITY);
            std::thread::Builder::new()
                .name("diagnostic-writer".to_owned())
                .spawn(move || {
                    let stdout = std::io::stdout();
                    for message in receiver {
                        match message {
                            DiagnosticMessage::Line(line) => {
                                // Do not hold the global stdout lock while waiting on
                                // the channel. The regular tracing subscriber also
                                // writes to stdout and may run on a Tokio worker.
                                let mut output = stdout.lock();
                                let _ = writeln!(output, "{line}");
                                let _ = output.flush();
                            }
                            DiagnosticMessage::Flush(acknowledgement) => {
                                let mut output = stdout.lock();
                                let _ = output.flush();
                                let _ = acknowledgement
                                    .send(DIAGNOSTIC_DROPPED_LINES.load(Ordering::Relaxed));
                            }
                        }
                    }
                })
                .ok()
                .map(|_| sender)
        })
        .as_ref()
}

pub(crate) fn emit<T: serde::Serialize>(prefix: &str, sample: &T) {
    let Ok(json) = serde_json::to_string(sample) else {
        return;
    };
    let line = format!("{prefix} {json}");
    let Some(sender) = diagnostic_sender() else {
        DIAGNOSTIC_DROPPED_LINES.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if sender.try_send(DiagnosticMessage::Line(line)).is_err() {
        DIAGNOSTIC_DROPPED_LINES.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) async fn flush() -> std::io::Result<u64> {
    if !enabled() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "drive diagnostics are disabled",
        ));
    }
    let sender = diagnostic_sender()
        .ok_or_else(|| std::io::Error::other("diagnostic writer is unavailable"))?
        .clone();
    tokio::task::spawn_blocking(move || {
        let (acknowledgement, response) = mpsc::sync_channel(0);
        sender
            .send(DiagnosticMessage::Flush(acknowledgement))
            .map_err(|_| std::io::Error::other("diagnostic writer stopped"))?;
        response
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))
    })
    .await
    .map_err(std::io::Error::other)?
}

#[cfg(test)]
mod tests {
    use super::{ride_bucket, TRACE_RIDE_BUCKETS};

    #[test]
    fn ride_bucket_is_stable() {
        assert_eq!(ride_bucket("01JTEST0000000000000000000"), 12);
        assert_eq!(ride_bucket("01JTEST0000000000000000001"), 31);
    }

    #[test]
    fn ride_bucket_distributes_sequential_ids() {
        let selected = (0..3_200)
            .filter(|number| ride_bucket(&format!("ride-{number:04}")) == 0)
            .count();

        assert!(
            (70..=130).contains(&selected),
            "selected={selected}, buckets={TRACE_RIDE_BUCKETS}"
        );
    }
}
