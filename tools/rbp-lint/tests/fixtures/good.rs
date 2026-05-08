use std::sync::Arc;

// reason: kept for the v2 API rollout (issue #123)
#[allow(dead_code)]
fn future_api() {}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string("config.toml")?;
    let v: i32 = "5".parse().map_err(|e| format!("not a number: {e}"))?;

    let arc = Arc::new(42);
    let _ = Arc::clone(&arc);

    tracing::info!(user = %raw, "loaded user from db");

    // SAFETY: the null pointer is only dereferenced via a checked path.
    unsafe {
        let _ = std::ptr::null::<u8>();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn unwrap_ok_in_tests() {
        let v: i32 = "5".parse().unwrap();
        assert_eq!(v, 5);
    }
}
