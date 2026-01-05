# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**DONADONA** - A gamified engineer assignment platform with multi-tenancy and OAuth2/OIDC authentication via Ory Hydra. Features incident/project management, skill-based engineer assignments, game mechanics (levels, XP, achievements), and recruitment system. Uses Axum web framework with PostgreSQL.

**Key Architecture**: Ory Hydra handles OAuth2/OIDC protocol; this service handles user authentication, consent UI, tenant isolation, and the DONADONA platform operations.

## Build and Development

```sh
# Build (requires nightly for Edition 2024)
cargo +nightly build

# Run tests
cargo test

# Single test
cargo test test_name

# Lint and format
cargo fmt && cargo clippy -- -D warnings

# Run locally (requires .env or environment variables)
cargo run
```

## Docker Development

```sh
# Start full environment (Hydra + PostgreSQL + Auth Provider + Frontend)
docker compose up -d --build

# View logs
docker compose logs -f auth-provider

# Create test OAuth2 client
./scripts/create-client.sh

# Health check
curl http://localhost:3000/health
```

## Architecture

### Core Modules

- `main.rs` - Router setup with nested route groups for platform/tenant APIs
- `config.rs` - Environment configuration (HOST, PORT, HYDRA_ADMIN_URL, JWT_*, DATABASE_URL)
- `state.rs` - AppState containing all services (hydra, auth, jwt, tenant, product, order, cart, user)
- `error.rs` - AppError enum with Axum IntoResponse implementation

### Handlers

- `handlers/login.rs`, `consent.rs`, `logout.rs` - Hydra OAuth2 provider endpoints
- `handlers/auth.rs` - JWT-based API authentication (/api/auth/*)
- `handlers/platform/` - Platform admin API for tenant management
- `handlers/tenant/` - Tenant-scoped APIs (products, orders, cart)
- `handlers/pages.rs` - HTML pages for admin UI

### Services

- `HydraClient` - Ory Hydra Admin API client
- `AuthService` - User authentication with Argon2id password hashing
- `JwtService` - JWT token generation/validation
- `TenantService`, `UserService` - Core multi-tenant services
- `IncidentService`, `ProjectService` - Task management
- `EngineerService`, `RecruitmentService` - Engineer/candidate management
- `LeaderboardService`, `GameEngineService` - Game mechanics

### Middleware

- `require_auth` - JWT authentication middleware
- `extract_tenant` - Tenant extraction from subdomain or `X-Tenant-Slug` header
- `require_role`, `require_tenant_membership` - RBAC middleware

### Multi-tenancy

Tenant isolation via:
1. Subdomain parsing (e.g., `shop-a.example.com` → tenant slug `shop-a`)
2. `X-Tenant-Slug` header for local development
3. Per-tenant database schemas managed by `TenantSchemaManager`

## DONADONA ユーザーロール

| ロール | 説明 | 権限 |
|--------|------|------|
| platform_admin | プラットフォーム管理者 | テナント管理、全テナントアクセス |
| manager | チームマネージャー | エンジニア管理、アサイン、設定変更 |
| engineer | エンジニア | 担当案件の更新、ステータス変更 |
| reporter | 報告者 | インシデント報告のみ |

## DONADONA ゲーム要素

| 要素 | 説明 |
|------|------|
| レベル・経験値 | エンジニアがXPを獲得してレベルアップ (1-100) |
| 満足度 | 適切な難易度の案件でないと満足度低下 (0-100) |
| 専門分野 | SRE, Frontend, Backend, Infrastructure, Mobile, QA, Security |
| 熟練度 | beginner, intermediate, expert |
| 難易度 | easy, normal, hard, expert, extreme |
| 重要度 | critical, high, medium, low |

## DONADONA データモデル

### コアモデル (テナントスキーマ)
- `specialties` - エンジニア専門分野定義
- `engineer_specialties` - エンジニアと専門分野の関連 (熟練度付き)
- `workflow_statuses` - カスタムワークフローステータス
- `incidents` - インシデント (severity, difficulty, reward)
- `projects` - プロジェクト (priority, deadline, estimated_hours)
- `assignments` - アサインメント (incident/project ↔ engineer)
- `comments` - コメント/アクティビティログ

### ゲームモデル (テナントスキーマ)
- `engineers` - エンジニア拡張情報 (level, xp, satisfaction, salary)
- `achievements` - 実績/バッジ定義
- `engineer_achievements` - 獲得済み実績
- `skill_nodes` - スキルツリーノード
- `engineer_skill_nodes` - アンロック済みスキル
- `tenant_finance` - テナント財務 (balance, monthly_revenue)
- `transactions` - 取引履歴
- `trainings` - トレーニング定義
- `engineer_trainings` - 進行中トレーニング

### 採用モデル (テナントスキーマ)
- `candidates` - 採用候補者プール (rarity: common〜legendary)
- `recruitment_events` - 採用イベントログ
- `recruitment_settings` - リフレッシュ設定

## API Structure

| Route Group | Purpose |
|-------------|---------|
| `/login`, `/consent`, `/logout` | Hydra OAuth2 provider |
| `/api/auth/*` | JWT authentication (register, login, refresh, logout) |
| `/api/v1/tenants` | Platform admin - tenant CRUD (requires auth) |
| `/api/v1/tenant/incidents` | Incident management (CRUD, status, assign) |
| `/api/v1/tenant/projects` | Project management (CRUD, status, assign, hours) |
| `/api/v1/tenant/engineers` | Engineer list/details with specialties |
| `/api/v1/tenant/recruitment/*` | Candidate pool, hiring |
| `/api/v1/tenant/leaderboard/*` | Level, XP, revenue rankings |
| `/pages/*` | Admin HTML pages |

## DONADONA API詳細

### インシデントAPI
```
POST   /api/v1/tenant/incidents              # 作成
GET    /api/v1/tenant/incidents              # 一覧
GET    /api/v1/tenant/incidents/stats        # 統計
GET    /api/v1/tenant/incidents/:id          # 詳細
PUT    /api/v1/tenant/incidents/:id          # 更新
DELETE /api/v1/tenant/incidents/:id          # 削除
PATCH  /api/v1/tenant/incidents/:id/status   # ステータス変更
POST   /api/v1/tenant/incidents/:id/assign   # アサイン
```

### プロジェクトAPI
```
POST   /api/v1/tenant/projects               # 作成
GET    /api/v1/tenant/projects               # 一覧
GET    /api/v1/tenant/projects/stats         # 統計
GET    /api/v1/tenant/projects/:id           # 詳細
PUT    /api/v1/tenant/projects/:id           # 更新
DELETE /api/v1/tenant/projects/:id           # 削除
PATCH  /api/v1/tenant/projects/:id/status    # ステータス変更
POST   /api/v1/tenant/projects/:id/assign    # アサイン
PATCH  /api/v1/tenant/projects/:id/hours     # 工数更新
```

### エンジニアAPI
```
GET    /api/v1/tenant/engineers              # 一覧 (専門分野付き)
GET    /api/v1/tenant/engineers/salary       # 給与総額
GET    /api/v1/tenant/engineers/:id          # 詳細
POST   /api/v1/tenant/engineers/:id/specialties  # 専門分野追加
```

### 採用API
```
GET    /api/v1/tenant/recruitment            # 候補者一覧
GET    /api/v1/tenant/recruitment/status     # リフレッシュ状態
POST   /api/v1/tenant/recruitment/refresh    # プールリフレッシュ
POST   /api/v1/tenant/recruitment/hire       # 採用実行
```

### リーダーボードAPI
```
GET    /api/v1/tenant/leaderboard            # 総合ランキング
GET    /api/v1/tenant/leaderboard/level      # レベルランキング
GET    /api/v1/tenant/leaderboard/revenue    # 売上ランキング
GET    /api/v1/tenant/leaderboard/incidents  # インシデント解決数
GET    /api/v1/tenant/leaderboard/projects   # プロジェクト完了数
```

## フロントエンド構成

### ページ構成 (Next.js App Router)
```
frontend/src/app/
├── page.tsx                # ホーム (テストアカウント一覧)
├── callback/page.tsx       # OAuth2 コールバック
├── dashboard/page.tsx      # ダッシュボード (統計)
├── incidents/page.tsx      # インシデント一覧・作成
├── projects/page.tsx       # プロジェクト一覧・作成
├── engineers/page.tsx      # エンジニア一覧
├── recruitment/page.tsx    # 採用候補者プール
├── leaderboard/page.tsx    # ランキング (Level/Revenue/Incidents/Projects)
├── tenants/page.tsx        # テナント管理 (platform_adminのみ)
├── not-found.tsx           # 404ページ
└── error.tsx               # エラーページ
```

### APIルート
```
frontend/src/app/api/auth/
├── login/route.ts          # OAuth2認可リクエスト開始
├── callback/route.ts       # トークン交換、クッキー設定
└── logout/route.ts         # ログアウト (Hydraセッション削除含む)
```

### 認証クッキー
| Cookie名 | 用途 | HttpOnly |
|----------|------|----------|
| session | セッション情報 (base64 JSON) | Yes |
| auth_token | アクセストークン (API呼び出し用) | No |
| user_info | ユーザー表示情報 (base64 JSON) | No |

## Environment Variables

- `HYDRA_ADMIN_URL` - Hydra Admin API (default: `http://localhost:4445`)
- `JWT_SECRET` - JWT signing key (32+ chars)
- `JWT_ISSUER` - JWT issuer claim
- `DATABASE_URL` - PostgreSQL connection string
- `HOST`, `PORT` - Server binding (default: `0.0.0.0:3000`)

## Port Mapping

- 3000: Auth Provider (this service)
- 3001: Next.js Frontend
- 4444: Hydra Public API
- 4445: Hydra Admin API

## Troubleshooting

### ユーザー情報がヘッダーに表示されない

**問題**: ログイン後、ヘッダーにユーザー名やロールが表示されない

**原因と解決策**:

1. **Hydraセッションキャッシュ問題** (`src/handlers/login.rs`)
   - 問題: ログインスキップ時（`skip=true`）に`context: None`を渡していた
   - 解決: スキップ時もDBからユーザー情報を取得してcontextに含める
   ```rust
   // ログインスキップ時もユーザー情報を取得
   let user_id = Uuid::parse_str(&login_request.subject)?;
   let user = state.auth.get_user_by_id(&user_id).await?;
   context: Some(serde_json::json!({
       "email": user.email,
       "role": user.role.to_string(),
   }))
   ```

2. **Cookie読み取り問題** (`frontend/src/components/shared/Header.tsx`)
   - 問題: `.split("=")[1]`でbase64の`=`パディングが切れる
   - 解決: `substring("user_info=".length)`で値を取得
   ```typescript
   const userInfoCookie = cookieRow.substring("user_info=".length);
   const decodedCookie = decodeURIComponent(userInfoCookie);
   const userInfo = JSON.parse(atob(decodedCookie));
   ```

3. **クライアント側ナビゲーション問題** (`frontend/src/app/callback/page.tsx`)
   - 問題: `router.push()`ではHeaderのuseEffectが再実行されない
   - 解決: `window.location.href`でフルページリロードを強制

### Hydraセッションをクリアする方法

```bash
# 特定ユーザーのセッションをクリア
curl -X DELETE "http://localhost:4445/admin/oauth2/auth/sessions/consent?subject=USER_ID&all=true"
curl -X DELETE "http://localhost:4445/admin/oauth2/auth/sessions/login?subject=USER_ID"

# Hydraを再起動してすべてのセッションをクリア
docker compose restart hydra
```

### テストアカウント

| Email | Password | Role | Description |
|-------|----------|------|-------------|
| demo@example.com | password123 | platform_admin | Full platform access |
| manager@example.com | password123 | manager | Manage engineers, assign tasks |
| sato@example.com | password123 | engineer | Level 5, SRE + Backend |
| tanaka@example.com | password123 | engineer | Level 3, Frontend |
| suzuki@example.com | password123 | engineer | Level 2, Backend + Infra |
| reporter@example.com | password123 | reporter | Report incidents only |

## フロントエンドデバッグ方法

### Claude Codeでのデバッグ

**1. Debugger Subagentを使用**
```
debugger subagentでこのエラーを調査して
debugger subagentを使ってReactコンポーネントの問題を見つけて
```

**2. 複雑な問題には拡張思考を使用**
```
ultrathink: このNext.jsアプリのパフォーマンス問題をデバッグして
```

**3. ファイル参照でコンテキストを提供**
```
@frontend/src/components/Header.tsx に問題がありそう。何が間違っている？
```

**4. スクリーンショットで視覚的デバッグ**
- UIの問題はスクリーンショットをドラッグ＆ドロップまたはCtrl+Vで貼り付け

### ブラウザでのデバッグ

**1. React Developer Tools**
- コンポーネント階層、props、stateの検査
- ハイドレーションエラーの検出
- 再レンダリングの追跡

**2. ブラウザDevTools**
- Console: エラーログ確認
- Application → Cookies: クッキーの確認・削除
- Network: APIリクエストの監視

**3. Next.js開発モード**
```bash
# 開発サーバー起動（詳細なエラー表示）
npm run dev

# デバッグモードで起動
NODE_OPTIONS='--inspect' npm run dev
```

### Docker環境でのデバッグ

```bash
# フロントエンドログを確認
docker logs -f ory-hydra-rust-frontend-1

# コンテナ内でコマンド実行
docker exec -it ory-hydra-rust-frontend-1 sh

# リアルタイムログ監視
docker compose logs -f frontend
```

### よくある問題と解決方法

| 症状 | 確認箇所 | コマンド |
|------|----------|----------|
| Cookieが読めない | Application → Cookies | `document.cookie` をConsoleで実行 |
| APIエラー | Network タブ | レスポンスボディを確認 |
| コンポーネント更新されない | React DevTools | propsとstateを確認 |
| ハイドレーションエラー | Console | サーバー/クライアントの差分を確認 |

### デバッグ用コード追加（一時的）

```typescript
// コンソールログを追加
console.log("Debug:", variable);

// useEffectのデバッグ
useEffect(() => {
  console.log("Effect triggered, deps:", dependency);
}, [dependency]);

// Cookieの確認
console.log("Cookies:", document.cookie);
```

### 参考リンク

- [Next.js Debugging Guide](https://nextjs.org/docs/app/guides/debugging)
- [React Developer Tools](https://react.dev/learn/react-developer-tools)

### ログアウト後も以前のアカウントで自動ログインされる

**問題**: ログアウト後に再度ログインすると、ログイン画面をスキップして以前のアカウントで自動ログインされる

**原因**: フロントエンドのCookieをクリアしただけではHydraのセッションが残っているため、Hydraが`skip: true`を返してログインをスキップする

**解決策** (`frontend/src/app/api/auth/logout/route.ts`):

```typescript
// 1. セッションからユーザーIDを取得
const session = JSON.parse(Buffer.from(sessionCookie.value, "base64").toString("utf-8"));
const userId = session.user?.id; // IDトークンのsub

// 2. アクセストークンをrevoke
await fetch(`${hydraPublicUrl}/oauth2/revoke`, {
  method: "POST",
  body: new URLSearchParams({
    token: accessToken,
    client_id: clientId,
    client_secret: clientSecret,
  }),
});

// 3. Hydra Admin APIでセッションを削除
await fetch(
  `${hydraAdminUrl}/admin/oauth2/auth/sessions/consent?subject=${userId}&all=true`,
  { method: "DELETE" }
);
await fetch(
  `${hydraAdminUrl}/admin/oauth2/auth/sessions/login?subject=${userId}`,
  { method: "DELETE" }
);
```

**重要なポイント**:
1. **ユーザーIDの保存**: callbackでIDトークンの`sub`をセッションに保存しておく
2. **3種類のクリアが必要**:
   - フロントエンドCookie（session, auth_token, user_info）
   - アクセストークンのrevoke（Hydra Public API）
   - ログイン/コンセントセッションの削除（Hydra Admin API）
3. **Admin APIアクセス**: Docker環境では`http://hydra:4445`、ローカルでは`http://localhost:4445`
4. **docker-compose.ymlの環境変数**: フロントエンドに`HYDRA_ADMIN_URL: http://hydra:4445`を設定する（Docker内では`localhost`ではなくサービス名を使用）

**Hydraセッションの仕組み**:
- `remember: true`でログインを記憶すると、次回のログインリクエストで`skip: true`が返される
- `skip: true`の場合、ログイン画面をスキップして以前のsubjectでログインを承認する
- セッションを削除しないと、この状態が維持される

## DONADONA特有の学び

### テナントスキーマの移行

**問題**: 既存のテナントスキーマ（EC用: products, orders, carts）をDONADONA用に変換する必要がある

**解決策**:
1. 既存スキーマをDROP: `DROP SCHEMA tenant_xxx CASCADE;`
2. 新スキーマテンプレート（`sql/tenant_schema_template.sql`）を適用
3. バックエンド再起動でシードデータが投入される

```bash
# 手動でスキーマを再作成
cat sql/tenant_schema_template.sql | sed 's/{{schema_name}}/tenant_xxx/g' | docker exec -i postgres psql -U postgres -d postgres
```

### シードデータのユーザーID問題

**問題**: シード関数でハードコードしたUUIDと既存ユーザーのUUIDが異なる場合、外部キー制約違反が発生

**解決策** (`src/services/user.rs`):
- ハードコードUUIDではなく、メールアドレスで既存ユーザーを検索してIDを取得

```rust
let user_result: Result<(Uuid,), _> = sqlx::query_as(
    "SELECT id FROM public.users WHERE email = $1 AND tenant_id = $2",
)
.bind(email)
.bind(tenant_id)
.fetch_one(&self.pool)
.await;
```

### パスワードハッシュの更新

**問題**: 既存ユーザーのパスワードハッシュが古い場合、ログインできない

**解決策**: UPSERTでパスワードハッシュを更新

```sql
INSERT INTO public.users (id, email, password_hash, ...)
VALUES (gen_random_uuid(), $1, $2, ...)
ON CONFLICT (email) DO UPDATE SET
    password_hash = EXCLUDED.password_hash,
    role = EXCLUDED.role,
    updated_at = EXCLUDED.updated_at
```

### Next.js ビルドエラー

**問題**: `<Html> should not be imported outside of pages/_document`

**原因**: `NODE_ENV`がdevelopmentのままビルドされる

**解決策**: `package.json`でビルドコマンドを修正

```json
"scripts": {
  "build": "NODE_ENV=production next build"
}
```

### テナントスキーマのテーブル構成

DONADONA用テナントスキーマ（19テーブル）:
- **コア**: specialties, workflow_statuses, incidents, projects, assignments, comments
- **ゲーム**: engineers, achievements, engineer_achievements, skill_nodes, engineer_skill_nodes
- **財務**: tenant_finance, transactions, trainings, engineer_trainings
- **採用**: candidates, recruitment_events, recruitment_settings, engineer_specialties

### DONADONA APIテスト方法

```bash
# ログイン
curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "sato@example.com", "password": "password123"}' | jq -r '.access_token'

# エンジニア一覧
curl -s http://localhost:3000/api/v1/tenant/engineers \
  -H 'Authorization: Bearer TOKEN' | jq .

# リーダーボード
curl -s http://localhost:3000/api/v1/tenant/leaderboard/level \
  -H 'Authorization: Bearer TOKEN' | jq .
```

### フロントエンドのクッキー名

**問題**: フロントエンドページが`access_token`クッキーを探しているが、callbackルートは`auth_token`として設定している

**解決策**: すべてのページで`auth_token=`を使用

```typescript
// NG
.find((row) => row.startsWith("access_token="))

// OK
.find((row) => row.startsWith("auth_token="))
```

**修正が必要なファイル**:
- `src/app/dashboard/page.tsx`
- `src/app/engineers/page.tsx`
- `src/app/incidents/page.tsx`
- `src/app/projects/page.tsx`
- `src/app/leaderboard/page.tsx`
- `src/app/recruitment/page.tsx`
