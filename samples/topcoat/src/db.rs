use toasty::Db;

use crate::models::{AuthSession, Review, TechniqueCard, TrainingSession, User};

/// Opens a Toasty database and applies the sample schema.
///
/// # Errors
///
/// Returns a Toasty error when the connection or schema push fails.
pub async fn connect(url: &str) -> toasty::Result<Db> {
    let db = Db::builder()
        .models(toasty::models!(
            User,
            AuthSession,
            TrainingSession,
            TechniqueCard,
            Review
        ))
        .connect(url)
        .await?;

    db.push_schema().await?;
    Ok(db)
}
