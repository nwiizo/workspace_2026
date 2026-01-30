# Rectitude

Rust製の汎用E2Eシナリオテストライブラリ。セキュリティテストに特化。

## 特徴

- **シナリオベースのテスト** - 複数のHTTPリクエストをステップで連鎖
- **状態管理** - ステップ間での変数、Cookie、JWTトークンの共有
- **セキュリティペイロード** - SQLi、XSS、XXE、JWT操作など
- **CTF検証** - `ChallengeVerifier` トレイトで実際のチャレンジ完了を検証
- **タグベースのフィルタリング** - シナリオをカテゴリ/難易度でタグ付け
- **レポート生成** - JSON/テキスト形式でテスト結果を出力
- **設定ファイル** - TOML形式でプロジェクト設定を管理

## インストール

```bash
cargo add rectitude
```

## クイックスタート

```rust
use rectitude::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Scenario::new("ログインテスト")
        .base_url("http://localhost:3000")
        .tags(&["auth", "smoke"])
        .step("ログイン", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.post("/api/login")
                .json(&serde_json::json!({
                    "email": "user@example.com",
                    "password": "password123"
                }))
                .send()
                .await?;

            ctx.assert_status(&resp, 200)?;
            ctx.store("token", &resp, "$.token").await?;
            ok_with("Login successful")
        })
        .step("保護されたリソースにアクセス", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("token").await?;
            let resp = ctx.get("/api/protected")
                .bearer_auth(&token)
                .send()
                .await?;

            ctx.assert_status(&resp, 200)?;
            ok()
        })
        .run()
        .await?;

    Ok(())
}
```

## セキュリティテストの例

### SQLi ログインバイパス

```rust
use rectitude::prelude::*;
use std::sync::Arc;

Scenario::new("SQLi Login Bypass")
    .base_url("http://localhost:3000")
    .tags(&["sqli", "auth"])
    .step("SQLiペイロードでログイン", |ctx: Arc<ScenarioContext>| async move {
        // 便利ヘルパーを使用
        ctx.sqli_login("/rest/user/login", "admin@example.com").await?;
        ok_with("SQLi bypass successful")
    })
    .run()
    .await?;
```

### レースコンディションテスト

```rust
.step("Race condition", |ctx: Arc<ScenarioContext>| async move {
    let results = ctx.race(10, || {
        ctx.post("/api/redeem-coupon")
            .json(&serde_json::json!({"code": "DISCOUNT50"}))
    }).await;

    let successes = results.iter().filter(|r| r.is_ok()).count();
    ok_with(format!("{} succeeded", successes))
})
```

## CTF検証

シナリオテストでは「HTTPが成功した」だけでなく「チャレンジが実際に解けた」ことを検証すべきです。

```rust
use rectitude::ctf::{ChallengeVerifier, ChallengeProgress};
use async_trait::async_trait;

// CTFプラットフォーム用のVerifierを実装
struct MyCtfVerifier { /* ... */ }

#[async_trait]
impl ChallengeVerifier for MyCtfVerifier {
    async fn is_solved(&self, key: &str) -> rectitude::Result<bool> {
        // CTF APIで検証
        Ok(true)
    }
    async fn get_progress(&self) -> rectitude::Result<ChallengeProgress> {
        Ok(ChallengeProgress::default())
    }
}

// テスト前後の進捗を比較
let initial = verifier.get_progress().await?;
// ... run tests ...
let newly_solved = verifier.compare_progress(&initial).await?;
println!("Newly solved: {:?}", newly_solved);
```

## レポート生成

```rust
use rectitude::reporter::ReportBuilder;

let report = ReportBuilder::new()
    .add_result(scenario1.run().await?)
    .add_result(scenario2.run().await?)
    .build();

report.print_summary();
println!("{}", report.to_json());
```

## 設定ファイル

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

## CLI

```bash
# 設定ファイル生成
rectitude init

# シナリオ一覧
rectitude list

# ペイロード生成
rectitude payloads sqli              # SQLiペイロード
rectitude payloads xss               # XSSペイロード
rectitude payloads jwt <TOKEN>       # JWTデコード
rectitude payloads jwt --unsigned    # 未署名JWT作成
rectitude payloads ssrf --port 8080  # SSRFバイパスURL
```

## セキュリティペイロード

| モジュール | 内容 |
|-----------|------|
| `sqli` | 認証バイパス、UNION、ブラインドSQLi |
| `xss` | 基本、フィルターバイパス |
| `jwt` | 未署名(alg:none)、HS256、アルゴリズム混同攻撃 |
| `ssrf` | localhost変換、クラウドメタデータ |
| `xxe` | ファイル読み取り、SSRF |
| `nosql` | MongoDB インジェクション |
| `traversal` | パストラバーサル各種エンコーディング |
| `encoding` | URL、Base64、Hex、HTMLエンティティ |

## 設計原則

### ライブラリは汎用、具体実装はexamples

- `src/` は汎用的なセキュリティテスト機能のみ
- CTF固有の実装（JuiceShopVerifier等）は `examples/` に配置
- ライブラリは「ついでに」特定CTFを解けるが、それが目的ではない

### シナリオテストの検証

```rust
// ❌ HTTPの成功だけを確認
let resp = ctx.get("/hidden-page").send().await?;
resp.expect_success()?;  // 200 OK = テスト成功？

// ✅ 実際にチャレンジが解決されたか検証
.step("Attack", |ctx| async move { /* ... */ })
.step("Verify", |_| async move {
    if verifier.is_solved("challengeKey").await? {
        ok_with("VERIFIED")
    } else {
        fail("NOT SOLVED")
    }
})
```

## Examples

```bash
# 検証付きJuice Shopシナリオ
cargo run --example juice_shop_verified

# CTF攻略シナリオ
cargo run --example juice_shop_ctf

# 追加チャレンジ
cargo run --example juice_shop_complete
```

## 開発

```bash
# ビルド
cargo build --release

# テスト
cargo test

# Lint
cargo fmt && cargo clippy -- -D warnings
```

## ライセンス

MIT
