use axum::{
    Router,
    extract::{Form, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rusqlite::Connection;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    download_dir: PathBuf,
}

#[derive(Debug)]
struct Book {
    id: i64,
    title: String,
    author: String,
    status: String,
}

#[derive(Debug)]
struct Patron {
    username: String,
    role: String,
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
}

#[derive(Deserialize)]
struct BookParams {
    id: Option<String>,
}

#[derive(Deserialize)]
struct DownloadParams {
    file: Option<String>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct AccountForm {
    display_name: String,
}

#[derive(Deserialize)]
struct LoanForm {
    book_id: String,
    borrower: String,
}

const STYLE: &str = r#"
:root {
  color-scheme: light;
  --ink: #172033;
  --muted: #667085;
  --paper: #ffffff;
  --canvas: #f5f7fa;
  --line: #d9e2ec;
  --line-strong: #b9c7d6;
  --teal: #0f766e;
  --teal-dark: #115e59;
  --green: #166534;
  --amber: #b45309;
  --rose: #be123c;
  --shadow: 0 18px 44px rgba(15, 35, 54, 0.09);
  background: var(--canvas);
  color: var(--ink);
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  background:
    linear-gradient(180deg, rgba(238, 244, 249, 0.92) 0%, rgba(245, 247, 250, 0) 360px),
    var(--canvas);
  margin: 0;
  min-height: 100vh;
}

body::before {
  background: linear-gradient(90deg, var(--teal), #2563eb 50%, var(--rose));
  content: "";
  display: block;
  height: 4px;
}

header {
  align-items: center;
  backdrop-filter: blur(16px);
  background: rgba(255, 255, 255, 0.86);
  border-bottom: 1px solid rgba(217, 226, 236, 0.9);
  display: flex;
  gap: 24px;
  justify-content: space-between;
  padding: 16px max(24px, calc((100vw - 1120px) / 2 + 24px));
  position: sticky;
  top: 0;
  z-index: 10;
}

a {
  color: var(--teal-dark);
  font-weight: 650;
  text-decoration-color: rgba(15, 118, 110, 0.28);
  text-underline-offset: 3px;
}

a:hover {
  color: #0f4d47;
  text-decoration-color: currentColor;
}

.brand {
  align-items: center;
  color: var(--ink);
  display: inline-flex;
  font-size: 17px;
  font-weight: 800;
  gap: 10px;
  letter-spacing: 0;
  text-decoration: none;
}

.brand-mark {
  align-items: center;
  background: var(--ink);
  border-radius: 8px;
  color: #ffffff;
  display: inline-flex;
  font-size: 13px;
  height: 32px;
  justify-content: center;
  width: 32px;
}

nav {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

nav a {
  border: 1px solid transparent;
  border-radius: 999px;
  color: #475467;
  font-size: 14px;
  padding: 8px 12px;
  text-decoration: none;
}

nav a:hover {
  background: #edf7f6;
  border-color: rgba(15, 118, 110, 0.18);
  color: var(--teal-dark);
}

main {
  margin: 0 auto;
  max-width: 1120px;
  padding: 42px 24px 64px;
}

section {
  margin: 0 0 28px;
}

h1 {
  font-size: 40px;
  letter-spacing: 0;
  line-height: 1.05;
  margin: 0;
}

h2 {
  font-size: 20px;
  letter-spacing: 0;
  line-height: 1.2;
  margin: 0;
}

p {
  color: var(--muted);
  line-height: 1.65;
  margin: 10px 0 0;
}

.eyebrow {
  color: var(--teal-dark);
  font-size: 12px;
  font-weight: 800;
  letter-spacing: 0;
  margin: 0 0 12px;
  text-transform: uppercase;
}

.intro {
  align-items: stretch;
  display: grid;
  gap: 24px;
  grid-template-columns: minmax(0, 1fr) minmax(300px, 380px);
  justify-content: space-between;
  margin-bottom: 28px;
}

.hero-copy {
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: var(--shadow);
  padding: 42px;
}

.hero-copy h1 {
  font-size: 56px;
  line-height: 0.98;
}

.hero-copy p:last-child {
  max-width: 680px;
}

.hero-panel,
.panel {
  background: rgba(255, 255, 255, 0.92);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: var(--shadow);
}

.hero-panel {
  align-self: stretch;
  display: grid;
  gap: 18px;
  padding: 22px;
}

.metric-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: 1fr 1fr;
}

.metric {
  border-left: 3px solid #c7d2fe;
  padding: 8px 0 8px 14px;
}

.metric strong {
  display: block;
  font-size: 28px;
  line-height: 1;
}

.metric span {
  color: var(--muted);
  display: block;
  font-size: 13px;
  margin-top: 8px;
}

.panel {
  padding: 24px;
}

.panel-header {
  align-items: start;
  display: flex;
  gap: 16px;
  justify-content: space-between;
  margin-bottom: 18px;
}

.search-form,
.stack {
  display: flex;
  gap: 12px;
}

.search-form {
  align-items: center;
}

.hero-panel .search-form {
  align-items: stretch;
}

.search-form input {
  flex: 1;
  min-width: 0;
}

.stack {
  align-items: start;
  flex-direction: column;
  width: min(100%, 440px);
}

label {
  color: #344054;
  display: grid;
  font-size: 14px;
  font-weight: 750;
  gap: 6px;
  width: 100%;
}

input,
select {
  background: #ffffff;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  color: var(--ink);
  font: inherit;
  min-height: 44px;
  outline: none;
  padding: 10px 12px;
  transition:
    border-color 160ms ease,
    box-shadow 160ms ease;
}

input:focus,
select:focus {
  border-color: var(--teal);
  box-shadow: 0 0 0 4px rgba(15, 118, 110, 0.13);
}

button {
  background: var(--teal-dark);
  border: 0;
  border-radius: 8px;
  box-shadow: 0 10px 22px rgba(17, 94, 89, 0.18);
  color: #ffffff;
  cursor: pointer;
  font: inherit;
  font-weight: 800;
  min-height: 44px;
  padding: 10px 16px;
  transition:
    background 160ms ease,
    transform 160ms ease;
}

button:hover {
  background: #0f4d47;
  transform: translateY(-1px);
}

.grid {
  display: grid;
  gap: 18px;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
}

article {
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: 0 12px 30px rgba(15, 35, 54, 0.06);
  padding: 22px;
}

.book-card {
  display: grid;
  gap: 16px;
  min-height: 210px;
}

.book-card h2 {
  min-height: 48px;
}

.book-meta {
  color: var(--muted);
  font-size: 14px;
  margin: 0;
}

.card-actions {
  align-items: center;
  border-top: 1px solid #edf2f7;
  display: flex;
  justify-content: space-between;
  margin-top: auto;
  padding-top: 14px;
}

.status-pill {
  align-items: center;
  background: #f8fafc;
  border: 1px solid #d9e2ec;
  border-radius: 999px;
  color: #475467;
  display: inline-flex;
  font-size: 12px;
  font-weight: 800;
  gap: 6px;
  line-height: 1;
  margin: 0;
  padding: 7px 10px;
  width: fit-content;
}

.status-pill::before {
  background: currentColor;
  border-radius: 999px;
  content: "";
  height: 7px;
  width: 7px;
}

.status-available {
  background: #eef8f3;
  border-color: #b7e4ca;
  color: var(--green);
}

.status-hold {
  background: #fff7ed;
  border-color: #fed7aa;
  color: var(--amber);
}

.record-list {
  margin-bottom: 18px;
}

.record-card {
  display: grid;
  gap: 10px;
}

.result-banner,
.session-banner,
.saved-banner {
  background: #f8fafc;
  border: 1px solid #e6edf5;
  border-radius: 8px;
  color: #344054;
  margin: 18px 0;
  padding: 14px 16px;
}

.session-banner code,
.saved-banner strong {
  color: var(--ink);
  font-weight: 800;
  overflow-wrap: anywhere;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 18px;
}

.link-button {
  align-items: center;
  background: #ffffff;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  color: var(--teal-dark);
  display: inline-flex;
  font-weight: 800;
  min-height: 44px;
  padding: 10px 14px;
  text-decoration: none;
}

.link-button:hover {
  background: #edf7f6;
}

pre {
  background: #111827;
  border-radius: 8px;
  color: #e5e7eb;
  font-size: 13px;
  line-height: 1.55;
  margin: 18px 0 0;
  overflow: auto;
  padding: 16px;
}

.error {
  background: #fff1f2;
  border: 1px solid #fecdd3;
  border-radius: 8px;
  color: var(--rose);
  font-weight: 800;
  padding: 12px 14px;
}

pre.download-path {
  border: 1px solid #fcd34d;
}

.spacious {
  margin-top: 28px;
}

@media (max-width: 720px) {
  header,
  .intro,
  .panel-header,
  .search-form {
    align-items: stretch;
    display: flex;
    flex-direction: column;
  }

  header {
    position: static;
  }

  nav {
    width: 100%;
  }

  nav a {
    background: #f8fafc;
  }

  main {
    padding: 28px 16px 48px;
  }

  .hero-copy,
  .hero-panel,
  .panel {
    padding: 20px;
  }

  .hero-copy h1,
  h1 {
    font-size: 36px;
  }
}
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        db: Arc::new(Mutex::new(init_db()?)),
        download_dir: PathBuf::from("files"),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/account", get(account).post(update_account))
        .route("/book", get(book))
        .route("/download", get(download))
        .route("/health", get(health))
        .route("/loans", post(create_loan))
        .route("/login", get(login_page).post(login))
        .route("/search", get(search))
        .route("/static/style.css", get(style))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        r#"
        CREATE TABLE patrons (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            role TEXT NOT NULL
        );

        CREATE TABLE books (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            author TEXT NOT NULL,
            status TEXT NOT NULL
        );

        INSERT INTO patrons (username, password, role) VALUES
            ('librarian', 'library123', 'staff'),
            ('alice', 'password', 'member');

        INSERT INTO books (title, author, status) VALUES
            ('Threat Modeling for Small Teams', 'A. Shiraishi', 'available'),
            ('Secure Rust Services', 'M. Tanaka', 'available'),
            ('OAuth Field Notes', 'R. Sato', 'checked out'),
            ('Practical Access Control', 'K. Mori', 'available');
        "#,
    )?;
    Ok(conn)
}

async fn health() -> &'static str {
    "ok"
}

async fn style() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLE)
}

async fn index(State(state): State<AppState>) -> Response {
    let rows = match list_books(&state) {
        Ok(rows) => rows,
        Err(error) => return server_error("Catalog Error", &error.to_string(), "").into_response(),
    };

    let books = rows
        .into_iter()
        .map(|book| {
            format!(
                r#"<article class="book-card">
                    <h2>{}</h2>
                    <p class="book-meta">by {}</p>
                    <div class="card-actions">
                      <p class="{}">{}</p>
                      <a href="/book?id={}">View record</a>
                    </div>
                  </article>"#,
                book.title,
                book.author,
                status_class(&book.status),
                book.status,
                book.id
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let content = format!(
        r#"
        <!-- TODO: remove demo credentials before launch: librarian/library123 -->
        <section class="intro">
          <div class="hero-copy">
            <p class="eyebrow">Community lending desk</p>
            <h1>Library Loans</h1>
            <p>Browse the security bookshelf, request a loan, and inspect live records from the demo catalog.</p>
          </div>
          <aside class="hero-panel" aria-label="Catalog actions">
            <form action="/search" method="get" class="search-form">
              <input name="q" value="rust" aria-label="Search query">
              <button type="submit">Search</button>
            </form>
            <div class="metric-grid">
              <div class="metric">
                <strong>4</strong>
                <span>catalog records</span>
              </div>
              <div class="metric">
                <strong>24h</strong>
                <span>desk pickup</span>
              </div>
            </div>
          </aside>
        </section>
        <section class="panel">
          <div class="panel-header">
            <div>
              <p class="eyebrow">Featured shelf</p>
              <h2>Available Titles</h2>
            </div>
            <a href="/download?file=borrowing-policy.txt">Download policy</a>
          </div>
          <div class="grid">{books}</div>
        </section>
        <section class="panel spacious">
          <h2>Request a Loan</h2>
          <form action="/loans" method="post" class="stack">
            <label>
              Book ID
              <input name="book_id" value="1">
            </label>
            <label>
              Borrower
              <input name="borrower" value="alice">
            </label>
            <button type="submit">Request loan</button>
          </form>
        </section>
        "#
    );

    layout("Library Loans", content).into_response()
}

async fn search(Query(params): Query<SearchParams>) -> Response {
    let query = params.q.unwrap_or_default();
    let result = if query.is_empty() {
        "Enter a search term.".to_string()
    } else {
        format!("Results for: {query}")
    };

    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Catalog finder</p>
          <h1>Search Catalog</h1>
          <form action="/search" method="get" class="search-form">
            <input name="q" value="{query}" aria-label="Search query">
            <button type="submit">Search</button>
          </form>
          <p class="result-banner">{result}</p>
        </section>
        "#
    );

    layout("Search Catalog", content).into_response()
}

async fn book(State(state): State<AppState>, Query(params): Query<BookParams>) -> Response {
    let id = params.id.unwrap_or_else(|| "1".to_string());
    let sql = format!("SELECT id, title, author, status FROM books WHERE id = {id}");

    let rows = match query_books(&state, &sql) {
        Ok(rows) => rows,
        Err(error) => {
            return server_error("Database Error", &error.to_string(), &sql).into_response();
        }
    };

    let records = if rows.is_empty() {
        "<p>No book found.</p>".to_string()
    } else {
        rows.into_iter()
            .map(|book| {
                format!(
                    r#"<article class="record-card">
                        <h2>{}</h2>
                        <p class="book-meta">Author: {}</p>
                        <p class="{}">{}</p>
                      </article>"#,
                    book.title,
                    book.author,
                    status_class(&book.status),
                    book.status
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Catalog record</p>
          <h1>Book Record</h1>
          <div class="grid record-list">{records}</div>
          <pre>{sql}</pre>
        </section>
        "#
    );

    layout("Book Record", content).into_response()
}

async fn login_page() -> Response {
    login_form(None).into_response()
}

async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let sql = format!(
        "SELECT username, role FROM patrons WHERE username = '{}' AND password = '{}'",
        form.username, form.password
    );

    let patron = match query_patron(&state, &sql) {
        Ok(patron) => patron,
        Err(error) => {
            return server_error("Database Error", &error.to_string(), &sql).into_response();
        }
    };

    if let Some(patron) = patron {
        let mut response = Redirect::to("/account").into_response();
        let cookie = format!(
            "library_session={}-{}-session; Path=/",
            patron.username, patron.role
        );
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
        return response;
    }

    login_form(Some("Invalid username or password.")).into_response()
}

async fn account(headers: HeaderMap) -> Response {
    let session =
        current_session(&headers).unwrap_or_else(|| "No session cookie is set.".to_string());
    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Member desk</p>
          <h1>Patron Account</h1>
          <p class="session-banner">Session: <code>{session}</code></p>
          <form action="/account" method="post" class="stack">
            <label>
              Display name
              <input name="display_name" value="Alice Reader">
            </label>
            <button type="submit">Save profile</button>
          </form>
        </section>
        "#
    );

    layout("Patron Account", content).into_response()
}

async fn update_account(headers: HeaderMap, Form(form): Form<AccountForm>) -> Response {
    let session =
        current_session(&headers).unwrap_or_else(|| "No session cookie is set.".to_string());
    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Member desk</p>
          <h1>Patron Account</h1>
          <p class="session-banner">Session: <code>{session}</code></p>
          <p class="saved-banner">Saved display name: <strong>{}</strong></p>
          <form action="/account" method="post" class="stack">
            <label>
              Display name
              <input name="display_name" value="{}">
            </label>
            <button type="submit">Save profile</button>
          </form>
        </section>
        "#,
        form.display_name, form.display_name
    );

    layout("Patron Account", content).into_response()
}

async fn create_loan(Form(form): Form<LoanForm>) -> Response {
    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Circulation desk</p>
          <h1>Loan Requested</h1>
          <p class="result-banner">Book ID: {}</p>
          <p class="result-banner">Borrower: {}</p>
          <div class="actions">
            <a class="link-button" href="/">Back to catalog</a>
          </div>
        </section>
        "#,
        form.book_id, form.borrower
    );

    layout("Loan Requested", content).into_response()
}

async fn download(State(state): State<AppState>, Query(params): Query<DownloadParams>) -> Response {
    let file_name = params
        .file
        .unwrap_or_else(|| "borrowing-policy.txt".to_string());
    let target = state.download_dir.join(&file_name);

    match tokio::fs::read(&target).await {
        Ok(bytes) => {
            let download_name = Path::new(&file_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("download.txt");
            let mut response = bytes.into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            if let Ok(value) =
                HeaderValue::from_str(&format!("attachment; filename=\"{download_name}\""))
            {
                response
                    .headers_mut()
                    .insert(header::CONTENT_DISPOSITION, value);
            }
            response
        }
        Err(error) => {
            let content = format!(
                r#"
                <section class="panel">
                  <p class="eyebrow">File desk</p>
                  <h1>Download Error</h1>
                  <p class="error">{}</p>
                  <pre class="download-path">{}</pre>
                </section>
                "#,
                error,
                target.display()
            );
            (StatusCode::NOT_FOUND, layout("Download Error", content)).into_response()
        }
    }
}

fn list_books(state: &AppState) -> rusqlite::Result<Vec<Book>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement = conn.prepare("SELECT id, title, author, status FROM books ORDER BY id")?;
    let rows = statement.query_map([], map_book)?;
    rows.collect()
}

fn query_books(state: &AppState, sql: &str) -> rusqlite::Result<Vec<Book>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], map_book)?;
    rows.collect()
}

fn query_patron(state: &AppState, sql: &str) -> rusqlite::Result<Option<Patron>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement = conn.prepare(sql)?;
    let mut rows = statement.query_map([], |row| {
        Ok(Patron {
            username: row.get(0)?,
            role: row.get(1)?,
        })
    })?;

    rows.next().transpose()
}

fn map_book(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
    Ok(Book {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        status: row.get(3)?,
    })
}

fn status_class(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("available") {
        "status-pill status-available"
    } else {
        "status-pill status-hold"
    }
}

fn login_form(message: Option<&str>) -> Html<String> {
    let message_html = message
        .map(|value| format!(r#"<p class="error">{value}</p>"#))
        .unwrap_or_default();
    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Staff area</p>
          <h1>Staff Login</h1>
          {message_html}
          <form action="/login" method="post" class="stack">
            <label>
              Username
              <input name="username" autocomplete="username">
            </label>
            <label>
              Password
              <input name="password" type="password" autocomplete="current-password">
            </label>
            <button type="submit">Login</button>
          </form>
        </section>
        "#
    );

    layout("Staff Login", content)
}

fn server_error(title: &str, error: &str, sql: &str) -> (StatusCode, Html<String>) {
    let content = format!(
        r#"
        <section class="panel">
          <p class="eyebrow">Application error</p>
          <h1>{title}</h1>
          <p class="error">{error}</p>
          <pre>{sql}</pre>
        </section>
        "#
    );

    (StatusCode::INTERNAL_SERVER_ERROR, layout(title, content))
}

fn current_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let trimmed = part.trim();
        trimmed
            .strip_prefix("library_session=")
            .map(ToOwned::to_owned)
    })
}

fn layout(title: &str, content: String) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    <header>
      <a class="brand" href="/"><span class="brand-mark">RL</span><span>Rust Library Loans</span></a>
      <nav>
        <a href="/search?q=rust">Search</a>
        <a href="/book?id=1">Book</a>
        <a href="/download?file=borrowing-policy.txt">Policy</a>
        <a href="/login">Login</a>
        <a href="/account">Account</a>
      </nav>
    </header>
    <main>{content}</main>
  </body>
</html>"#
    ))
}
