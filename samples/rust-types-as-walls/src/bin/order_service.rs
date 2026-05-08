#![allow(
    clippy::print_stdout,
    reason = "the demo binary is expected to log its listening address"
)]

use std::net::{Ipv4Addr, SocketAddr};

use rust_types_as_walls::order_service;
use tokio::net::TcpListener;

fn port_from_env() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = order_service::app().await?;
    let port = port_from_env();
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))).await?;

    println!("order_service listening on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}
