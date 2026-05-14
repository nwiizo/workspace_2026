//! hot-lib-reloader 検証用アプリケーション。
//!
//! # 実行方法
//!
//! ## ホットリロードモード（デフォルト）
//! ```sh
//! # ターミナル1: lib を監視して自動リビルド
//! cargo watch -w part1-hot-lib-reloader/lib -x 'build -p hot-lib'
//!
//! # ターミナル2: アプリを実行
//! cargo run -p hot-app
//! ```
//!
//! ## 静的リンクモード（リロードなし）
//! ```sh
//! cargo run -p hot-app --no-default-features
//! ```

// --- ホットリロード有効時: hot_module 属性マクロで dylib をラップ ---
#[cfg(feature = "reload")]
#[hot_lib_reloader::hot_module(
    dylib = "hot_lib",
    lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug"),
    file_watch_debounce = 300
)]
mod hot_lib {
    pub use hot_lib::State;

    #[hot_functions]
    extern "Rust" {
        pub fn step(state: &mut State);
        pub fn render(state: &State) -> String;
        pub fn step_serialized(state_json: &str) -> String;
    }

    #[lib_change_subscription]
    pub fn subscribe() -> hot_lib_reloader::LibReloadObserver {}
}

fn main() {
    println!("=== hot-lib-reloader demo ===");
    #[cfg(feature = "reload")]
    println!("Mode: HOT RELOAD enabled (change lib/src/lib.rs and save)");
    #[cfg(not(feature = "reload"))]
    println!("Mode: STATIC (no hot-reload)");
    println!();

    let mut state = hot_lib::State::new();

    // --- ホットリロードモード ---
    #[cfg(feature = "reload")]
    {
        let observer = hot_lib::subscribe();
        let mut iteration = 0u64;
        loop {
            iteration += 1;
            hot_lib::step(&mut state);
            let rendered = hot_lib::render(&state);
            println!("[iter {}] {}", iteration, rendered);

            println!(
                "[iter {}] Waiting for lib change... (Ctrl+C to exit)",
                iteration
            );
            observer.wait_for_reload();
            println!("  -> reloaded!");
        }
    }

    // --- 静的リンクモード ---
    #[cfg(not(feature = "reload"))]
    {
        for iteration in 1..=5 {
            hot_lib::step(&mut state);
            let rendered = hot_lib::render(&state);
            println!("[iter {}] {}", iteration, rendered);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        println!("(static mode: stopping after 5 iterations)");
    }
}
