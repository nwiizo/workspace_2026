#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::postgres::PgPoolOptions;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::PostgresStore;

    use chirp::app::{shell, App};

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://chirp:chirp@localhost:5432/chirp".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let session_store = PostgresStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to migrate session store");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    let conf = get_configuration(None).expect("Failed to get leptos configuration");
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
        .layer(session_layer)
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Server error");
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
}
