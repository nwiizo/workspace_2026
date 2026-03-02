# Leptos 0.8 で Twitter クローンを作って学んだこと【前編】Leptos の基本機能

[https://leptos.dev/:embed:cite]

[https://github.com/leptos-rs/leptos:embed:cite]

`cargo leptos serve` を叩いて、コンパイルが通るまでの数十秒を待つ。Vite なら瞬きの間に終わる作業だ。Rust のコンパイラは型を検査し、借用をチェックし、所有権の整合性を検証している。その間に「これ、TypeScript で書いた方が速かったのでは」と頭をよぎる。しかしコンパイルが通った瞬間、ブラウザに表示された画面は — 動く。undefined のランタイムエラーもない。型の不一致もない。「動くはず」ではなく「コンパイラが動くことを保証した」という確信がある。

この体験が、Chirp — Twitter ライクな SNS を Leptos 0.8 + Axum 0.8 で作り切る動機になった。

プログラミング言語には設計者の哲学が刻まれている。C は「プログラマを信頼する」という思想のもと、ハードウェアへの直接アクセスと引き換えに未定義動作のリスクを受け入れた。JavaScript は「とにかく動かす」を優先し、型の暗黙変換と `undefined` の混沌とともにブラウザの標準語になった。そして Rust は「信頼するが、コンパイル時に検証する」という立場を取り、所有権システムによってメモリ安全性をランタイムコストなしに保証する道を選んだ。

フロントエンド開発の歴史は「状態の管理」との格闘の歴史だ。jQuery の直接 DOM 操作から、React の仮想 DOM と「UI は状態の関数」という宣言的モデルへ。そして SolidJS のきめ細かいリアクティビティへ。各世代のフレームワークは「変更可能な共有状態をどう手懐けるか」という問いに、それぞれの時代の答えを出してきた。

Leptos はこの二つの流れが交差する場所に立っている。Rust の「共有可能か、変更可能か、どちらか一方」という厳格な哲学と、フロントエンドの「すべてが共有された変更可能な状態」という現実。本記事では、Chirp の実装経験をもとに Leptos の基本機能を解説し、Rust の言語特性とフロントエンドの要請がどこで衝突し、どう折り合いをつけているのかを掘り下げる。

> 本記事は3部構成です。
> - **前編（本記事）**: Leptos の基本機能 — プロジェクト構成、コンポーネント、Signal、サーバー関数、ルーティング
> - [中編](blog_02.md): バックエンドの設計パターン — DB 設計、認証、検索、通知
> - [後編](blog_03.md): Rust の言語哲学とフロントエンドの交差点 — 所有権、クロージャ、型システムの衝突と折り合い

## 目次

1. [プロジェクト構成と SSR/Hydration の仕組み](#1-プロジェクト構成と-ssrhydration-の仕組み)
2. [コンポーネントと `view!` マクロ](#2-コンポーネントと-view-マクロ)
3. [リアクティブシステム — Signal](#3-リアクティブシステム--signal)
4. [サーバー関数 — `#[server]`](#4-サーバー関数--server)
5. [非同期データ取得 — Resource と Suspense](#5-非同期データ取得--resource-と-suspense)
6. [フォーム処理 — ServerAction と ActionForm](#6-フォーム処理--serveraction-と-actionform)
7. [ルーティング](#7-ルーティング)
8. [Axum 統合とコンテキスト共有](#8-axum-統合とコンテキスト共有)
9. [エラーハンドリング](#9-エラーハンドリング)
10. [スタイリング — Tailwind CSS v4](#10-スタイリング--tailwind-css-v4)

---

## 1. プロジェクト構成と SSR/Hydration の仕組み

Leptos アプリは **1 つの Rust クレートから 2 つのビルドターゲット** を生成する。

```
chirp/
├── Cargo.toml          # features: ssr / hydrate
├── src/
│   ├── main.rs         # サーバーバイナリ (feature = "ssr")
│   ├── lib.rs          # WASM ライブラリ (feature = "hydrate")
│   ├── app.rs          # ルートコンポーネント + ルーティング
│   ├── components/     # UI コンポーネント
│   ├── pages/          # ページコンポーネント
│   ├── server/         # サーバー関数・DB アクセス
│   ├── models/         # 共有データモデル
│   └── error.rs        # エラー型
├── migrations/         # SQLx マイグレーション
├── style/              # Tailwind CSS
└── public/             # 静的アセット
```

`Cargo.toml` の `[features]` でサーバーとクライアントの依存を分離する:

```toml
[features]
hydrate = [
    "dep:console_error_panic_hook",
    "dep:wasm-bindgen",
    "leptos/hydrate",
]
ssr = [
    "dep:axum",
    "dep:tokio",
    "dep:leptos_axum",
    "dep:sqlx",
    "dep:tower-sessions",
    "dep:argon2",
    "leptos/ssr",
    "leptos_meta/ssr",
    "leptos_router/ssr",
]
```

**SSR (Server-Side Rendering)**: サーバーが HTML を生成してクライアントに送信する。初回表示が高速で SEO にも有利。

**Hydration**: ブラウザが WASM をロードした後、既存の DOM にイベントハンドラを接続する。ページの再描画なしにインタラクティブになる。

クライアント側のエントリポイント (`lib.rs`) は短い:

```rust
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
```

`cargo-leptos` がこの 2 つのビルドを自動管理する。`cargo leptos serve` 一発で SSR バイナリと WASM の両方がビルドされ、ホットリロード付きの開発サーバーが立ち上がる。

---

## 2. コンポーネントと `view!` マクロ

Leptos のコンポーネントは `#[component]` 属性付きの関数で定義する。JSX の代わりに `view!` マクロで UI を記述する。

```rust
#[component]
pub fn PostCard(post: PostWithMeta) -> impl IntoView {
    let post_url = format!("/post/{}", post.id);
    let user_url = format!("/user/{}", post.author.username);
    let at_username = format!("@{}", post.author.username);

    view! {
        <article class="p-4 border-b border-gray-800 hover:bg-gray-900/50 transition-colors">
            <div class="flex gap-3">
                <A href=user_url.clone()>
                    <UserAvatar url=post.author.avatar_url.clone() size=40 />
                </A>
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-1 text-sm">
                        <A href=user_url attr:class="font-bold hover:underline truncate">
                            {post.author.display_name.clone()}
                        </A>
                        <A href=post_url attr:class="text-gray-500 hover:underline">
                            {format_time_ago(post.created_at)}
                        </A>
                    </div>
                    <div class="mt-1 whitespace-pre-wrap break-words">
                        {post.content.clone()}
                    </div>
                </div>
            </div>
        </article>
    }
}
```

### ポイント

- `view!` マクロ内は **HTML ライクな構文** だが、`{}` 内に Rust の式を直接書ける
- 文字列リテラルは `"テキスト"` のように引用符で囲む
- 子コンポーネントは `<PostCard post=post />` のように props を渡す
- `Children` 型で子要素のスロットを受け取れる:

```rust
#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="flex justify-center min-h-screen">
            <NavSidebar />
            <main class="flex-1 max-w-[600px]">{children()}</main>
            <RightSidebar />
        </div>
    }
}
```

---

## 3. リアクティブシステム — Signal

Leptos のリアクティビティの核は **Signal** だ。React の `useState` に似ているが、仮想 DOM なしの **きめ細かいリアクティビティ (fine-grained reactivity)** を実現する。Signal が変更されると、その Signal を購読している DOM ノードだけが更新される。

```rust
#[component]
pub fn ActionBar(
    post_id: String,
    like_count: i32,
    liked: bool,
) -> impl IntoView {
    // signal() でリアクティブな値を作成
    let (is_liked, set_liked) = signal(liked);
    let (local_like_count, set_like_count) = signal(like_count);

    let on_like = move |_| {
        let was_liked = is_liked.get_untracked();  // 購読せずに値を取得
        set_liked.set(!was_liked);                  // 値をセット
        set_like_count.update(|c| {                 // ミュータブル参照で更新
            if was_liked { *c -= 1 } else { *c += 1 }
        });
    };

    view! {
        <button
            // クロージャを渡すと、Signal 変更時に自動更新される
            class=move || {
                if is_liked.get() {
                    "text-pink-600"
                } else {
                    "hover:text-pink-600"
                }
            }
            on:click=on_like
        >
            // この部分も Signal 変更で自動更新
            <span>{move || if is_liked.get() { "❤️" } else { "🤍" }}</span>
            <span>{move || local_like_count.get()}</span>
        </button>
    }
}
```

### Signal の API

| メソッド | 説明 |
|---------|------|
| `signal(value)` | `(ReadSignal, WriteSignal)` のペアを作成 |
| `.get()` | 値を取得 + 購読（変更時に再実行） |
| `.get_untracked()` | 値を取得するが購読しない |
| `.set(value)` | 値を上書き |
| `.update(\|v\| ...)` | ミュータブル参照で更新 |

`class` や `view!` 内で `move || signal.get()` のクロージャを渡すと、Signal が変更されたときにその部分だけが自動で再レンダリングされる。コンポーネント全体ではなく、**DOM ノード単位** で更新が走るのが React との大きな違いだ。

---

## 4. サーバー関数 — `#[server]`

Leptos の最も強力な機能の一つが **サーバー関数** だ。`#[server]` 属性を付けた関数は:

- **サーバー上でのみ実行される**（DB アクセス、認証チェックなど）
- **クライアントからは自動生成された RPC として呼び出される**
- 引数と戻り値は `Serialize + Deserialize` を実装する必要がある

```rust
/// 新しい投稿を作成する
#[server]
pub async fn create_post(
    content: String,
    reply_to_id: Option<String>,
) -> Result<PostWithMeta, ServerFnError> {
    // ここから先はサーバーでのみ実行される
    // use 文もサーバー限定の依存を使える
    use uuid::Uuid;

    let pool = super::db::pool()?;
    let session = super::auth::extract_session().await?;
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // バリデーション
    let char_count = content.chars().count();
    if content.trim().is_empty() || char_count > 280 {
        return Err(ServerFnError::new(
            "Post must be between 1 and 280 characters",
        ));
    }

    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO posts (id, user_id, content, reply_to_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(id)
    .bind(user_id)
    .bind(&content)
    .bind(reply_to)
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(post_with_meta)
}
```

### DB プールの共有パターン

Axum のコンテキストとして `PgPool` を注入し、サーバー関数から `use_context` で取得する:

```rust
// server/db.rs
pub fn pool() -> Result<PgPool, ServerFnError> {
    use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found in context"))
}
```

### セッション認証

`tower-sessions` の `Session` を `leptos_axum::extract` で取り出す:

```rust
pub(crate) async fn extract_session() -> Result<tower_sessions::Session, ServerFnError> {
    leptos_axum::extract::<tower_sessions::Session>()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract session: {e}")))
}
```

サーバー関数の中で `leptos_axum::extract` を使うことで、Axum のエクストラクタ（Session, Headers など）にアクセスできる。これにより Leptos と Axum のエコシステムがシームレスに統合される。

---

## 5. 非同期データ取得 — Resource と Suspense

**Resource** はサーバー関数の呼び出し結果をリアクティブに管理する仕組みだ。

```rust
#[component]
pub fn HomePage() -> impl IntoView {
    // Resource::new(依存Signal, 非同期関数)
    let current_user = Resource::new(
        || (),
        |_| async { get_current_user().await },
    );
    let timeline = Resource::new(
        || (),
        |_| async { get_home_timeline(None, None).await },
    );

    view! {
        // Suspense でローディング中のフォールバックを表示
        <Suspense fallback=move || {
            view! { <div class="p-4 text-gray-500">"タイムラインを読み込み中..."</div> }
        }>
            {move || {
                timeline
                    .get()
                    .map(|result| {
                        match result {
                            Ok(posts) => view! { <PostList posts=posts /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="p-4 text-red-500">
                                        {format!("エラー: {e}")}
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    })
            }}
        </Suspense>
    }
}
```

### 動作の流れ

1. **SSR 時**: サーバーで `Resource` の非同期関数を実行し、結果を HTML に埋め込む
2. **Hydration 時**: クライアントは SSR の結果をそのまま使い、不要な再フェッチを避ける
3. **クライアントナビゲーション時**: クライアントからサーバー関数を RPC で呼び出す

`Suspense` は React の `Suspense` と同じ概念で、Resource がまだ解決されていない間はフォールバック UI を表示する。

---

## 6. フォーム処理 — ServerAction と ActionForm

データの変更（作成・更新・削除）には **ServerAction** と **ActionForm** を使う。

```rust
#[component]
pub fn LoginPage() -> impl IntoView {
    // サーバー関数 Login に対応する Action を作成
    let login_action = ServerAction::<Login>::new();
    let value = login_action.value();  // 結果をリアクティブに監視

    view! {
        // ActionForm はサーバー関数を呼び出す <form> を生成する
        <ActionForm action=login_action>
            <input type="text" name="username" required />
            <input type="password" name="password" required />

            // エラーメッセージの表示
            {move || {
                value
                    .get()
                    .and_then(|r: Result<(), ServerFnError>| r.err())
                    .map(|e: ServerFnError| {
                        view! { <p class="text-red-500">{e.to_string()}</p> }
                    })
            }}

            <button type="submit">"ログイン"</button>
        </ActionForm>
    }
}
```

### プログレッシブエンハンスメント

`ActionForm` の大きな利点は **JavaScript が無効でも動作する** ことだ。

- **JS 無効時**: 通常の HTML フォーム送信としてサーバーに POST される
- **JS 有効時**: フォーム送信をインターセプトし、非同期でサーバー関数を呼び出す。ページ遷移なしで結果が反映される

投稿コンポーザーでは、Signal と組み合わせてリアルタイムの文字数カウントを実現している:

```rust
#[component]
pub fn PostComposer(/* ... */) -> impl IntoView {
    let create_post = ServerAction::<CreatePost>::new();
    let (content, set_content) = signal(String::new());
    let char_count = move || content.get().len();

    view! {
        <ActionForm action=create_post>
            <textarea
                name="content"
                placeholder="いまどうしてる？"
                maxlength="280"
                prop:value=move || content.get()
                on:input=move |ev| {
                    set_content.set(event_target_value(&ev));
                }
            />
            <span class=move || {
                if char_count() > 260 { "text-red-500" } else { "text-gray-500" }
            }>
                {move || format!("{}/280", char_count())}
            </span>
            <button
                type="submit"
                disabled=move || {
                    content.get().trim().is_empty() || char_count() > 280
                }
            >
                "Chirp"
            </button>
        </ActionForm>
    }
}
```

---

## 7. ルーティング

`leptos_router` でクライアントサイドルーティングを設定する:

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/chirp.css" />
        <Title text="Chirp" />

        <Router>
            <Layout>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=pages::home::HomePage />
                    <Route path=path!("/login") view=pages::login::LoginPage />
                    <Route path=path!("/post/:id") view=pages::post_detail::PostDetailPage />
                    <Route path=path!("/user/:username") view=pages::profile::ProfilePage />
                    <Route path=path!("/explore") view=pages::explore::ExplorePage />
                    <Route path=path!("/search") view=pages::search::SearchPage />
                </Routes>
            </Layout>
        </Router>
    }
}
```

- `path!("/post/:id")` — コンパイル時にパスパターンを検証するマクロ
- `<A href=...>` コンポーネントでクライアントサイドナビゲーション（ページ全体のリロードなし）
- `<Layout>` が `Children` を受け取り、3 カラムレイアウト（ナビ・メイン・サイドバー）を構成

---

## 8. Axum 統合とコンテキスト共有

`main.rs` でサーバーを構築する。Leptos と Axum の統合ポイントは `leptos_routes_with_context` だ:

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // マイグレーション自動実行
    sqlx::migrate!().run(&pool).await.expect("Failed to run migrations");

    // セッションストア
    let session_store = PostgresStore::new(pool.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let pool = pool.clone();
                move || {
                    // PgPool を Leptos のコンテキストに注入
                    // サーバー関数から use_context::<PgPool>() で取得可能になる
                    leptos::context::provide_context(pool.clone());
                }
            },
            move || shell(leptos_options.clone()),
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(session_layer);  // tower レイヤーをそのまま使える

    axum::serve(listener, app.into_make_service()).await.unwrap();
}
```

### shell 関数

HTML ドキュメントの外殻を定義する。`<HydrationScripts>` が WASM の読み込みスクリプトを自動挿入する:

```rust
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ja" class="dark">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body class="bg-black text-white min-h-screen">
                <App />
            </body>
        </html>
    }
}
```

---

## 9. エラーハンドリング

### アプリケーションエラー型

`thiserror` で `AppError` を定義し、HTTP ステータスコードへのマッピングを持たせる:

```rust
#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

### サーバー関数でのエラーパターン

```rust
#[server]
pub async fn signup(username: String, password: String) -> Result<(), ServerFnError> {
    // バリデーションエラー
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }

    // DB エラーを map_err で変換
    sqlx::query("INSERT INTO users ...")
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(())
}
```

`.unwrap()` は使わず、すべて `?` 演算子と `map_err` でエラーを伝播する。

---

## 10. スタイリング — Tailwind CSS v4

`cargo-leptos` が Tailwind CSS のビルドパイプラインを統合しており、Rust ファイル内のクラス名を自動スキャンする:

```toml
# Cargo.toml の [package.metadata.leptos]
style-file = "style/tailwind.css"
tailwind-input-file = "style/tailwind.css"
tailwind-config-file = "tailwind.config.js"
```

`view!` マクロ内で直接 Tailwind のクラスを書く:

```rust
// 静的なクラス
view! { <article class="p-4 border-b border-gray-800 hover:bg-gray-900/50"> ... </article> }

// Signal に応じた動的クラス
view! {
    <button class=move || {
        if is_liked.get() { "text-pink-600" } else { "hover:text-pink-600" }
    }>
        ...
    </button>
}
```

カスタムカラーを `tailwind.config.js` で定義し、`bg-chirp-blue` のように使える:

```js
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        chirp: {
          blue: "#1d9bf0",
          hover: "#1a8cd8",
          dark: "#15202b",
          border: "#38444d",
        },
      },
    },
  },
};
```

---

次回の[中編](blog_02.md)では、DB 設計パターン（カウンターキャッシュ、カーソルページネーション、トライグラム検索）、認証、条件付きコンパイルの実践を解説する。

## 参考資料

- [Leptos 公式サイト](https://leptos.dev/)
- [leptos-rs/leptos - GitHub](https://github.com/leptos-rs/leptos)
- [Leptos 0.8 リリースノート](https://github.com/leptos-rs/leptos/releases)
- [Axum - GitHub](https://github.com/tokio-rs/axum)
- [Tailwind CSS v4](https://tailwindcss.com/)
