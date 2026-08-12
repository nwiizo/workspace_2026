use tatami_log::{db, web};
use topcoat::asset::AssetBundle;

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(value) => value,
        Err(_) => "sqlite:tatami-log.db".to_owned(),
    };
    let database = db::connect(&database_url).await?;
    let assets = AssetBundle::load()?;

    topcoat::start(web::router(database, Some(assets))).await?;
    Ok(())
}
