use toasty::Db;

use crate::models::{AuthSession, Review, TechniqueCard, TrainingSession, User};

/// Opens a Toasty database and applies the sample schema to a new `SQLite` file.
///
/// # Errors
///
/// Returns a Toasty error when the connection or schema push fails.
pub async fn connect(url: &str) -> toasty::Result<Db> {
    let should_initialize = should_initialize(url);
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

    if should_initialize {
        db.push_schema().await?;
    }
    Ok(db)
}

fn should_initialize(url: &str) -> bool {
    let Some(path) = url.strip_prefix("sqlite:") else {
        return true;
    };
    if path == ":memory:" {
        return true;
    }

    match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() == 0,
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}
