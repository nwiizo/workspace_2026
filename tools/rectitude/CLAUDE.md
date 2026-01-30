# Rectitude

General-purpose E2E scenario testing library for security testing.

## Quick Reference

- **Edition**: Rust 2024
- **Binary**: `rectitude`
- **Library**: `rectitude`

## Commands

```bash
cargo build --release       # Build
cargo test                  # Run tests
cargo run -- --help         # CLI help
cargo run -- init           # Create config file
cargo run -- list           # List scenarios
cargo run -- payloads sqli  # Generate SQLi payloads

# Run examples
cargo run --example juice_shop_verified
cargo run --example juice_shop_ctf
cargo run --example juice_shop_complete
```

## Architecture

```
src/
├── lib.rs           # Library entry point
├── client.rs        # HTTP client with session management
├── scenario.rs      # Scenario builder and runner
├── ctf.rs           # CTF verification traits (汎用)
├── config.rs        # Configuration file support
├── reporter.rs      # Test report generation
├── error.rs         # Error types
├── extractors.rs    # Response data extraction
├── helpers.rs       # Security testing helpers
├── payloads/        # Security payloads
│   ├── sqli.rs      # SQL Injection
│   ├── xss.rs       # Cross-Site Scripting
│   ├── xxe.rs       # XML External Entity
│   ├── jwt.rs       # JWT manipulation
│   ├── ssrf.rs      # Server-Side Request Forgery
│   ├── nosql.rs     # NoSQL Injection
│   ├── traversal.rs # Path traversal
│   └── encoding.rs  # Encoding utilities
└── bin/main.rs      # CLI application

examples/
├── juice_shop_verified.rs   # JuiceShop with verification
├── juice_shop_ctf.rs        # JuiceShop solver
├── juice_shop_complete.rs   # Additional challenges
└── ...
```

## Usage

### Library

```rust
use rectitude::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Scenario::new("Security Test")
        .base_url("http://localhost:3000")
        .tags(&["sqli", "auth"])
        .step("SQLi Login", |ctx| Box::pin(async move {
            let resp = ctx.post("/api/login")
                .json(&serde_json::json!({
                    "email": "admin@test.com'--",
                    "password": "x"
                }))
                .send()
                .await?;

            ctx.assert_status(&resp, 200)?;
            Ok(StepResult::success())
        }))
        .run()
        .await?;

    Ok(())
}
```

### CTF Verification

```rust
use rectitude::ctf::{ChallengeVerifier, ChallengeProgress};
use async_trait::async_trait;

// Implement for your CTF platform
struct MyCtfVerifier { /* ... */ }

#[async_trait]
impl ChallengeVerifier for MyCtfVerifier {
    async fn is_solved(&self, key: &str) -> rectitude::Result<bool> {
        // Check your CTF API
        Ok(true)
    }
    async fn get_progress(&self) -> rectitude::Result<ChallengeProgress> {
        Ok(ChallengeProgress::default())
    }
}
```

### CLI

```bash
# Initialize config
rectitude init

# Generate payloads
rectitude payloads sqli
rectitude payloads xss
rectitude payloads jwt --unsigned
rectitude payloads ssrf --port 8080

# List/run scenarios
rectitude list
rectitude run --output json
```

---

## 設計原則

### ライブラリは汎用、具体実装はexamples

- `src/` は汎用的なセキュリティテスト機能のみ
- CTF固有の実装（JuiceShopVerifier等）は `examples/` に配置
- ライブラリは「ついでに」特定CTFを解けるが、それが目的ではない

### シナリオテストの検証

**悪い例**: HTTPリクエストの成功だけを確認
```rust
// ❌ これだけでは不十分
let resp = ctx.get("/hidden-page").send().await?;
resp.expect_success()?;  // 200 OK = テスト成功？
```

**良い例**: 実際にチャレンジが解決されたか検証
```rust
// ✅ CTF APIで検証
.step("Attack", |ctx| async move { /* ... */ })
.step("Verify", |_| async move {
    if verifier.is_solved("challengeKey").await? {
        ok_with("VERIFIED")
    } else {
        fail("NOT SOLVED")
    }
})
```

### 進捗追跡

```rust
// テスト前後の進捗を比較
let initial = verifier.get_progress().await?;
// ... run tests ...
let newly_solved = verifier.compare_progress(&initial).await?;
```

---

## モジュール詳細

### ctf.rs - CTF検証トレイト

```rust
pub trait ChallengeVerifier: Send + Sync {
    async fn is_solved(&self, key: &str) -> Result<bool>;
    async fn get_progress(&self) -> Result<ChallengeProgress>;
    async fn compare_progress(&self, before: &ChallengeProgress) -> Result<Vec<String>>;
}

pub struct ChallengeProgress {
    pub total: usize,
    pub solved: usize,
    pub percentage: f64,
    pub challenges: HashMap<String, bool>,
}
```

### scenario.rs - シナリオテスト

```rust
Scenario::new("Name")
    .base_url("http://...")
    .tag("category")
    .tags(&["tag1", "tag2"])
    .step("Step 1", |ctx| async move { ok() })
    .step("Step 2", |ctx| async move { ok_with("message") })
    .run()
    .await?;
```

### reporter.rs - レポート生成

```rust
let report = ReportBuilder::new()
    .add_result(result1)
    .add_result(result2)
    .build();

report.print_summary();
println!("{}", report.to_json());
```

### config.rs - 設定ファイル

```toml
# rectitude.toml
base_url = "http://localhost:3000"
timeout = 30
output = "text"
include_tags = ["security"]
exclude_tags = ["slow"]

[variables]
API_KEY = "test"
```
