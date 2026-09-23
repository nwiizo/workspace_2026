use std::time::{SystemTime, UNIX_EPOCH};

use tatami_log::db;

#[tokio::test]
async fn persistent_database_can_be_opened_again() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let directory = std::path::Path::new("target/test-databases");
    std::fs::create_dir_all(directory).expect("test database directory should be created");
    let path = directory.join(format!("restart-{}-{nonce}.db", std::process::id()));
    let url = format!("sqlite:{}", path.display());

    let first = db::connect(&url)
        .await
        .expect("new persistent database should open");
    drop(first);
    let reopened = db::connect(&url).await;

    assert!(
        reopened.is_ok(),
        "existing database should reopen: {reopened:?}"
    );
    drop(reopened);
    std::fs::remove_file(path).expect("test database should be removable");
}
