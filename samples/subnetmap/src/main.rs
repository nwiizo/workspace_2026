#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::postgres::PgPoolOptions;

    use subnetmap::app::{shell, App};

    dotenvy::dotenv().ok();

    if let Err(e) = run_server().await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }

    async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://subnetmap:subnetmap@localhost:5433/subnetmap".to_string()
        });

        let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(&database_url)
            .await
            .map_err(|e| format!("failed to connect to database: {e}"))?;

        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|e| format!("failed to run migrations: {e}"))?;

        let conf = get_configuration(None)
            .map_err(|e| format!("failed to get leptos configuration: {e}"))?;
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(App);

        let app = Router::new()
            .leptos_routes_with_context(
                &leptos_options,
                routes,
                {
                    let pool = pool.clone();
                    move || {
                        leptos::context::provide_context(pool.clone());
                    }
                },
                {
                    let leptos_options = leptos_options.clone();
                    move || shell(leptos_options.clone())
                },
            )
            .fallback(leptos_axum::file_and_error_handler(shell))
            .with_state(leptos_options);

        log!("listening on http://{}", &addr);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("failed to bind to {addr}: {e}"))?;
        axum::serve(listener, app.into_make_service())
            .await
            .map_err(|e| format!("server error: {e}"))?;

        Ok(())
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
