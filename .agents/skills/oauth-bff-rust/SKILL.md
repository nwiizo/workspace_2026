---
name: oauth-bff-rust
description: Rust/Axum などで SPA 向け OAuth 2.0 / OpenID Connect BFF を実装・レビュー・検証する。ブラウザにアクセストークンやリフレッシュトークンを出さない設計、HttpOnly Cookie セッション、CSRF/CORS/Origin 検証、Hydra などの認可サーバー連携、Playwright でのログイン往復検証を扱うときに使用。
---

# OAuth BFF Rust

## Overview

SPA で OAuth/OIDC を扱うときは、トークンをブラウザに置かず BFF が confidential client として保持する。Rust 実装では「OAuth Agent」と「OAuth Proxy」の責務を分け、Cookie はセッション識別子と CSRF 用の値だけにする。

## Design Gate

実装前に次を明確にする。

- SPA origin、BFF origin、認可サーバーの browser-visible URL、BFF から見た internal URL を分ける。Docker Compose では `http://localhost:4444` と `http://hydra:4444` が別物になりやすい。
- OAuth client は confidential client とし、認可コード交換・refresh・logout を BFF 内に閉じる。
- ブラウザへ渡す値は `__Host-` prefix の HttpOnly session cookie と、必要なら JavaScript が読める CSRF cookie/token に限定する。
- Proxy は任意 URL を受け取らない。ルートごとに承認済み upstream allowlist へ固定変換する。
- SameSite は CSRF 対策の一部でしかない。CORS exact origin、Origin 検証、unsafe method の CSRF token 検証を併用する。

## Implementation Checklist

### OAuth Agent

- `GET /login`: PKCE verifier/challenge、state、nonce を生成し、state-bound の短命 login state を保存して authorization URL を返すか redirect する。
- `GET /callback`: state と PKCE verifier を検証し、authorization code を token endpoint で交換する。SPA 側 callback では code/token を処理しない。
- Token storage は server-side session store を優先する。Cookie には opaque session id だけを入れる。
- Access token expiry を保存し、proxy 時に期限切れなら refresh token で更新する。refresh 失敗時は session を破棄する。
- `GET /me` は BFF session から userinfo または ID token claims を返し、トークン値は返さない。
- `POST /logout` は BFF session cookie 失効、server-side token 削除、必要なら OP logout を連動させる。

### OAuth Proxy

- Browser request の Cookie から session を解決し、上流 API へ `Authorization: Bearer <access_token>` を付与する。
- Client から来た `Authorization` header は信用しない。BFF が保持する token だけを使う。
- Hop-by-hop headers、Cookie、Host などは上流へ雑に転送しない。
- Path rewrite は explicit mapping にする。`/bff/orders/create -> https://order-api.example.com/create` のように固定し、host/path injection を避ける。
- Upstream から返すエラーは認証情報や token endpoint の詳細を漏らさない範囲に整形する。

### Cookie And CSRF

- Session cookie: `__Host-bff-session`, `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, no `Domain`。
- CSRF cookie/token: JavaScript が読める値と server-side session の値を照合する double-submit または session-bound token にする。
- Unsafe methods (`POST`, `PUT`, `PATCH`, `DELETE`) は `Origin` と `X-CSRF-Token` を検証する。
- CORS は exact SPA origin のみ許可し、credentialed request で `Access-Control-Allow-Origin: *` を使わない。
- Localhost 検証でも本番 cookie 属性を落とさない。ブラウザ挙動は Playwright で実測する。

## Rust/Axum Notes

- Shared state には OAuth client config、HTTP client、session store、upstream allowlist、allowed origin を集約する。
- `thiserror` で OAuth/token/session/proxy error を分け、handler では `IntoResponse` へ変換する。
- Secret は env/config から注入し、コード・README・test snapshot に値を残さない。
- Axum の wildcard route はバージョン差に注意する。`/api/bff/proxy/*path` と `{*path}` のどちらが有効かを起動時に確認する。
- Redirect URI は Hydra/OIDC client 登録と完全一致させる。frontend callback ではなく BFF callback を登録する。
- Docker 内の token endpoint は service name、ブラウザ redirect は localhost/public URL を使う構成にする。

## Frontend Changes

- localStorage、sessionStorage、memory、non-HttpOnly cookie から token 読み書きを削除する。
- API client は `credentials: "include"` を使い、`Authorization` header を組み立てない。
- Unsafe method では BFF から CSRF token を取得して `X-CSRF-Token` に付与する。
- Login は BFF の `/login` に遷移し、callback token exchange は SPA から削除または 410 にする。
- UI は `/me` などの BFF endpoint から認証状態を読む。

## Validation

必ず local stack を起動してブラウザで検証する。Playwright MCP が使える場合は MCP を優先し、使えない場合は Playwright CLI で同じ観点を確認する。

Minimum checks:

- Rust: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all --all-targets`。
- Frontend: package manager に合う lint/build/typecheck。
- Login round trip: SPA login -> authorization server -> BFF callback -> SPA route。
- Browser storage: page body, cookies visible to JS, localStorage, sessionStorage に `access_token`, `refresh_token`, `auth_token` がない。
- Cookie attributes: session cookie が `httpOnly=true`, `secure=true`, `sameSite=Strict`, `__Host-` prefix。
- BFF `/me`: 200 を返し、token 値を含めない。
- Proxy: authenticated request が 200/expected status、unauthenticated request が 401/403。
- CSRF: unsafe method without header が 403、valid token 付きが expected status。
- Origin/CORS: allowed origin だけ credentialed request を許可する。

## Commit Discipline

- Stage only files touched for the BFF change. Leave unrelated dirty workspace files alone.
- Commit message should name the sample/tool scope, e.g. `feat(ory-hydra-rust): add OAuth BFF token isolation`.
- Final report should include verification commands, browser checks, local URLs, and any unavailable requested tool such as Playwright MCP.
