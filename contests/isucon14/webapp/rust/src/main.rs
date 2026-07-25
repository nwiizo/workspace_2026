use anyhow::Context;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use isuride::chair_handlers::CoordinateWriteQueue;
use isuride::{
    ensure_chair_stats, ActiveRideEvaluationTracker, AppState, AuthCache, DbAdmission, Error,
    LatestChairLocationCache, NotificationCache,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{Duration, MissedTickBehavior};

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 50;
const DEFAULT_DB_COORDINATE_CONNECTIONS: u32 = 24;
const DEFAULT_COORDINATE_QUEUE_CAPACITY_PER_SHARD: u32 = 64;

#[derive(Debug, PartialEq, Eq)]
struct DbPoolLimits {
    total: u32,
    general: u32,
    coordinate: u32,
    shared: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct CoordinateQueueConfig {
    shards: usize,
    capacity_per_shard: usize,
    max_in_flight: usize,
}

fn parse_positive_connection_limit(
    name: &str,
    value: Option<&str>,
    default: u32,
) -> anyhow::Result<u32> {
    let Some(value) = value else {
        return Ok(default);
    };
    let max_connections = value
        .parse::<u32>()
        .with_context(|| format!("{name} must be a positive integer: {value}"))?;
    anyhow::ensure!(max_connections > 0, "{name} must be greater than zero");
    Ok(max_connections)
}

fn parse_db_pool_limits(
    total: Option<&str>,
    coordinate: Option<&str>,
    general_permits: Option<&str>,
) -> anyhow::Result<DbPoolLimits> {
    let total = parse_positive_connection_limit(
        "ISUCON_DB_MAX_CONNECTIONS",
        total,
        DEFAULT_DB_MAX_CONNECTIONS,
    )?;
    if let Some(general_permits) = general_permits {
        anyhow::ensure!(
            coordinate.is_none(),
            "ISUCON_DB_COORDINATE_CONNECTIONS and ISUCON_DB_GENERAL_PERMITS cannot be set together"
        );
        let general =
            parse_positive_connection_limit("ISUCON_DB_GENERAL_PERMITS", Some(general_permits), 1)?;
        anyhow::ensure!(
            general < total,
            "ISUCON_DB_GENERAL_PERMITS ({general}) must be smaller than ISUCON_DB_MAX_CONNECTIONS ({total})"
        );
        return Ok(DbPoolLimits {
            total,
            general,
            coordinate: total - general,
            shared: true,
        });
    }

    let coordinate = match coordinate {
        Some(value) => {
            let coordinate = parse_positive_connection_limit(
                "ISUCON_DB_COORDINATE_CONNECTIONS",
                Some(value),
                DEFAULT_DB_COORDINATE_CONNECTIONS,
            )?;
            anyhow::ensure!(
                coordinate < total,
                "ISUCON_DB_COORDINATE_CONNECTIONS ({coordinate}) must be smaller than ISUCON_DB_MAX_CONNECTIONS ({total})"
            );
            return Ok(DbPoolLimits {
                total,
                general: total - coordinate,
                coordinate,
                shared: false,
            });
        }
        None => {
            let derived = DEFAULT_DB_COORDINATE_CONNECTIONS.min(total / 2);
            anyhow::ensure!(
                derived > 0,
                "ISUCON_DB_MAX_CONNECTIONS ({total}) must be at least 2 when database admission reserves coordinate headroom"
            );
            derived
        }
    };
    Ok(DbPoolLimits {
        total,
        general: total - coordinate,
        coordinate,
        shared: true,
    })
}

fn parse_coordinate_queue_config(
    shards: Option<&str>,
    capacity_per_shard: Option<&str>,
    max_in_flight: Option<&str>,
    default_max_in_flight: u32,
) -> anyhow::Result<Option<CoordinateQueueConfig>> {
    let Some(shards) = shards.filter(|value| !value.is_empty() && *value != "0") else {
        anyhow::ensure!(
            capacity_per_shard.is_none_or(|value| value.is_empty()),
            "ISUCON_COORDINATE_QUEUE_CAPACITY requires ISUCON_COORDINATE_QUEUE_SHARDS"
        );
        anyhow::ensure!(
            max_in_flight.is_none_or(|value| value.is_empty()),
            "ISUCON_COORDINATE_QUEUE_MAX_IN_FLIGHT requires ISUCON_COORDINATE_QUEUE_SHARDS"
        );
        return Ok(None);
    };
    let shards =
        parse_positive_connection_limit("ISUCON_COORDINATE_QUEUE_SHARDS", Some(shards), 1)?;
    let capacity_per_shard = parse_positive_connection_limit(
        "ISUCON_COORDINATE_QUEUE_CAPACITY",
        capacity_per_shard.filter(|value| !value.is_empty()),
        DEFAULT_COORDINATE_QUEUE_CAPACITY_PER_SHARD,
    )?;
    let max_in_flight = parse_positive_connection_limit(
        "ISUCON_COORDINATE_QUEUE_MAX_IN_FLIGHT",
        max_in_flight.filter(|value| !value.is_empty()),
        default_max_in_flight,
    )?;
    anyhow::ensure!(
        max_in_flight <= shards,
        "ISUCON_COORDINATE_QUEUE_MAX_IN_FLIGHT ({max_in_flight}) must not exceed ISUCON_COORDINATE_QUEUE_SHARDS ({shards})"
    );
    Ok(Some(CoordinateQueueConfig {
        shards: usize::try_from(shards)
            .context("coordinate queue shard count does not fit usize")?,
        capacity_per_shard: usize::try_from(capacity_per_shard)
            .context("coordinate queue capacity does not fit usize")?,
        max_in_flight: usize::try_from(max_in_flight)
            .context("coordinate queue in-flight limit does not fit usize")?,
    }))
}

fn optional_env(name: &str) -> anyhow::Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => {
            Err(anyhow::Error::new(error).context(format!("{name} must contain valid Unicode")))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info,tower_http=debug,axum::rejection=trace");
    }
    tracing_subscriber::fmt::init();

    let host = std::env::var("ISUCON_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = std::env::var("ISUCON_DB_PORT")
        .map(|port_str| {
            port_str.parse().expect(
                "failed to convert DB port number from ISUCON_DB_PORT environment variable into u16",
            )
        })
        .unwrap_or(3306);
    let user = std::env::var("ISUCON_DB_USER").unwrap_or_else(|_| "isucon".to_owned());
    let password = std::env::var("ISUCON_DB_PASSWORD").unwrap_or_else(|_| "isucon".to_owned());
    let dbname = std::env::var("ISUCON_DB_NAME").unwrap_or_else(|_| "isuride".to_owned());
    let db_max_connections = optional_env("ISUCON_DB_MAX_CONNECTIONS")?;
    let db_coordinate_connections = optional_env("ISUCON_DB_COORDINATE_CONNECTIONS")?;
    let db_general_permits = optional_env("ISUCON_DB_GENERAL_PERMITS")?;
    let coordinate_queue_shards = optional_env("ISUCON_COORDINATE_QUEUE_SHARDS")?;
    let coordinate_queue_capacity = optional_env("ISUCON_COORDINATE_QUEUE_CAPACITY")?;
    let coordinate_queue_max_in_flight = optional_env("ISUCON_COORDINATE_QUEUE_MAX_IN_FLIGHT")?;
    let db_pool_limits = parse_db_pool_limits(
        db_max_connections.as_deref(),
        db_coordinate_connections
            .as_deref()
            .filter(|value| !value.is_empty()),
        db_general_permits
            .as_deref()
            .filter(|value| !value.is_empty()),
    )?;
    let coordinate_queue_config = parse_coordinate_queue_config(
        coordinate_queue_shards.as_deref(),
        coordinate_queue_capacity.as_deref(),
        coordinate_queue_max_in_flight.as_deref(),
        db_pool_limits.coordinate,
    )?;

    let connect_options = sqlx::mysql::MySqlConnectOptions::default()
        .host(&host)
        .port(port)
        .username(&user)
        .password(&password)
        .database(&dbname);
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(if db_pool_limits.shared {
            db_pool_limits.total
        } else {
            db_pool_limits.general
        })
        .connect_with(connect_options.clone())
        .await?;
    let (coordinate_pool, general_db_admission) = if db_pool_limits.shared {
        (
            pool.clone(),
            DbAdmission::limited(
                usize::try_from(db_pool_limits.general)
                    .context("general database permit count does not fit usize")?,
            ),
        )
    } else {
        (
            sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(db_pool_limits.coordinate)
                .connect_with(connect_options)
                .await?,
            DbAdmission::default(),
        )
    };
    tracing::info!(
        total = db_pool_limits.total,
        general = db_pool_limits.general,
        coordinate = db_pool_limits.coordinate,
        shared = db_pool_limits.shared,
        "configured database connection pools"
    );

    let auth_cache = AuthCache::load(&pool)
        .await
        .context("failed to load authentication cache")?;
    let latest_chair_locations = LatestChairLocationCache::load(&pool)
        .await
        .context("failed to load latest chair locations")?;
    ensure_chair_stats(&pool)
        .await
        .context("failed to load chair stats")?;
    let notification_cache = NotificationCache::default();
    let maintenance_lock = Arc::new(RwLock::new(()));
    let coordinate_write_queue = coordinate_queue_config.map(|config| {
        CoordinateWriteQueue::spawn(
            config.shards,
            config.capacity_per_shard,
            config.max_in_flight,
            coordinate_pool.clone(),
            latest_chair_locations.clone(),
            notification_cache.clone(),
            maintenance_lock.clone(),
        )
    });
    let app_state = AppState {
        pool,
        coordinate_pool,
        payment_client: reqwest::Client::builder()
            .build()
            .context("failed to initialize payment HTTP client")?,
        auth_cache,
        notification_cache,
        latest_chair_locations,
        coordinate_write_queue,
        active_ride_evaluations: ActiveRideEvaluationTracker::default(),
        maintenance_lock,
        general_db_admission,
    };

    spawn_latest_chair_location_reconciliation(&app_state);

    let api_routes = axum::Router::new()
        .merge(isuride::app_handlers::app_routes(app_state.clone()))
        .merge(isuride::owner_handlers::owner_routes(app_state.clone()))
        .merge(isuride::chair_handlers::chair_routes(app_state.clone()))
        .merge(isuride::internal_handlers::internal_routes())
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            wait_for_maintenance,
        ));
    let app = axum::Router::new()
        .route("/api/initialize", axum::routing::post(post_initialize))
        .merge(api_routes)
        .with_state(app_state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let tcp_listener =
        if let Some(std_listener) = listenfd::ListenFd::from_env().take_tcp_listener(0)? {
            TcpListener::from_std(std_listener)?
        } else {
            TcpListener::bind(&SocketAddr::from(([0, 0, 0, 0], 8080))).await?
        };
    axum::serve(tcp_listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        parse_coordinate_queue_config, parse_db_pool_limits, CoordinateQueueConfig, DbPoolLimits,
    };

    #[test]
    fn db_pool_limits_default_to_shared_admission_with_coordinate_headroom() {
        assert_eq!(
            parse_db_pool_limits(None, None, None).expect("default pool limits"),
            DbPoolLimits {
                total: 50,
                general: 26,
                coordinate: 24,
                shared: true,
            }
        );
    }

    #[test]
    fn db_pool_limits_split_the_configured_total() {
        assert_eq!(
            parse_db_pool_limits(Some("50"), Some("16"), None).expect("configured pool limits"),
            DbPoolLimits {
                total: 50,
                general: 34,
                coordinate: 16,
                shared: false,
            }
        );
    }

    #[test]
    fn coordinate_headroom_scales_down_with_a_small_total() {
        assert_eq!(
            parse_db_pool_limits(Some("16"), None, None).expect("derived pool limits"),
            DbPoolLimits {
                total: 16,
                general: 8,
                coordinate: 8,
                shared: true,
            }
        );
        assert_eq!(
            parse_db_pool_limits(Some("2"), None, None).expect("minimum admission limits"),
            DbPoolLimits {
                total: 2,
                general: 1,
                coordinate: 1,
                shared: true,
            }
        );
        assert!(parse_db_pool_limits(Some("1"), None, None).is_err());
    }

    #[test]
    fn db_pool_limits_reject_zero_and_non_numbers() {
        assert!(parse_db_pool_limits(Some("0"), None, None).is_err());
        assert!(parse_db_pool_limits(Some(""), None, None).is_err());
        assert!(parse_db_pool_limits(Some("many"), None, None).is_err());
        assert!(parse_db_pool_limits(None, Some("0"), None).is_err());
        assert!(parse_db_pool_limits(None, Some(""), None).is_err());
        assert!(parse_db_pool_limits(None, Some("many"), None).is_err());
        assert!(parse_db_pool_limits(None, None, Some("0")).is_err());
        assert!(parse_db_pool_limits(None, None, Some("")).is_err());
        assert!(parse_db_pool_limits(None, None, Some("many")).is_err());
    }

    #[test]
    fn coordinate_pool_must_leave_at_least_one_general_connection() {
        assert!(parse_db_pool_limits(Some("16"), Some("16"), None).is_err());
        assert!(parse_db_pool_limits(Some("15"), Some("16"), None).is_err());
    }

    #[test]
    fn shared_pool_uses_general_permits_with_coordinate_headroom() {
        assert_eq!(
            parse_db_pool_limits(Some("50"), None, Some("26"))
                .expect("shared pool admission limits"),
            DbPoolLimits {
                total: 50,
                general: 26,
                coordinate: 24,
                shared: true,
            }
        );
        assert!(parse_db_pool_limits(Some("50"), None, Some("50")).is_err());
        assert!(parse_db_pool_limits(Some("50"), Some("24"), Some("26")).is_err());
    }

    #[test]
    fn coordinate_queue_is_opt_in_and_bounded() {
        assert_eq!(
            parse_coordinate_queue_config(None, None, None, 24).unwrap(),
            None
        );
        assert_eq!(
            parse_coordinate_queue_config(Some(""), None, None, 24).unwrap(),
            None
        );
        assert_eq!(
            parse_coordinate_queue_config(Some("0"), None, None, 24).unwrap(),
            None
        );
        assert_eq!(
            parse_coordinate_queue_config(Some("128"), None, None, 24).unwrap(),
            Some(CoordinateQueueConfig {
                shards: 128,
                capacity_per_shard: 64,
                max_in_flight: 24,
            })
        );
        assert_eq!(
            parse_coordinate_queue_config(Some("64"), Some("32"), Some("16"), 24).unwrap(),
            Some(CoordinateQueueConfig {
                shards: 64,
                capacity_per_shard: 32,
                max_in_flight: 16,
            })
        );
    }

    #[test]
    fn coordinate_queue_rejects_invalid_or_orphan_capacity() {
        assert!(parse_coordinate_queue_config(None, Some("64"), None, 24).is_err());
        assert!(parse_coordinate_queue_config(None, None, Some("24"), 24).is_err());
        assert!(parse_coordinate_queue_config(Some("0"), Some("64"), None, 24).is_err());
        assert!(parse_coordinate_queue_config(Some("many"), None, None, 24).is_err());
        assert!(parse_coordinate_queue_config(Some("64"), Some("0"), None, 24).is_err());
        assert!(parse_coordinate_queue_config(Some("64"), Some("many"), None, 24).is_err());
        assert!(parse_coordinate_queue_config(Some("64"), None, Some("0"), 24).is_err());
        assert!(parse_coordinate_queue_config(Some("64"), None, Some("many"), 24).is_err());
        assert!(parse_coordinate_queue_config(Some("16"), None, Some("24"), 24).is_err());
    }
}

fn spawn_latest_chair_location_reconciliation(app_state: &AppState) {
    let pool = app_state.pool.clone();
    let latest_chair_locations = app_state.latest_chair_locations.clone();
    let maintenance_lock = app_state.maintenance_lock.clone();
    let general_db_admission = app_state.general_db_admission.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            interval.tick().await;
            let _maintenance_guard = maintenance_lock.read().await;
            let _admission_guard = general_db_admission
                .acquire("latest_location_reconcile", &pool)
                .await;
            if let Err(error) = latest_chair_locations.reconcile(&pool).await {
                tracing::warn!(%error, "failed to reconcile latest chair locations");
            }
        }
    });
}

#[derive(Debug, serde::Deserialize)]
struct PostInitializeRequest {
    payment_server: String,
}

#[derive(Debug, serde::Serialize)]
struct PostInitializeResponse {
    language: &'static str,
}

async fn post_initialize(
    State(AppState {
        pool,
        auth_cache,
        notification_cache,
        latest_chair_locations,
        active_ride_evaluations,
        coordinate_write_queue,
        maintenance_lock,
        general_db_admission,
        ..
    }): State<AppState>,
    axum::Json(req): axum::Json<PostInitializeRequest>,
) -> Result<axum::Json<PostInitializeResponse>, Error> {
    // Wait for in-flight API requests and keep new requests/reconciliation out
    // while init.sh drops and recreates tables. This also prevents a coordinate
    // request from observing an old cache with an empty current-state table.
    let _maintenance_guard = maintenance_lock.write().await;
    if let Some(queue) = coordinate_write_queue {
        queue.advance_generation();
    }
    // Invalidate process-local state before the first destructive initialization
    // step. If init.sh or a later refresh fails, requests must neither
    // authenticate a token nor retain an evaluation lease from the previous
    // database generation.
    auth_cache.clear();
    notification_cache.clear();
    active_ride_evaluations.clear();
    let output = tokio::process::Command::new("../sql/init.sh")
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::Initialize {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let _admission_guard = general_db_admission
        .acquire("initialize_refresh", &pool)
        .await;
    sqlx::query("UPDATE settings SET value = ? WHERE name = 'payment_gateway_url'")
        .bind(req.payment_server)
        .execute(&pool)
        .await?;
    auth_cache.refresh(&pool).await?;
    latest_chair_locations.refresh(&pool).await?;

    Ok(axum::Json(PostInitializeResponse { language: "rust" }))
}

async fn wait_for_maintenance(
    State(AppState {
        maintenance_lock, ..
    }): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let _maintenance_guard = maintenance_lock.read().await;
    next.run(request).await
}
