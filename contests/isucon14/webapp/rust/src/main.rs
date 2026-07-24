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

fn parse_db_max_connections(value: Option<&str>) -> anyhow::Result<u32> {
    let Some(value) = value else {
        return Ok(DEFAULT_DB_MAX_CONNECTIONS);
    };
    let max_connections = value.parse::<u32>().with_context(|| {
        format!("ISUCON_DB_MAX_CONNECTIONS must be a positive integer: {value}")
    })?;
    anyhow::ensure!(
        max_connections > 0,
        "ISUCON_DB_MAX_CONNECTIONS must be greater than zero"
    );
    Ok(max_connections)
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
    let db_max_connections = match std::env::var("ISUCON_DB_MAX_CONNECTIONS") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(anyhow::Error::new(error)
                .context("ISUCON_DB_MAX_CONNECTIONS must contain valid Unicode"));
        }
    };
    let db_max_connections = parse_db_max_connections(db_max_connections.as_deref())?;

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(db_max_connections)
        .connect_with(
            sqlx::mysql::MySqlConnectOptions::default()
                .host(&host)
                .port(port)
                .username(&user)
                .password(&password)
                .database(&dbname),
        )
        .await?;

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
    use super::{parse_db_max_connections, DEFAULT_DB_MAX_CONNECTIONS};

    #[test]
    fn db_max_connections_defaults_to_fifty() {
        assert_eq!(
            parse_db_max_connections(None).expect("default pool size"),
            DEFAULT_DB_MAX_CONNECTIONS
        );
    }

    #[test]
    fn db_max_connections_accepts_a_positive_integer() {
        assert_eq!(
            parse_db_max_connections(Some("75")).expect("configured pool size"),
            75
        );
    }

    #[test]
    fn db_max_connections_rejects_zero_and_non_numbers() {
        assert!(parse_db_max_connections(Some("0")).is_err());
        assert!(parse_db_max_connections(Some("")).is_err());
        assert!(parse_db_max_connections(Some("many")).is_err());
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
