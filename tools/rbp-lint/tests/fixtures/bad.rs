use std::sync::Arc;

const API_KEY: &str = "sk-1234567890abcdef";

#[allow(dead_code)]
fn unjustified() {}

pub struct UserId(pub String);

pub struct Order {
    pub id: u64,
    pub user_id: String,
    pub status: String,
    pub is_paid: bool,
    pub payment_id: Option<u64>,
}

pub enum LoadError {
    Empty,
}

lazy_static::lazy_static! {
    static ref FOO: u32 = 42;
}

fn run() -> Result<(), String> {
    let raw = std::fs::read_to_string("config.toml").unwrap();
    let v: i32 = "5".parse().expect("");
    panic!("boom");

    let arc = Arc::new(42);
    let _ = arc.clone();

    tracing::info!("loaded user {} from db", raw);

    println!("debug: {raw}");

    let _opt: Option<i32> = None;
    let _x = _opt.unwrap_or(Default::default());

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<i32>();

    if v < 0 {
        return Err("negative".to_string());
    }

    let maybe: Option<i32> = Some(1);
    if let Some(n) = maybe {
        let _ = n + 1;
    } else {
        return Err("missing".to_string());
    }

    unsafe {
        let _ = std::ptr::null::<u8>();
    }

    return Ok(());
}
