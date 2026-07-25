use anyhow::Context;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use isuride::{
    ensure_chair_stats, ActiveRideEvaluationTracker, AppState, AuthCache, Error,
    LatestChairLocationCache, NotificationCache,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{Duration, MissedTickBehavior};

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 50;
const DEFAULT_DB_COORDINATE_CONNECTIONS: u32 = 24;

#[derive(Debug, PartialEq, Eq)]
struct DbPoolLimits {
    total: u32,
    general: u32,
    coordinate: u32,
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
) -> anyhow::Result<DbPoolLimits> {
    let total = parse_positive_connection_limit(
        "ISUCON_DB_MAX_CONNECTIONS",
        total,
        DEFAULT_DB_MAX_CONNECTIONS,
    )?;
    let coordinate = match coordinate {
        Some(value) => parse_positive_connection_limit(
            "ISUCON_DB_COORDINATE_CONNECTIONS",
            Some(value),
            DEFAULT_DB_COORDINATE_CONNECTIONS,
        )?,
        None => {
            let derived = DEFAULT_DB_COORDINATE_CONNECTIONS.min(total / 2);
            anyhow::ensure!(
                derived > 0,
                "ISUCON_DB_MAX_CONNECTIONS ({total}) must be at least 2 when database pools are partitioned"
            );
            derived
        }
    };
    anyhow::ensure!(
        coordinate < total,
        "ISUCON_DB_COORDINATE_CONNECTIONS ({coordinate}) must be smaller than ISUCON_DB_MAX_CONNECTIONS ({total})"
    );
    Ok(DbPoolLimits {
        total,
        general: total - coordinate,
        coordinate,
    })
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
    let db_pool_limits = parse_db_pool_limits(
        db_max_connections.as_deref(),
        db_coordinate_connections
            .as_deref()
            .filter(|value| !value.is_empty()),
    )?;

    let connect_options = sqlx::mysql::MySqlConnectOptions::default()
        .host(&host)
        .port(port)
        .username(&user)
        .password(&password)
        .database(&dbname);
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(db_pool_limits.general)
        .connect_with(connect_options.clone())
        .await?;
    let coordinate_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(db_pool_limits.coordinate)
        .connect_with(connect_options)
        .await?;
    tracing::info!(
        total = db_pool_limits.total,
        general = db_pool_limits.general,
        coordinate = db_pool_limits.coordinate,
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
    let app_state = AppState {
        pool,
        coordinate_pool,
        payment_client: reqwest::Client::builder()
            .build()
            .context("failed to initialize payment HTTP client")?,
        auth_cache,
        notification_cache: NotificationCache::default(),
        latest_chair_locations,
        active_ride_evaluations: ActiveRideEvaluationTracker::default(),
        maintenance_lock: Arc::new(RwLock::new(())),
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
    use super::{parse_db_pool_limits, DbPoolLimits};

    #[test]
    fn db_pool_limits_default_to_a_fifty_connection_budget() {
        assert_eq!(
            parse_db_pool_limits(None, None).expect("default pool limits"),
            DbPoolLimits {
                total: 50,
                general: 26,
                coordinate: 24,
            }
        );
    }

    #[test]
    fn db_pool_limits_split_the_configured_total() {
        assert_eq!(
            parse_db_pool_limits(Some("50"), Some("16")).expect("configured pool limits"),
            DbPoolLimits {
                total: 50,
                general: 34,
                coordinate: 16,
            }
        );
    }

    #[test]
    fn coordinate_default_scales_down_with_a_small_total() {
        assert_eq!(
            parse_db_pool_limits(Some("16"), None).expect("derived pool limits"),
            DbPoolLimits {
                total: 16,
                general: 8,
                coordinate: 8,
            }
        );
        assert_eq!(
            parse_db_pool_limits(Some("2"), None).expect("minimum split pool limits"),
            DbPoolLimits {
                total: 2,
                general: 1,
                coordinate: 1,
            }
        );
        assert!(parse_db_pool_limits(Some("1"), None).is_err());
    }

    #[test]
    fn db_pool_limits_reject_zero_and_non_numbers() {
        assert!(parse_db_pool_limits(Some("0"), None).is_err());
        assert!(parse_db_pool_limits(Some(""), None).is_err());
        assert!(parse_db_pool_limits(Some("many"), None).is_err());
        assert!(parse_db_pool_limits(None, Some("0")).is_err());
        assert!(parse_db_pool_limits(None, Some("")).is_err());
        assert!(parse_db_pool_limits(None, Some("many")).is_err());
    }

    #[test]
    fn coordinate_pool_must_leave_at_least_one_general_connection() {
        assert!(parse_db_pool_limits(Some("16"), Some("16")).is_err());
        assert!(parse_db_pool_limits(Some("15"), Some("16")).is_err());
    }
}

fn spawn_latest_chair_location_reconciliation(app_state: &AppState) {
    let pool = app_state.pool.clone();
    let latest_chair_locations = app_state.latest_chair_locations.clone();
    let maintenance_lock = app_state.maintenance_lock.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            interval.tick().await;
            let _maintenance_guard = maintenance_lock.read().await;
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
        maintenance_lock,
        ..
    }): State<AppState>,
    axum::Json(req): axum::Json<PostInitializeRequest>,
) -> Result<axum::Json<PostInitializeResponse>, Error> {
    // Wait for in-flight API requests and keep new requests/reconciliation out
    // while init.sh drops and recreates tables. This also prevents a coordinate
    // request from observing an old cache with an empty current-state table.
    let _maintenance_guard = maintenance_lock.write().await;
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
