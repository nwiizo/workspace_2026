use std::error::Error;

use tokio::net::TcpListener;
use verus_discount_service::app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:3001").await?;
    axum::serve(listener, app()).await?;
    Ok(())
}
