use toasty::Db;

#[derive(Debug, Clone, toasty::Model)]
struct Note {
    #[key]
    #[auto]
    id: u64,

    body: String,
}

async fn connect(url: &str) -> toasty::Result<Db> {
    let should_initialize = should_initialize(url);
    let db = Db::builder()
        .models(toasty::models!(Note))
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

#[tokio::main]
async fn main() -> toasty::Result<()> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:tip.db".to_owned());
    let _db = connect(&url).await?;
    println!("opened {url}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn reopens_a_persistent_database() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let directory = std::path::Path::new("target/test-databases");
        std::fs::create_dir_all(directory).expect("test directory should be created");
        let path = directory.join(format!("tip-{}-{nonce}.db", std::process::id()));
        let url = format!("sqlite:{}", path.display());

        let first = connect(&url).await.expect("first open should succeed");
        drop(first);
        let reopened = connect(&url).await;

        assert!(reopened.is_ok(), "database should reopen: {reopened:?}");
        drop(reopened);
        std::fs::remove_file(path).expect("test database should be removable");
    }
}
