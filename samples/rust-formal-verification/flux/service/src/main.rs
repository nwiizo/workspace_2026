use std::error::Error;

use flux_discount_service::app;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:3002").await?;
    axum::serve(listener, app()).await?;
    Ok(())
}
